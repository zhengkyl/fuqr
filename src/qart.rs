use crate::{
    bit_info::{BitInfo, Info},
    constants::{GEN_POLYNOMIALS, NUM_BLOCKS, NUM_DATA_MODULES, NUM_EC_CODEWORDS},
    data::{BitVec, Data},
    error_correction::remainder,
    matrix::{Matrix, Module},
    qr_code::{mask_fn, Mask, QrCode},
};

#[derive(Debug, Clone, Copy)]
pub struct WeightPixel(pub u8);

impl WeightPixel {
    pub fn new(value: bool, hardness: u8) -> Self {
        WeightPixel(value as u8 | (hardness << 1))
    }
    /// 1 or 0
    pub fn value(&self) -> bool {
        self.0 & 1 == 1
    }
    /// 0 - 127 representing how important current value is
    pub fn weight(&self) -> u8 {
        self.0 >> 1
    }
}

#[derive(Debug)]
pub struct QArtCode {
    pub bit_info: BitInfo,
    pub blocks: Vec<BitVec>,
    pub block_weights: Vec<Vec<WeightPixel>>,
}

impl QArtCode {
    pub fn new(mut data: Data, mask: Mask) -> Self {
        let modules = NUM_DATA_MODULES[data.version.0] as usize;
        let codewords = modules / 8;

        let num_ec_codewords = NUM_EC_CODEWORDS[data.version.0][data.ecl as usize] as usize;
        let num_data_codewords = codewords - num_ec_codewords;

        let blocks = NUM_BLOCKS[data.version.0][data.ecl as usize] as usize;

        let group_2_blocks = codewords % blocks;
        let group_1_blocks = blocks - group_2_blocks;

        let data_per_g1_block = num_data_codewords / blocks;
        let ecc_per_block = num_ec_codewords / blocks;

        // terminator seems required
        let remainder_data_bits = (num_data_codewords * 8) - (data.bits.len());
        let term_len = if remainder_data_bits < 4 {
            remainder_data_bits
        } else {
            4
        };

        data.bits.push_n(0, term_len);
        let orig_data_bit_len = data.bits.len();
        if data.bits.len() < num_data_codewords * 8 {
            // TODO filling with 0 creates checkerboard
            // perhaps fill with random noise
            data.bits.resize(num_data_codewords * 8, 0b11101100);
        }

        let mut blocks = vec![];
        let mut block_weights = vec![];
        let mut data_i = 0;

        for i in 0..(group_1_blocks + group_2_blocks) as usize {
            let data_per_block = if i < group_1_blocks as usize {
                data_per_g1_block
            } else {
                data_per_g1_block + 1
            };

            let byte_start = data_i / 8;
            let mut data_codewords =
                data.bits.as_ref()[byte_start..(byte_start + data_per_block)].to_vec();
            let mut ecc = remainder(
                &data_codewords,
                &GEN_POLYNOMIALS[ecc_per_block][..ecc_per_block],
            );
            data_codewords.append(&mut ecc);
            blocks.push(data_codewords.into());

            block_weights.push(Vec::with_capacity(data_per_block + ecc_per_block));
            for j in 0..(data_per_block + ecc_per_block) * 8 {
                if data_i < orig_data_bit_len && j < (data_per_block * 8) {
                    block_weights[i].push(WeightPixel::new(data.bits.get(data_i), 127));
                    data_i += 1;
                } else {
                    block_weights[i].push(WeightPixel::new(false, 0));
                }
            }
        }

        QArtCode {
            bit_info: BitInfo::new(data.mode, data.version, data.ecl, mask),
            blocks,
            block_weights,
        }
    }

    pub fn to_qr_code(mut self, img_weights: &Vec<WeightPixel>) -> QrCode {
        let width = self.bit_info.version.0 * 4 + 17;
        assert_eq!(img_weights.len(), width * width);

        let modules = NUM_DATA_MODULES[self.bit_info.version.0] as usize;
        let codewords = modules / 8;

        let num_ec_codewords =
            NUM_EC_CODEWORDS[self.bit_info.version.0][self.bit_info.ecl as usize] as usize;
        let num_data_codewords = codewords - num_ec_codewords;

        let blocks = NUM_BLOCKS[self.bit_info.version.0][self.bit_info.ecl as usize] as usize;

        let group_2_blocks = codewords % blocks;
        let group_1_blocks = blocks - group_2_blocks;

        let data_per_g1_block = num_data_codewords / blocks;
        let ecc_per_block = num_ec_codewords / blocks;

        let mask = mask_fn(self.bit_info.mask);

        for y in 0..width {
            for x in 0..width {
                let bit = self.bit_info.matrix.get(x, y);
                if !bit.module.has(Module::DATA) || bit.module == Info::REMAINDER {
                    continue;
                }

                // TODO reconsider randomizing "dead" pixels
                // must be in image, and maybe needs to not be random to notice
                if self.block_weights[bit.block_i][bit.bit_i].weight() < 127 {
                    let value = mask(x as u16, y as u16) ^ img_weights[y * width + x].value();
                    self.block_weights[bit.block_i][bit.bit_i] =
                        WeightPixel::new(value, img_weights[y * width + x].weight())
                }
            }
        }

        let mut g1_basis = vec![];
        for i in 0..data_per_g1_block * 8 {
            let mut v: BitVec = vec![0; data_per_g1_block].into();
            v.set(i);
            v.append(&mut remainder(
                v.as_ref(),
                &GEN_POLYNOMIALS[ecc_per_block][..ecc_per_block],
            ));
            g1_basis.push(v);
        }

        let mut g2_basis = vec![];
        for i in 0..(data_per_g1_block + 1) * 8 {
            let mut v: BitVec = vec![0; data_per_g1_block + 1].into();
            v.set(i);
            v.append(&mut remainder(
                v.as_ref(),
                &GEN_POLYNOMIALS[ecc_per_block][..ecc_per_block],
            ));
            g2_basis.push(v);
        }

        for i in 0..group_1_blocks {
            apply_first_matches(&mut self.blocks[i], &self.block_weights[i], &g1_basis);
        }
        for i in 0..group_2_blocks {
            apply_first_matches(
                &mut self.blocks[group_1_blocks + i],
                &self.block_weights[group_1_blocks + i],
                &g2_basis,
            );
        }

        let mut matrix = Matrix::new(self.bit_info.version, Module(0));

        for y in 0..width {
            for x in 0..width {
                let info = self.bit_info.matrix.get(x, y);

                if !info.module.has(Module::DATA) {
                    matrix.set(x, y, info.module);
                } else if info.module == Info::REMAINDER {
                    let on = mask(x as u16, y as u16) ^ img_weights[y * width + x].value();
                    matrix.set(x, y, Module::DATA | (Module(on as u8)));
                } else {
                    let on = mask(x as u16, y as u16) ^ self.blocks[info.block_i].get(info.bit_i);
                    matrix.set(x, y, Module::DATA | Module(on as u8));
                }
            }
        }

        QrCode {
            matrix,
            mode: self.bit_info.mode,
            version: self.bit_info.version,
            ecl: self.bit_info.ecl,
            mask: self.bit_info.mask,
        }
    }
}

// based on https://github.com/andrewyur/qart b/c go version too confusing
//
// my understanding so far:
// first, apply matching basis to control a desired bit
//  -> remove that basis from pool and XOR against all other basis vectors with that bit set
//  -> therefore, no future basis vector can affect that bit
// repeat
//
// questions:
// why does greedy work?
//  -> implies arbitrary subset of basis vectors can mostly span error correction bit vector space
//    -> is this because num data bits >> num error correction bits?
// is it worth matching bits in order of location/contrast/importance
//  -> unordered (current) works suprisingly well
fn apply_first_matches(
    block: &mut BitVec,
    block_weight: &Vec<WeightPixel>,
    basis_matrix: &Vec<BitVec>,
) {
    let mut basis_matrix: Vec<Option<BitVec>> = basis_matrix
        .iter()
        .map(|basis| Some(basis.as_ref().to_vec().into()))
        .collect();

    for (i, pixel) in block_weight.iter().enumerate() {
        if pixel.weight() == 0 {
            continue;
        }

        let mut found: Option<BitVec> = None;
        for basis_opt in basis_matrix.iter_mut() {
            if let Some(basis) = basis_opt {
                if !basis.get(i) {
                    continue;
                }
                if let Some(found) = found.as_ref() {
                    let basis = basis.as_mut();
                    let found = found.as_ref();

                    for k in 0..basis.len() {
                        basis[k] ^= found[k]
                    }
                } else {
                    found = basis_opt.take();
                }
            }
        }

        if let Some(found) = found {
            if block.get(i) != pixel.value() {
                let block = block.as_mut();
                let found = found.as_ref();

                for j in 0..block.len() {
                    block[j] ^= found[j];
                }
            }
        }
    }
}
