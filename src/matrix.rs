use std::ops::BitOr;

use crate::{
    constants::{FORMAT_INFO, VERSION_INFO},
    data::Data,
    error_correction::ecc_and_sequence,
    mask::score,
    qrcode::{Mask, Mode, Version, ECL},
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

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy)]
pub struct Margin {
    pub top: usize,
    pub right: usize,
    pub bottom: usize,
    pub left: usize,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Margin {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new(margin: usize) -> Self {
        Margin {
            top: margin,
            right: margin,
            bottom: margin,
            left: margin,
        }
    }

    // in js, properties and methods share namespace
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = setTop))]
    pub fn top(mut self, top: usize) -> Self {
        self.top = top;
        self
    }
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = setRight))]
    pub fn right(mut self, right: usize) -> Self {
        self.right = right;
        self
    }
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = setBottom))]
    pub fn bottom(mut self, bottom: usize) -> Self {
        self.bottom = bottom;
        self
    }
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = setLeft))]
    pub fn left(mut self, left: usize) -> Self {
        self.left = left;
        self
    }
    pub fn y(mut self, y: usize) -> Self {
        self.top = y;
        self.bottom = y;
        self
    }
    pub fn x(mut self, x: usize) -> Self {
        self.left = x;
        self.right = x;
        self
    }
}

// Can't figure out how to conditionally apply #[wasm_bindgen(getter_with_clone)]
// implementing getter manually doesn't work either

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct Matrix {
    #[wasm_bindgen(getter_with_clone)]
    pub value: Vec<Module>,
    pub margin: Margin,
    pub mode: Mode,
    pub version: Version,
    pub ecl: ECL,
    pub mask: Mask,
}

#[cfg(not(feature = "wasm"))]
pub struct Matrix {
    pub value: Vec<Module>,
    pub margin: Margin,
    pub mode: Mode,
    pub version: Version,
    pub ecl: ECL,
    pub mask: Mask,
}

impl Matrix {
    pub fn new(data: Data, mask: Option<Mask>, margin: Margin) -> Self {
        let mut matrix = Matrix {
            value: Vec::new(),
            mode: data.mode,
            version: data.version,
            ecl: data.ecl,
            mask: if let Some(mask) = mask {
                mask
            } else {
                Mask::M0
            },
            margin,
        };
        matrix.value = vec![Module::Unset; matrix.width() * matrix.height()];

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

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl Matrix {
    // wasm-bindgen doesn't support trait impls
    #[wasm_bindgen(js_name=width)]
    pub fn width_wrapper(&self) -> usize {
        self.width()
    }
    #[wasm_bindgen(js_name=height)]
    pub fn height_wrapper(&self) -> usize {
        self.height()
    }
}

impl QrMatrix for Matrix {
    fn set(&mut self, x: usize, y: usize, module: Module) {
        // todo consider layout
        // Writing data means zigzag up and down, right to left
        let i = y * self.width() + x;
        self.value[i] = module;
    }
    fn get(&self, x: usize, y: usize) -> Module {
        let i = y * self.width() + x;
        self.value[i]
    }
    fn qr_width(&self) -> usize {
        self.version.0 * 4 + 17
    }
    fn margin(&self) -> Margin {
        self.margin
    }
}

pub trait QrMatrix {
    fn set(&mut self, x: usize, y: usize, module: Module);
    fn get(&self, x: usize, y: usize) -> Module;
    fn qr_width(&self) -> usize;
    fn margin(&self) -> Margin;

    fn width(&self) -> usize {
        self.qr_width() + self.margin().left + self.margin().right
    }
    fn height(&self) -> usize {
        self.qr_width() + self.margin().top + self.margin().bottom
    }

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
        self.iterate_finder_part(self.margin().left, self.margin().top, f);
        for i in 0..8 {
            f(
                self,
                self.margin().left + i,
                self.margin().top + 7,
                Module::FinderOFF,
            );
        }
        for i in 0..7 {
            f(
                self,
                self.margin().left + 7,
                self.margin().top + i,
                Module::FinderOFF,
            );
        }

        self.iterate_finder_part(
            self.margin().left,
            self.margin().top + self.qr_width() - 7,
            f,
        );
        for i in 0..8 {
            f(
                self,
                self.margin().left + i,
                self.margin().top + self.qr_width() - 8,
                Module::FinderOFF,
            );
        }
        for i in 0..7 {
            f(
                self,
                self.margin().left + 7,
                self.margin().top + self.qr_width() - 1 - i,
                Module::FinderOFF,
            );
        }

        self.iterate_finder_part(
            self.margin().left + self.qr_width() - 7,
            self.margin().top,
            f,
        );
        for i in 0..8 {
            f(
                self,
                self.margin().left + self.qr_width() - 1 - i,
                self.margin().top + 7,
                Module::FinderOFF,
            );
        }
        for i in 0..7 {
            f(
                self,
                self.margin().left + self.qr_width() - 8,
                self.margin().top + i,
                Module::FinderOFF,
            );
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
            f(self, self.margin().left + x, self.margin().top + y, module);

            let y = match i {
                i if i < 8 => 8,
                _ => self.qr_width() - (15 - i),
            };
            let x = match i {
                i if i < 8 => self.qr_width() - (i + 1),
                _ => 8,
            };
            f(self, self.margin().left + x, self.margin().top + y, module);
        }

        // always set
        f(
            self,
            self.margin().left + 8,
            self.margin().top + self.qr_width() - 8,
            Module::FormatON,
        );
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

            f(
                self,
                self.margin().left + x,
                self.margin().top + y + self.qr_width() - 11,
                module,
            );
            f(
                self,
                self.margin().left + y + self.qr_width() - 11,
                self.margin().top + x,
                module,
            );
        }
    }

    fn iterate_timing(&mut self, f: fn(&mut Self, usize, usize, Module)) {
        let len = self.qr_width() - 16;
        for i in 0..len {
            let module = Module::TimingOFF | ((i & 1) ^ 1) as u8;
            f(
                self,
                self.margin().left + 8 + i,
                self.margin().top + 6,
                module,
            );
            f(
                self,
                self.margin().left + 6,
                self.margin().top + 8 + i,
                module,
            );
        }
    }

    fn iterate_alignment(&mut self, version: Version, f: fn(&mut Self, usize, usize, Module)) {
        let version = version.0;
        if version == 1 {
            return;
        }

        let first = 6;
        let last = self.qr_width() - 7;
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
                    f(
                        self,
                        self.margin().left + col,
                        self.margin().top + row + i,
                        Module::AlignmentON,
                    )
                }

                for i in 1..4 {
                    f(
                        self,
                        self.margin().left + col + i,
                        self.margin().top + row,
                        Module::AlignmentON,
                    );
                    f(
                        self,
                        self.margin().left + col + i,
                        self.margin().top + row + 1,
                        Module::AlignmentOFF,
                    );
                    f(
                        self,
                        self.margin().left + col + i,
                        self.margin().top + row + 2,
                        Module::AlignmentOFF,
                    );
                    f(
                        self,
                        self.margin().left + col + i,
                        self.margin().top + row + 3,
                        Module::AlignmentOFF,
                    );
                    f(
                        self,
                        self.margin().left + col + i,
                        self.margin().top + row + 4,
                        Module::AlignmentON,
                    );
                }

                f(
                    self,
                    self.margin().left + col + 2,
                    self.margin().top + row + 2,
                    Module::AlignmentON,
                );

                for i in 0..5 {
                    f(
                        self,
                        self.margin().left + col + 4,
                        self.margin().top + row + i,
                        Module::AlignmentON,
                    )
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

        let mut col = self.qr_width() - 1;
        let mut row = self.qr_width() - 1;

        // get() coords wrong
        loop {
            loop {
                if self.get(self.margin().left + col, self.margin().top + row) == Module::Unset {
                    f(
                        self,
                        self.margin().left + col,
                        self.margin().top + row,
                        get_bit(&data, i),
                    );
                    i += 1;
                }
                if self.get(self.margin().left + col - 1, self.margin().top + row) == Module::Unset
                {
                    f(
                        self,
                        self.margin().left + col - 1,
                        self.margin().top + row,
                        get_bit(&data, i),
                    );
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
                if self.get(self.margin().left + col, self.margin().top + row) == Module::Unset {
                    f(
                        self,
                        self.margin().left + col,
                        self.margin().top + row,
                        get_bit(&data, i),
                    );
                    i += 1;
                }
                if self.get(self.margin().left + col - 1, self.margin().top + row) == Module::Unset
                {
                    f(
                        self,
                        self.margin().left + col - 1,
                        self.margin().top + row,
                        get_bit(&data, i),
                    );
                    i += 1;
                }
                if row == self.qr_width() - 1 {
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

        for i in 0..self.qr_width() {
            for j in 0..self.qr_width() {
                let module = self.get(self.margin().left + i, self.margin().top + j) as u8;
                if module | 1 != Module::DataON as u8 {
                    continue;
                }

                self.set(
                    self.margin().left + i,
                    self.margin().top + j,
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
