use std::ops::BitOr;

use crate::{
    constants::{FORMAT_INFO, VERSION_INFO},
    data::Data,
    error_correction::ecc_and_sequence,
    mask::score,
    qrcode::{Mask, Version, ECL},
};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

// todo, should be possible to set branchless
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy, PartialEq)]
pub enum Module {
    DataOFF,
    DataON,

    FinderOFF,
    FinderON,

    AlignmentOFF,
    AlignmentON,

    TimingOFF,
    TimingON,

    FormatOFF,
    FormatON,

    VersionOFF,
    VersionON,

    Unset,
}

impl BitOr<u8> for Module {
    type Output = Module;
    fn bitor(self, rhs: u8) -> Self::Output {
        unsafe { std::mem::transmute(self as u8 | rhs) }
    }
}

// Can't figure out how to conditionally apply #[wasm_bindgen(getter_with_clone)]
// implementing getter manually doesn't work either

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct Matrix {
    #[wasm_bindgen(getter_with_clone)]
    pub value: Vec<Module>,
    pub width: usize,
    pub version: Version,
    pub ecl: ECL,
    pub mask: Mask,
}

#[cfg(not(feature = "wasm"))]
pub struct Matrix {
    pub value: Vec<Module>,
    pub width: usize,
    pub version: Version,
    pub ecl: ECL,
    pub mask: Mask,
}

impl Matrix {
    pub fn new(data: Data, mask: Option<Mask>) -> Self {
        let width = data.version.0 * 4 + 17;
        let mut matrix = Matrix {
            width,
            value: vec![Module::Unset; width * width],
            version: data.version,
            ecl: data.ecl,
            mask: if let Some(mask) = mask {
                mask
            } else {
                Mask::M0
            },
        };

        matrix.iterate_finder(|matrix, col, row, module| matrix.set(col, row, module));
        matrix.iterate_format(matrix.ecl, matrix.mask, |matrix, col, row, module| {
            matrix.set(col, row, module)
        });
        matrix.iterate_timing(|matrix, col, row, module| matrix.set(col, row, module));
        matrix.iterate_version(matrix.version, |matrix, col, row, module| {
            matrix.set(col, row, module)
        });
        matrix.iterate_alignment(matrix.version, |matrix, col, row, module| {
            matrix.set(col, row, module)
        });
        let data = ecc_and_sequence(data);
        matrix.iterate_data(data, |matrix, col, row, module| {
            matrix.set(col, row, module)
        });
        matrix.apply_mask(matrix.mask);

        if let None = mask {
            let mut min_score = score(&matrix);
            let mut min_mask = matrix.mask;
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
                matrix.apply_mask(matrix.mask);

                matrix.mask = m;
                matrix.apply_mask(matrix.mask);
                matrix.iterate_format(matrix.ecl, matrix.mask, |matrix, col, row, module| {
                    matrix.set(col, row, module)
                });
                let score = score(&matrix);
                if score < min_score {
                    min_score = score;
                    min_mask = matrix.mask;
                }
            }
            if min_mask != matrix.mask {
                // undo prev mask
                matrix.apply_mask(matrix.mask);

                matrix.mask = min_mask;
                matrix.apply_mask(matrix.mask);

                matrix.iterate_format(matrix.ecl, matrix.mask, |matrix, col, row, module| {
                    matrix.set(col, row, module)
                });
            }
        }

        matrix
    }
}

impl QrMatrix for Matrix {
    fn set(&mut self, x: usize, y: usize, module: Module) {
        // todo consider layout
        // Writing data means zigzag up and down, right to left
        let i = x * self.width + y;
        self.value[i] = module;
    }
    fn get(&self, x: usize, y: usize) -> Module {
        let i = x * self.width + y;
        self.value[i]
    }
    fn width(&self) -> usize {
        self.width
    }
}

pub trait QrMatrix {
    fn set(&mut self, x: usize, y: usize, module: Module);
    fn get(&self, x: usize, y: usize) -> Module;
    fn width(&self) -> usize;

    fn iterate_finder_part(
        &mut self,
        col: usize,
        mut row: usize,
        f: fn(&mut Self, usize, usize, Module),
    ) {
        for i in 0..7 {
            f(self, col + i, row, Module::FinderON);
        }
        row += 1;

        f(self, col + 0, row, Module::FinderON);
        for i in 1..6 {
            f(self, col + i, row, Module::FinderOFF);
        }
        f(self, col + 6, row, Module::FinderON);
        row += 1;

        for _ in 0..3 {
            f(self, col + 0, row, Module::FinderON);
            f(self, col + 1, row, Module::FinderOFF);
            f(self, col + 2, row, Module::FinderON);
            f(self, col + 3, row, Module::FinderON);
            f(self, col + 4, row, Module::FinderON);
            f(self, col + 5, row, Module::FinderOFF);
            f(self, col + 6, row, Module::FinderON);
            row += 1;
        }

        f(self, col + 0, row, Module::FinderON);
        for i in 1..6 {
            f(self, col + i, row, Module::FinderOFF);
        }
        f(self, col + 6, row, Module::FinderON);
        row += 1;

        for i in 0..7 {
            f(self, col + i, row, Module::FinderON);
        }
    }

    fn iterate_finder(&mut self, f: fn(&mut Self, usize, usize, Module)) {
        self.iterate_finder_part(0, 0, f);
        for i in 0..8 {
            f(self, i, 7, Module::FinderOFF);
        }
        for i in 0..7 {
            f(self, 7, i, Module::FinderOFF);
        }

        self.iterate_finder_part(0, self.width() - 7, f);
        for i in 0..8 {
            f(self, i, self.width() - 8, Module::FinderOFF);
        }
        for i in 0..7 {
            f(self, 7, self.width() - 1 - i, Module::FinderOFF);
        }

        self.iterate_finder_part(self.width() - 7, 0, f);
        for i in 0..8 {
            f(self, self.width() - 1 - i, 7, Module::FinderOFF);
        }
        for i in 0..7 {
            f(self, self.width() - 8, i, Module::FinderOFF);
        }
    }

    fn iterate_format(&mut self, ecl: ECL, mask: Mask, f: fn(&mut Self, usize, usize, Module)) {
        let format_info = FORMAT_INFO[ecl as usize][mask as usize];
        for i in 0..15 {
            let module = Module::FormatOFF | ((format_info >> i) & 1) as u8;

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
            f(self, x, y, module);

            let y = match i {
                i if i < 8 => 8,
                _ => self.width() - (15 - i),
            };
            let x = match i {
                i if i < 8 => self.width() - (i + 1),
                _ => 8,
            };
            f(self, x, y, module);
        }

        // always set
        f(self, 8, self.width() - 8, Module::FormatON);
    }

    fn iterate_version(&mut self, version: Version, f: fn(&mut Self, usize, usize, Module)) {
        if version.0 < 7 {
            return;
        }
        let info = VERSION_INFO[version.0];

        for i in 0..18 {
            let module = Module::VersionOFF | ((info >> i) & 1) as u8;

            let x = i / 3;
            let y = i % 3;

            f(self, x, y + self.width() - 11, module);
            f(self, y + self.width() - 11, x, module);
        }
    }

    fn iterate_timing(&mut self, f: fn(&mut Self, usize, usize, Module)) {
        let len = self.width() - 16;
        for i in 0..len {
            let module = Module::TimingOFF | ((i & 1) ^ 1) as u8;
            f(self, 8 + i, 6, module);
            f(self, 6, 8 + i, module);
        }
    }

    fn iterate_alignment(&mut self, version: Version, f: fn(&mut Self, usize, usize, Module)) {
        let version = version.0;
        if version == 1 {
            return;
        }

        let first = 6;
        let last = self.width() - 7;
        let len = version / 7 + 2;
        let mut coords = Vec::with_capacity(len);

        coords.push(first);
        if version >= 7 {
            for i in (1..len - 1).rev() {
                coords.push(last - i * ALIGN_COORDS[version - 7]);
            }
        }
        coords.push(last);

        for i in 0..len {
            for j in 0..len {
                if (i == 0 && (j == 0 || j == len - 1)) || (i == len - 1 && j == 0) {
                    continue;
                }

                let col = coords[i] - 2;
                let row = coords[j] - 2;

                for i in 0..5 {
                    f(self, col, row + i, Module::AlignmentON)
                }

                for i in 1..4 {
                    f(self, col + i, row, Module::AlignmentON);
                    f(self, col + i, row + 1, Module::AlignmentOFF);
                    f(self, col + i, row + 2, Module::AlignmentOFF);
                    f(self, col + i, row + 3, Module::AlignmentOFF);
                    f(self, col + i, row + 4, Module::AlignmentON);
                }

                f(self, col + 2, row + 2, Module::AlignmentON);

                for i in 0..5 {
                    f(self, col + 4, row + i, Module::AlignmentON)
                }
            }
        }
    }

    // This depends on all placements occuring beforehand
    fn iterate_data(&mut self, data: Vec<u8>, f: fn(&mut Self, usize, usize, Module)) {
        fn get_bit(data: &Vec<u8>, i: usize) -> Module {
            // FOR FUTURE KYLE
            // i-th data bit
            // qrcode.value[i / 8] gets current codeword aka byte
            // 7 - (*i % 8) gets the current bit position in codeword (greatest to least order)
            Module::DataOFF | ((data[i / 8] >> (7 - (i % 8))) & 1)
        }

        let mut i = 0;

        let mut col = self.width() - 1;
        let mut row = self.width() - 1;

        loop {
            loop {
                if self.get(col, row) == Module::Unset {
                    f(self, col, row, get_bit(&data, i));
                    i += 1;
                }
                if self.get(col - 1, row) == Module::Unset {
                    f(self, col - 1, row, get_bit(&data, i));
                    i += 1;
                }
                if row == 0 {
                    break;
                }
                row -= 1;
            }

            col -= 2;
            // col 6 is vertical timing belt
            if col == 6 {
                col -= 1;
            }

            loop {
                if self.get(col, row) == Module::Unset {
                    f(self, col, row, get_bit(&data, i));
                    i += 1;
                }
                if self.get(col - 1, row) == Module::Unset {
                    f(self, col - 1, row, get_bit(&data, i));
                    i += 1;
                }
                if row == self.width() - 1 {
                    break;
                }
                row += 1;
            }

            if col == 1 {
                break;
            }
            col -= 2;
        }
    }

    fn apply_mask(&mut self, mask: Mask) {
        let mask_bit = match mask {
            Mask::M0 => |row: usize, col: usize| (row + col) % 2 == 0,
            Mask::M1 => |row: usize, _: usize| (row) % 2 == 0,
            Mask::M2 => |_: usize, col: usize| (col) % 3 == 0,
            Mask::M3 => |row: usize, col: usize| (row + col) % 3 == 0,
            Mask::M4 => |row: usize, col: usize| ((row / 2) + (col / 3)) % 2 == 0,
            Mask::M5 => |row: usize, col: usize| ((row * col) % 2 + (row * col) % 3) == 0,
            Mask::M6 => |row: usize, col: usize| ((row * col) % 2 + (row * col) % 3) % 2 == 0,
            Mask::M7 => |row: usize, col: usize| ((row + col) % 2 + (row * col) % 3) % 2 == 0,
        };

        for i in 0..self.width() {
            for j in 0..self.width() {
                let module = self.get(i, j) as u8;
                if module | 1 != Module::DataON as u8 {
                    continue;
                }

                self.set(
                    i,
                    j,
                    // TODO NOTE THAT ROW=j COL=i, DataOFF = 0, DataON = 1
                    Module::DataOFF | (module ^ mask_bit(j, i) as u8),
                );
            }
        }
    }
}

const ALIGN_COORDS: [usize; 34] = [
    16, 18, 20, 22, 24, 26, 28, // 7-13
    20, 22, 24, 24, 26, 28, 28, // 14-20
    22, 24, 24, 26, 26, 28, 28, // 21-27
    24, 24, 26, 26, 26, 28, 28, // 28-34
    24, 26, 26, 26, 28, 28, // 35-40
];
