use std::ops::BitOrAssign;

use crate::{
    constants::{NUM_BLOCKS, NUM_DATA_MODULES, NUM_EC_CODEWORDS},
    data::Data,
    encoding::num_cci_bits,
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
        let mask_bit = mask_fn(mask);

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

pub fn mask_fn(mask: Mask) -> fn(u16, u16) -> bool {
    match mask {
        Mask::M0 => |col: u16, row: u16| (row + col) % 2 == 0,
        Mask::M1 => |_: u16, row: u16| (row) % 2 == 0,
        Mask::M2 => |col: u16, _: u16| (col) % 3 == 0,
        Mask::M3 => |col: u16, row: u16| (row + col) % 3 == 0,
        Mask::M4 => |col: u16, row: u16| ((row / 2) + (col / 3)) % 2 == 0,
        Mask::M5 => |col: u16, row: u16| ((row * col) % 2 + (row * col) % 3) == 0,
        Mask::M6 => |col: u16, row: u16| ((row * col) % 2 + (row * col) % 3) % 2 == 0,
        Mask::M7 => |col: u16, row: u16| ((row + col) % 2 + (row * col) % 3) % 2 == 0,
    }
}
