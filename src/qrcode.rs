use std::ops::BitOrAssign;

use crate::{
    constants::{NUM_BLOCKS, NUM_DATA_MODULES, NUM_EC_CODEWORDS},
    data::Data,
    encoding::bits_char_count_indicator,
    error_correction::ecc_and_sequence,
    mask::score,
    matrix::{Matrix, Module},
};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Numeric,
    Alphanumeric,
    Byte,
    // no plans for Kanji, ECI, StructuredAppend, FNC1,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ECL {
    Low,      // 7
    Medium,   // 15
    Quartile, // 25
    High,     // 30
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Version(pub u8);

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Version {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new(version: u8) -> Self {
        assert!(version >= 1 && version <= 40);
        Version(version)
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mask {
    M0,
    M1,
    M2,
    M3,
    M4,
    M5,
    M6,
    M7,
}

#[derive(Debug)]
pub struct QrCode {
    pub matrix: Matrix<Module>,
    pub mode: Mode,
    pub version: Version,
    pub ecl: ECL,
    pub mask: Mask,
}

// vec while in rust only land
// when wasm, we know we're gonna copy so -> use static buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bit {
    /// Module except meaning changes when DATA set
    module: Module,
    /// 1-indexed error correction block
    block: u8,
    /// 0-indexed bit index
    index: u16,
}

impl Bit {
    const HEADER: Module = Module(Module::DATA.0);
    const MESSAGE: Module = Module(Module::DATA.0 | Module::ON.0);
    const PADDING: Module = Module(Module::DATA.0 | Module::ON.0 | Module::MODIFIER.0);
    const ERROR_CORRECTION: Module = Module(Module::DATA.0 | Module::MODIFIER.0);
}

impl From<Bit> for Module {
    fn from(value: Bit) -> Self {
        value.module
    }
}
impl From<Module> for Bit {
    fn from(module: Module) -> Self {
        Bit {
            module,
            block: 0,
            index: 0,
        }
    }
}

impl BitOrAssign<Module> for Bit {
    fn bitor_assign(&mut self, rhs: Module) {
        self.module |= rhs;
    }
}

#[derive(Debug)]
pub struct BitInfo {
    pub matrix: Matrix<Bit>,
    pub mode: Mode,
    pub version: Version,
    pub ecl: ECL,
}

impl BitInfo {
    pub fn new_image(data: Data) -> Self {
        let mut bit_info = BitInfo {
            matrix: Matrix::new(
                data.version,
                Bit {
                    module: Module(0),
                    block: 0,
                    index: 0,
                },
            ),
            mode: data.mode,
            version: data.version,
            ecl: data.ecl,
        };

        bit_info.matrix.set_finder();
        bit_info.matrix.set_alignment();
        bit_info.matrix.set_timing();
        bit_info.matrix.set_format(bit_info.ecl, Mask::M0);
        bit_info.matrix.set_version();

        let modules = NUM_DATA_MODULES[bit_info.version.0 as usize];
        let codewords = modules / 8;

        let num_ec_codewords = NUM_EC_CODEWORDS[bit_info.version.0 as usize][bit_info.ecl as usize];
        let num_data_codewords = codewords - num_ec_codewords;

        let blocks = NUM_BLOCKS[bit_info.version.0 as usize][bit_info.ecl as usize] as u16;

        let group_2_blocks = codewords % blocks;
        let group_1_blocks = blocks - group_2_blocks;

        let data_per_g1_block = num_data_codewords / blocks;
        let ecc_per_block = num_ec_codewords / blocks;

        let mut i = 0;

        let mut block = 1;
        let mut index = 0;

        let header_len = 4 + bits_char_count_indicator(bit_info.version, bit_info.mode);

        bit_info.matrix.set_data(|| {
            let val = Bit {
                module: match index {
                    j if j < header_len as u16 => Bit::HEADER,
                    j if j < data.bit_len as u16 => Bit::MESSAGE,
                    j if j < num_data_codewords * 8 => Bit::PADDING,
                    j if j < codewords * 8 => Bit::ERROR_CORRECTION,
                    _ => Module(0)
                },
                block,
                index,
            };
            index += 1;
            i += 1;

            if i % 8 == 0 {
                let byte_i = i / 8;

                if byte_i < num_data_codewords {
                    let col = byte_i / blocks;
                    let row = if col < data_per_g1_block {
                        byte_i % blocks
                    } else {
                        (byte_i + group_1_blocks) % blocks
                    };

                    block = (row + 1) as u8;
                    index = if row < group_1_blocks {
                        ((row * data_per_g1_block) + col) * 8
                    } else {
                        ((row * data_per_g1_block) + (row - group_1_blocks) + col) * 8
                    };
                } else {
                    let ecc_i = byte_i - num_data_codewords;
                    let col = ecc_i / blocks;
                    let row = ecc_i % blocks;

                    block = (row + 1) as u8; // block is 1-index
                    index = (row * ecc_per_block + col) * 8;
                }
            }

            val
        });

        bit_info
    }
}

impl QrCode {
    pub fn new(data: Data, mask: Option<Mask>) -> Self {
        let mut qr_code = QrCode {
            matrix: Matrix::new(data.version, Module(0)),
            mode: data.mode,
            version: data.version,
            ecl: data.ecl,
            mask: if let Some(mask) = mask {
                mask
            } else {
                Mask::M0
            },
        };
        qr_code.matrix.set_finder();
        qr_code.matrix.set_alignment();
        qr_code.matrix.set_timing();
        qr_code.matrix.set_format(qr_code.ecl, qr_code.mask);
        qr_code.matrix.set_version();

        let data = ecc_and_sequence(data);

        let mut i = 0;
        qr_code.matrix.set_data(|| {
            let val = Module::DATA | ((data[i / 8] >> (7 - (i % 8))) & 1).into();
            i += 1;
            val
        });
        qr_code.apply_mask(qr_code.mask);

        if let None = mask {
            let mut min_score = score(&qr_code.matrix);
            let mut min_mask = qr_code.mask;
            for m in [
                Mask::M1,
                Mask::M2,
                Mask::M3,
                Mask::M4,
                Mask::M5,
                Mask::M6,
                Mask::M7,
            ] {
                // undo prev mask
                qr_code.apply_mask(qr_code.mask);

                qr_code.mask = m;
                qr_code.apply_mask(qr_code.mask);
                qr_code.matrix.set_format(qr_code.ecl, qr_code.mask);
                let score = score(&qr_code.matrix);
                if score < min_score {
                    min_score = score;
                    min_mask = qr_code.mask;
                }
            }
            if min_mask != qr_code.mask {
                // undo prev mask
                qr_code.apply_mask(qr_code.mask);

                qr_code.mask = min_mask;
                qr_code.apply_mask(qr_code.mask);

                qr_code.matrix.set_format(qr_code.ecl, qr_code.mask);
            }
        }

        qr_code
    }

    fn apply_mask(&mut self, mask: Mask) {
        let mask_bit = match mask {
            Mask::M0 => |col: u16, row: u16| (row + col) % 2 == 0,
            Mask::M1 => |_: u16, row: u16| (row) % 2 == 0,
            Mask::M2 => |col: u16, _: u16| (col) % 3 == 0,
            Mask::M3 => |col: u16, row: u16| (row + col) % 3 == 0,
            Mask::M4 => |col: u16, row: u16| ((row / 2) + (col / 3)) % 2 == 0,
            Mask::M5 => |col: u16, row: u16| ((row * col) % 2 + (row * col) % 3) == 0,
            Mask::M6 => |col: u16, row: u16| ((row * col) % 2 + (row * col) % 3) % 2 == 0,
            Mask::M7 => |col: u16, row: u16| ((row + col) % 2 + (row * col) % 3) % 2 == 0,
        };

        for y in 0..self.matrix.width {
            for x in 0..self.matrix.width {
                let module = self.matrix.get_mut(x, y);
                if module.has(Module::DATA) {
                    *module ^= (mask_bit(x as u16, y as u16) as u8).into();
                }
            }
        }
    }
}
