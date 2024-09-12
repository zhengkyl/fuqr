use crate::{
    constants::{FORMAT_INFO, VERSION_INFO},
    data::Data,
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
    // no usage for Kanji, ECI, StructuredAppend, FNC1,
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
    pub matrix: Matrix,
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
            matrix: Matrix::new(data.version),
            mode: data.mode,
            version: data.version,
            ecl: data.ecl,
            mask: if let Some(mask) = mask {
                mask
            } else {
                Mask::M0
            },
        };
        qr_code.set_finder();
        qr_code.set_alignment();
        qr_code.set_timing();

        qr_code.set_format();
        qr_code.set_version();
        let data = ecc_and_sequence(data);
        qr_code.set_data(data);
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
                qr_code.set_format();
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

                qr_code.set_format();
            }
        }

        qr_code
    }

    fn set_finder(&mut self) {
        for (x, mut y) in [
            (0, 0),
            (0, self.matrix.width - 7),
            (self.matrix.width - 7, 0),
        ] {
            for i in 0..7 {
                self.matrix.set(x + i, y, Module::FINDER | Module::ON);
            }
            y += 1;

            self.matrix.set(x + 0, y, Module::FINDER | Module::ON);
            for i in 1..6 {
                self.matrix.set(x + i, y, Module::FINDER);
            }
            self.matrix.set(x + 6, y, Module::FINDER | Module::ON);
            y += 1;

            for _ in 0..3 {
                self.matrix.set(x + 0, y, Module::FINDER | Module::ON);
                self.matrix.set(x + 1, y, Module::FINDER);
                self.matrix
                    .set(x + 2, y, Module::FINDER_CENTER | Module::ON);
                self.matrix
                    .set(x + 3, y, Module::FINDER_CENTER | Module::ON);
                self.matrix
                    .set(x + 4, y, Module::FINDER_CENTER | Module::ON);
                self.matrix.set(x + 5, y, Module::FINDER);
                self.matrix.set(x + 6, y, Module::FINDER | Module::ON);
                y += 1;
            }

            self.matrix.set(x + 0, y, Module::FINDER | Module::ON);
            for i in 1..6 {
                self.matrix.set(x + i, y, Module::FINDER);
            }
            self.matrix.set(x + 6, y, Module::FINDER | Module::ON);
            y += 1;

            for i in 0..7 {
                self.matrix.set(x + i, y, Module::FINDER | Module::ON);
            }
        }
    }

    fn set_alignment(&mut self) {
        if self.version.0 == 1 {
            return;
        }

        let first = 6;
        let last = self.matrix.width as usize - 7;
        let len = self.version.0 as usize / 7 + 2;
        let mut coords = Vec::with_capacity(len);

        coords.push(first);
        if self.version.0 >= 7 {
            for i in (1..len - 1).rev() {
                coords.push((last - i * ALIGN_OFFSETS[(self.version.0 - 7) as usize]) as u8);
            }
        }
        coords.push(last as u8);

        for i in 0..len {
            for j in 0..len {
                if (i == 0 && (j == 0 || j == len - 1)) || (i == len - 1 && j == 0) {
                    continue;
                }

                let col = coords[i] - 2;
                let row = coords[j] - 2;

                for i in 0..5 {
                    self.matrix
                        .set(col, row + i, Module::ALIGNMENT | Module::ON);
                }

                for i in 1..4 {
                    self.matrix
                        .set(col + i, row + 0, Module::ALIGNMENT | Module::ON);
                    self.matrix.set(col + i, row + 1, Module::ALIGNMENT);
                    self.matrix.set(col + i, row + 2, Module::ALIGNMENT);
                    self.matrix.set(col + i, row + 3, Module::ALIGNMENT);
                    self.matrix
                        .set(col + i, row + 4, Module::ALIGNMENT | Module::ON);
                }

                self.matrix
                    .set(col + 2, row + 2, Module::ALIGNMENT_CENTER | Module::ON);

                for i in 0..5 {
                    self.matrix
                        .set(col + 4, row + i, Module::ALIGNMENT | Module::ON)
                }
            }
        }
    }

    fn set_timing(&mut self) {
        // overlaps with alignment pattern so must |=
        let len = self.matrix.width - 16;
        for i in 0..len {
            let module = Module::TIMING | ((i & 1) ^ 1).into();
            *self.matrix.get_mut(8 + i, 6) |= module;
        }
        for i in 0..len {
            let module = Module::TIMING | ((i & 1) ^ 1).into();
            *self.matrix.get_mut(6, 8 + i) |= module;
        }
    }

    fn set_format(&mut self) {
        let format_info = FORMAT_INFO[self.ecl as usize][self.mask as usize];
        for i in 0..15 {
            let on = ((format_info >> i) as u8 & 1).into();

            let y = match i {
                i if i < 6 => i,
                6 => 7,
                _ => 8,
            };
            let x = match i {
                i if i < 8 => 8,
                8 => 7,
                _ => 14 - i,
            };
            self.matrix.set(x, y, Module::FORMAT | on);

            let y = match i {
                i if i < 8 => 8,
                _ => self.matrix.width - (15 - i),
            };
            let x = match i {
                i if i < 8 => self.matrix.width - (i + 1),
                _ => 8,
            };
            self.matrix.set(x, y, Module::FORMAT_COPY | on);
        }

        // always set bit, not part of format info
        self.matrix
            .set(8, self.matrix.width - 8, Module::FORMAT_COPY | Module::ON);
    }

    fn set_version(&mut self) {
        if self.version.0 < 7 {
            return;
        }
        let info = VERSION_INFO[self.version.0 as usize];

        for i in 0..18 {
            let on = ((info >> i) as u8 & 1).into();

            let x = i / 3;
            let y = i % 3;

            self.matrix
                .set(x, y + self.matrix.width - 11, Module::VERSION | on);
            self.matrix
                .set(y + self.matrix.width - 11, x, Module::VERSION_COPY | on);
        }
    }

    /// This must run AFTER everything else placed
    fn set_data(&mut self, data: Vec<u8>) {
        fn get_bit(data: &Vec<u8>, i: usize) -> Module {
            // qrcode.value[i / 8] gets current byte
            // 7 - (i % 8) gets the current bit position in byte (greatest to least order)
            Module::DATA | ((data[i / 8] >> (7 - (i % 8))) & 1).into()
            // Module::DATA | 1.into()
        }

        let mut i = 0;

        let mut col = self.matrix.width - 1;
        let mut row = self.matrix.width - 1;

        let mut row_dir = -1;
        let mut row_limit = 8;

        let mut top_bot_gap = (self.matrix.width - 9) as isize;

        loop {
            loop {
                if self.matrix.get(col, row) == Module(0) {
                    self.matrix.set(col, row, get_bit(&data, i));
                    i += 1;
                }
                if self.matrix.get(col - 1, row) == Module(0) {
                    self.matrix.set(col - 1, row, get_bit(&data, i));
                    i += 1;
                }
                if row == row_limit {
                    break;
                }
                row = ((row as isize) + row_dir) as u8;
            }

            if col == 1 {
                break;
            }

            col -= 2;
            row_dir *= -1;

            // passed first finder
            if col == self.matrix.width - 9 {
                top_bot_gap = (self.matrix.width - 1) as isize;
                row_limit = 0;
            }
            // between left finders
            else if col == 8 {
                top_bot_gap = (self.matrix.width - 17) as isize;
                row_limit = 8;
                row = self.matrix.width - 9;
            } else {
                // vertical timing belt
                if col == 6 {
                    col -= 1;
                }
                row_limit = (row_limit as isize + top_bot_gap * row_dir) as u8;
            }
        }
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

        // TODO maybe u16?
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

const ALIGN_OFFSETS: [usize; 34] = [
    16, 18, 20, 22, 24, 26, 28, // 7-13
    20, 22, 24, 24, 26, 28, 28, // 14-20
    22, 24, 24, 26, 26, 28, 28, // 21-27
    24, 24, 26, 26, 26, 28, 28, // 28-34
    24, 26, 26, 26, 28, 28, // 35-40
];
