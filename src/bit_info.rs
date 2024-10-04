use std::ops::BitOrAssign;

use crate::{
    constants::{GEN_POLYNOMIALS, NUM_BLOCKS, NUM_DATA_MODULES, NUM_EC_CODEWORDS},
    data::Data,
    encoding::num_cci_bits,
    error_correction::remainder,
    matrix::{Matrix, Module},
    qr_code::{mask_fn, Mask, Mode, QrCode, Version, ECL},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Info {
    /// Module except meaning changes when DATA set
    pub module: Module,
    /// 0-indexed error correction block
    pub block_i: usize,
    /// 0-indexed bit index
    pub bit_i: usize,
}

impl Info {
    pub const DATA: Module = Module(Module::DATA.0 | Module::ON.0);
    pub const EC: Module = Module(Module::DATA.0 | Module::MODIFIER.0);
    pub const REMAINDER: Module = Module(Module::DATA.0);
}

impl From<Info> for Module {
    fn from(value: Info) -> Self {
        value.module
    }
}
impl From<Module> for Info {
    fn from(module: Module) -> Self {
        Info {
            module,
            block_i: 0,
            bit_i: 0,
        }
    }
}

impl BitOrAssign<Module> for Info {
    fn bitor_assign(&mut self, rhs: Module) {
        self.module |= rhs;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WeightBit(pub u8);

impl WeightBit {
    pub fn new(value: bool, hardness: u8) -> Self {
        WeightBit(value as u8 | (hardness << 1))
    }
    /// 1 or 0
    pub fn value(&self) -> u8 {
        self.0 & 1
    }
    /// 0 - 127 representing how important current value is
    pub fn hardness(&self) -> u8 {
        self.0 >> 1
    }
}

#[derive(Debug)]
pub struct BitInfo {
    pub matrix: Matrix<Info>,
    pub mode: Mode,
    pub version: Version,
    pub ecl: ECL,
    pub mask: Mask,
}

impl BitInfo {
    pub fn new(mode: Mode, version: Version, ecl: ECL, mask: Mask) -> Self {
        let mut bit_info = BitInfo {
            matrix: Matrix::new(
                version,
                Info {
                    module: Module(0),
                    block_i: 0,
                    bit_i: 0,
                },
            ),
            mode,
            version,
            ecl,
            mask,
        };

        bit_info.matrix.set_finder();
        bit_info.matrix.set_alignment();
        bit_info.matrix.set_timing();
        bit_info.matrix.set_format(bit_info.ecl, mask);
        bit_info.matrix.set_version();

        let modules = NUM_DATA_MODULES[bit_info.version.0] as usize;
        let codewords = modules / 8;

        let num_ec_codewords = NUM_EC_CODEWORDS[bit_info.version.0][bit_info.ecl as usize] as usize;
        let num_data_codewords = codewords - num_ec_codewords;

        let blocks = NUM_BLOCKS[bit_info.version.0][bit_info.ecl as usize] as usize;

        let group_2_blocks = codewords % blocks;
        let group_1_blocks = blocks - group_2_blocks;

        let data_per_g1_block = num_data_codewords / blocks;
        let ecc_per_block = num_ec_codewords / blocks;

        let data_end = num_data_codewords * 8;
        let ecc_end = codewords * 8;

        let mut i = 0;
        let mut block_i = 0;
        let mut bit_i = 0;

        bit_info.matrix.set_data(|| {
            let val = Info {
                module: match i {
                    j if j < data_end => Info::DATA,
                    j if j < ecc_end * 8 => Info::EC,
                    _ => Info::REMAINDER,
                },
                block_i,
                bit_i,
            };
            bit_i += 1;
            i += 1;

            if i % 8 != 0 {
                return val;
            }

            if i < data_end {
                let byte_i = i / 8;
                let col = byte_i / blocks;
                let row = if col < data_per_g1_block {
                    byte_i % blocks
                } else {
                    (byte_i + group_1_blocks) % blocks
                };

                block_i = row;
                bit_i = col * 8;
                // bit_i = if row < group_1_blocks {
                //     ((row * data_per_g1_block) + col) * 8
                // } else {
                //     ((row * data_per_g1_block) + (row - group_1_blocks) + col) * 8
                // };
            } else if i < ecc_end {
                let ecc_i = (i / 8) - num_data_codewords;
                let col = ecc_i / blocks;
                let row = ecc_i % blocks;

                block_i = row;

                // TODO KYLE HERE
                // I BELIEVE THIS IS WRONG
                // BIT_I is index into BLOCK
                bit_i = if row < group_1_blocks {
                    (data_per_g1_block + col) * 8
                } else {
                    (data_per_g1_block + 1 + col) * 8
                }
                // bit_i = col * 8;
                // bit_i = (row * ecc_per_block + col) * 8;
            } else {
                block_i = 0;
                bit_i = 0;
            }

            val
        });

        bit_info
    }
}

#[derive(Debug)]
pub struct QartCode<'a> {
    pub bit_info: BitInfo,
    pub initials: Vec<Vec<u8>>,
    pub blocks: Vec<Vec<WeightBit>>,
    pub data: &'a mut Data,
}

impl<'a> QartCode<'a> {
    pub fn new(data: &'a mut Data, mask: Mask) -> Self {
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
        let remainder_data_bits = (num_data_codewords * 8) - (data.bit_len);
        let term_len = if remainder_data_bits < 4 {
            remainder_data_bits
        } else {
            4
        };

        data.push_bits(0, term_len);
        let orig_data_bit_len = data.bit_len;
        if data.value.len() < num_data_codewords {
            data.value.resize(num_data_codewords, 0);
        }

        let mut initials = vec![];
        let mut blocks = vec![];
        let mut data_i = 0;

        for i in 0..(group_1_blocks + group_2_blocks) as usize {
            let data_per_block = if i < group_1_blocks as usize {
                data_per_g1_block
            } else {
                data_per_g1_block + 1
            };

            let byte_start = data_i / 8;
            let mut data_codewords = data.value[byte_start..(byte_start + data_per_block)].to_vec();
            let mut ecc = remainder(
                &data_codewords,
                &GEN_POLYNOMIALS[ecc_per_block][..ecc_per_block],
            );
            data_codewords.append(&mut ecc);
            initials.push(data_codewords);

            blocks.push(Vec::with_capacity(data_per_block + ecc_per_block));
            for j in 0..(data_per_block + ecc_per_block) * 8 {
                if data_i < orig_data_bit_len && j < (data_per_block * 8) {
                    let byte = data.value[data_i / 8];
                    let on = byte & (1 << (7 - (data_i % 8))) != 0;
                    blocks[i].push(WeightBit::new(on, 127));
                    data_i += 1;
                } else {
                    blocks[i].push(WeightBit::new(false, 0));
                }
            }
        }

        // initials.iter().for_each(|init|{
        //     init.iter().for_each(|byte| {
        //         print!("{:0>8b}", byte);
        //     });
        //     println!("");
        // });

        QartCode {
            bit_info: BitInfo::new(data.mode, data.version, data.ecl, mask),
            initials,
            blocks,
            data,
        }
    }

    pub fn to_qr_code(mut self, image: Vec<bool>) -> QrCode {
        self.data.push_bits(0, 0);

        let width = self.data.version.0 * 4 + 17;

        let modules = NUM_DATA_MODULES[self.data.version.0] as usize;
        let codewords = modules / 8;

        let num_ec_codewords =
            NUM_EC_CODEWORDS[self.data.version.0][self.data.ecl as usize] as usize;
        let num_data_codewords = codewords - num_ec_codewords;

        let blocks = NUM_BLOCKS[self.data.version.0][self.data.ecl as usize] as usize;

        let group_2_blocks = codewords % blocks;
        let group_1_blocks = blocks - group_2_blocks;

        let data_per_g1_block = num_data_codewords / blocks;
        let ecc_per_block = num_ec_codewords / blocks;

        assert_eq!(image.len(), width * width);

        let mask = mask_fn(self.bit_info.mask);

        for y in 0..width {
            for x in 0..width {
                let bit = self.bit_info.matrix.get(x, y);
                if !bit.module.has(Module::DATA) || bit.module == Info::REMAINDER {
                    continue;
                }

                let pbit = self.blocks[bit.block_i][bit.bit_i];
                if pbit.hardness() < 127 {
                    let value = mask(x as u16, y as u16) ^ image[y * width + x];
                    self.blocks[bit.block_i][bit.bit_i] = WeightBit::new(value, 64)
                }
            }
        }

        let mut g1_basis = vec![];
        for i in 0..data_per_g1_block * 8 {
            let mut v = vec![0; data_per_g1_block];
            v[i / 8] = 1 << (7 - (i % 8));
            v.append(&mut remainder(
                &v,
                &GEN_POLYNOMIALS[ecc_per_block][..ecc_per_block],
            ));
            g1_basis.push(v);
        }

        let mut g2_basis = vec![];
        for i in 0..(data_per_g1_block + 1) * 8 {
            let mut v = vec![0; data_per_g1_block + 1];
            v[i / 8] = 1 << (7 - (i % 8));
            v.append(&mut remainder(
                &v,
                &GEN_POLYNOMIALS[ecc_per_block][..ecc_per_block],
            ));
            g2_basis.push(v);
        }

        let mut final_blocks = Vec::with_capacity(self.blocks.len());
        for i in 0..group_1_blocks {
            final_blocks.push(find_closest_match(
                &g1_basis,
                &self.initials[i],
                &self.blocks[i],
                1,
            ));
        }
        for i in 0..group_2_blocks {
            final_blocks.push(find_closest_match(
                &g2_basis,
                &self.initials[i],
                &self.blocks[i],
                1,
            ));
        }

        let mut matrix = Matrix::new(self.data.version, Module(0));

        let width = self.data.version.0 * 4 + 17;
        for y in 0..width {
            for x in 0..width {
                let info = self.bit_info.matrix.get(x, y);

                if !info.module.has(Module::DATA) {
                    matrix.set(x, y, info.module);
                } else if info.module == Info::REMAINDER {
                    let on = mask(x as u16, y as u16) as u8 ^ image[y * width + x] as u8;
                    matrix.set(x, y, Module::DATA | (Module(on)));
                } else {
                    let on = mask(x as u16, y as u16) as u8
                        ^ (final_blocks[info.block_i][info.bit_i / 8] >> (7 - (info.bit_i % 8)))
                            & 1;
                    matrix.set(x, y, Module::DATA | Module(on));
                }
            }
        }

        QrCode {
            matrix,
            mode: self.data.mode,
            version: self.data.version,
            ecl: self.data.ecl,
            mask: self.bit_info.mask,
        }
        // if dividing data by generator,
        // the data that produces a remainder = desired remainder + generator
        //
        // basis matrix = data bits * data bits , width height
    }
}

fn find_closest_match(
    basis_matrix: &Vec<Vec<u8>>,
    initial: &Vec<u8>,
    desired: &Vec<WeightBit>,
    beam_width: usize,
) -> Vec<u8> {
    // return initial.clone();
    // let new_vector = initial
    //     .iter()
    //     .enumerate()
    //     .map(|(j, byte)| byte ^ basis_matrix[0][j])
    //     .collect::<Vec<u8>>();
    // println!("initial");
    // initial.iter().for_each(|byte| {
    //     print!("{:0>8b}", byte);
    // });
    // println!("basis 0");
    // basis_matrix[0].iter().for_each(|byte| {
    //     print!("{:0>8b}", byte);
    // });
    // println!("out");
    // new_vector.iter().for_each(|byte| {
    //     print!("{:0>8b}", byte);
    // });
    // println!("");
    // return new_vector;
    let mut beam = vec![(initial.clone(), weighted_hamming(&initial, &desired))];
    println!("start {}", beam[0].1);
    // opportunity to try adding every basis
    for _ in 0..basis_matrix.len() {
        let mut new_beam = vec![];
        for (vector, _weight) in &beam {
            for (i, basis) in basis_matrix.iter().enumerate() {
                if (vector[i / 8] >> (7 - (i % 8))) & 1 == 1 {
                    // basis vector has only 1 bit set in data codewords
                    continue;
                }
                let new_vector = vector
                    .iter()
                    .enumerate()
                    .map(|(j, byte)| byte ^ basis[j])
                    .collect::<Vec<u8>>();
                let new_weight = weighted_hamming(&new_vector, desired);
                new_beam.push((new_vector, new_weight));
            }
        }

        beam.append(&mut new_beam);
        beam.sort_by(|a, b| a.1.cmp(&b.1));
        beam.truncate(beam_width);

        println!("iter {}", beam[0].1);
    }

    beam[0].0.clone()
}

fn weighted_hamming(v: &Vec<u8>, w: &Vec<WeightBit>) -> u16 {
    let mut dist = 0;
    // let mut matches = 0;
    for i in 0..w.len() {
        let a = (v[i / 8] >> (7 - (i % 8))) & 1;
        let b = w[i].value();
        if a != b {
            dist += w[i].hardness() as u16;
        }
    }
    // println!("matches {} dist {}", matches,  dist);
    dist
}
