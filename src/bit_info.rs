use std::ops::BitOrAssign;

use crate::{
    constants::{NUM_BLOCKS, NUM_DATA_MODULES, NUM_EC_CODEWORDS},
    matrix::{Matrix, Module},
    qr_code::{Mask, Mode, Version, ECL},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Info {
    /// Module except meaning changes when DATA set
    pub module: Module,
    /// 0-indexed error correction block
    pub block: u8,
    /// bit index within block
    pub bit: u16,
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
            block: 0,
            bit: 0,
        }
    }
}

impl BitOrAssign<Module> for Info {
    fn bitor_assign(&mut self, rhs: Module) {
        self.module |= rhs;
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
                    block: 0,
                    bit: 0,
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

        let data_end = num_data_codewords * 8;
        let ecc_end = codewords * 8;

        let mut i = 0;
        let mut block = 0;
        let mut bit = 0;

        bit_info.matrix.set_data(|| {
            let val = Info {
                module: match i {
                    j if j < data_end => Info::DATA,
                    j if j < ecc_end * 8 => Info::EC,
                    _ => Info::REMAINDER,
                },
                block,
                bit,
            };
            bit += 1;
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

                block = row as u8;
                bit = (col * 8) as u16;
            } else if i < ecc_end {
                let ecc_i = (i / 8) - num_data_codewords;
                let col = ecc_i / blocks;
                let row = ecc_i % blocks;

                block = row as u8;

                bit = if row < group_1_blocks {
                    (data_per_g1_block + col) * 8
                } else {
                    (data_per_g1_block + 1 + col) * 8
                } as u16
            } else {
                block = 0;
                bit = 0;
            }

            val
        });

        bit_info
    }
}
