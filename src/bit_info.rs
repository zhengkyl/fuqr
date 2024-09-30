use std::ops::BitOrAssign;

use crate::{
    constants::{NUM_BLOCKS, NUM_DATA_MODULES, NUM_EC_CODEWORDS},
    encoding::num_cci_bits,
    matrix::{Matrix, Module},
    qr_code::{Mask, Mode, Version, ECL},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bit {
    /// Module except meaning changes when DATA set
    pub module: Module,
    /// 1-indexed error correction block
    pub block: u8,
    /// 0-indexed bit index
    pub index: u16,
}

impl Bit {
    pub const HEADER: Module = Module(Module::DATA.0 | Module::ON.0);
    pub const MESSAGE: Module = Module(Module::DATA.0 | Module::ON.0 | Module::MODIFIER.0);
    pub const ERROR_CORRECTION: Module = Module(Module::DATA.0 | Module::MODIFIER.0);
    pub const REMAINDER: Module = Module(Module::DATA.0);
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
    pub fn new(mode: Mode, version: Version, ecl: ECL) -> Self {
        let mut bit_info = BitInfo {
            matrix: Matrix::new(
                version,
                Bit {
                    module: Module(0),
                    block: 0,
                    index: 0,
                },
            ),
            mode,
            version,
            ecl,
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
        let mut module = Bit::HEADER;

        let header_len = 4 + num_cci_bits(bit_info.version, bit_info.mode);

        bit_info.matrix.set_data(|| {
            let val = Bit {
                module,
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
                    module = if index < header_len as u16 {
                        Bit::HEADER
                    } else {
                        Bit::MESSAGE
                    };
                } else {
                    let ecc_i = byte_i - num_data_codewords;

                    if ecc_i < num_ec_codewords {
                        let col = ecc_i / blocks;
                        let row = ecc_i % blocks;

                        block = (row + 1) as u8; // block is 1-index
                        index = (row * ecc_per_block + col) * 8;
                        module = Bit::ERROR_CORRECTION;
                    } else {
                        block = 0;
                        index = 0;
                        module = Bit::REMAINDER;
                    }
                }
            }

            val
        });

        bit_info
    }
}
