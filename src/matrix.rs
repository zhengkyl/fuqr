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

        place_all(&mut matrix, ecc_and_sequence(data));

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
                apply_mask(&mut matrix);

                matrix.mask = m;
                apply_mask(&mut matrix);
                iterate_format(&mut matrix, |matrix, col, row, on| {
                    matrix.set(col, row, Module::FormatOFF | on as u8)
                });
                let score = score(&matrix);
                if score < min_score {
                    min_score = score;
                    min_mask = matrix.mask;
                }
            }
            if min_mask != matrix.mask {
                // undo prev mask
                apply_mask(&mut matrix);

                matrix.mask = min_mask;
                apply_mask(&mut matrix);

                iterate_format(&mut matrix, |matrix, col, row, on| {
                    matrix.set(col, row, Module::FormatOFF | on as u8)
                });
            }
        }

        matrix
    }
    fn set(&mut self, x: usize, y: usize, module: Module) {
        // todo consider layout
        // Writing data means zigzag up and down, right to left
        let i = x * self.width + y;
        self.value[i] = module;
    }
    pub fn get(&self, x: usize, y: usize) -> Module {
        let i = x * self.width + y;
        self.value[i]
    }
}

fn place_all(matrix: &mut Matrix, data: Vec<u8>) {
    iterate_finder(matrix, |matrix, col, row, on| {
        matrix.set(col, row, Module::FinderOFF | on as u8)
    });
    iterate_format(matrix, |matrix, col, row, on| {
        matrix.set(col, row, Module::FormatOFF | on as u8)
    });
    iterate_timing(matrix, |matrix, col, row, on| {
        matrix.set(col, row, Module::TimingOFF | on as u8)
    });
    iterate_version(matrix, |matrix, col, row, on| {
        matrix.set(col, row, Module::VersionOFF | on as u8)
    });
    iterate_alignment(matrix, |matrix, col, row, on| {
        matrix.set(col, row, Module::AlignmentOFF | on as u8)
    });
    iterate_data(matrix, data, |matrix, col, row, on| {
        matrix.set(col, row, Module::DataOFF | on as u8)
    });
    apply_mask(matrix);
}

fn iterate_finder(matrix: &mut Matrix, f: fn(&mut Matrix, usize, usize, bool)) {
    fn iterate_pattern(
        matrix: &mut Matrix,
        col: usize,
        mut row: usize,
        f: fn(&mut Matrix, usize, usize, bool),
    ) {
        for i in 0..7 {
            f(matrix, col + i, row, true);
        }
        row += 1;

        f(matrix, col + 0, row, true);
        for i in 1..6 {
            f(matrix, col + i, row, false);
        }
        f(matrix, col + 6, row, true);
        row += 1;

        for _ in 0..3 {
            f(matrix, col + 0, row, true);
            f(matrix, col + 1, row, false);
            f(matrix, col + 2, row, true);
            f(matrix, col + 3, row, true);
            f(matrix, col + 4, row, true);
            f(matrix, col + 5, row, false);
            f(matrix, col + 6, row, true);
            row += 1;
        }

        f(matrix, col + 0, row, true);
        for i in 1..6 {
            f(matrix, col + i, row, false);
        }
        f(matrix, col + 6, row, true);
        row += 1;

        for i in 0..7 {
            f(matrix, col + i, row, true);
        }
    }

    iterate_pattern(matrix, 0, 0, f);
    for i in 0..8 {
        f(matrix, i, 7, false);
    }
    for i in 0..7 {
        f(matrix, 7, i, false);
    }

    let width = matrix.width;
    iterate_pattern(matrix, 0, width - 7, f);
    for i in 0..8 {
        f(matrix, i, matrix.width - 8, false);
    }
    for i in 0..7 {
        f(matrix, 7, matrix.width - 1 - i, false);
    }

    iterate_pattern(matrix, width - 7, 0, f);
    for i in 0..8 {
        f(matrix, matrix.width - 1 - i, 7, false);
    }
    for i in 0..7 {
        f(matrix, matrix.width - 8, i, false);
    }
}

fn iterate_format(matrix: &mut Matrix, f: fn(&mut Matrix, usize, usize, bool)) {
    let format_info = FORMAT_INFO[matrix.ecl as usize][matrix.mask as usize];
    for i in 0..15 {
        let on = ((format_info >> i) & 1) == 1;

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
        f(matrix, x, y, on);

        let y = match i {
            i if i < 8 => 8,
            _ => matrix.width - (15 - i),
        };
        let x = match i {
            i if i < 8 => matrix.width - (i + 1),
            _ => 8,
        };
        f(matrix, x, y, on);
    }

    // always set
    f(matrix, 8, matrix.width - 8, true);
}

fn iterate_version(matrix: &mut Matrix, f: fn(&mut Matrix, usize, usize, bool)) {
    if matrix.version.0 < 7 {
        return;
    }
    let info = VERSION_INFO[matrix.version.0];

    for i in 0..18 {
        let on = (info >> i) & 1 == 1;

        let x = i / 3;
        let y = i % 3;

        f(matrix, x, y + matrix.width - 11, on);
        f(matrix, y + matrix.width - 11, x, on);
    }
}

fn iterate_timing(matrix: &mut Matrix, f: fn(&mut Matrix, usize, usize, bool)) {
    let len = matrix.width - 16;
    for i in 0..len {
        let on = i & 1 == 0;
        f(matrix, 8 + i, 6, on);
        f(matrix, 6, 8 + i, on);
    }
}

const ALIGN_COORDS: [usize; 34] = [
    16, 18, 20, 22, 24, 26, 28, // 7-13
    20, 22, 24, 24, 26, 28, 28, // 14-20
    22, 24, 24, 26, 26, 28, 28, // 21-27
    24, 24, 26, 26, 26, 28, 28, // 28-34
    24, 26, 26, 26, 28, 28, // 35-40
];

fn iterate_alignment(matrix: &mut Matrix, f: fn(&mut Matrix, usize, usize, bool)) {
    let version = matrix.version.0;
    if version == 1 {
        return;
    }

    let first = 6;
    let last = matrix.width - 7;
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
                f(matrix, col, row + i, true)
            }

            for i in 1..4 {
                f(matrix, col + i, row, true);
                f(matrix, col + i, row + 1, false);
                f(matrix, col + i, row + 2, false);
                f(matrix, col + i, row + 3, false);
                f(matrix, col + i, row + 4, true);
            }

            f(matrix, col + 2, row + 2, true);

            for i in 0..5 {
                f(matrix, col + 4, row + i, true)
            }
        }
    }
}

// This depends on all placements occuring beforehand
fn iterate_data(matrix: &mut Matrix, data: Vec<u8>, f: fn(&mut Matrix, usize, usize, bool)) {
    fn get_bit(data: &Vec<u8>, i: usize) -> bool {
        // FOR FUTURE KYLE
        // i-th data bit
        // qrcode.value[i / 8] gets current codeword aka byte
        // 7 - (*i % 8) gets the current bit position in codeword (greatest to least order)
        ((data[i / 8] >> (7 - (i % 8))) & 1) == 1
    }

    let mut i = 0;

    let mut col = matrix.width - 1;
    let mut row = matrix.width - 1;

    loop {
        loop {
            if matrix.get(col, row) == Module::Unset {
                f(matrix, col, row, get_bit(&data, i));
                i += 1;
            }
            if matrix.get(col - 1, row) == Module::Unset {
                f(matrix, col - 1, row, get_bit(&data, i));
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
            if matrix.get(col, row) == Module::Unset {
                f(matrix, col, row, get_bit(&data, i));
                i += 1;
            }
            if matrix.get(col - 1, row) == Module::Unset {
                f(matrix, col - 1, row, get_bit(&data, i));
                i += 1;
            }
            if row == matrix.width - 1 {
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

fn apply_mask(matrix: &mut Matrix) {
    let mask_bit = match matrix.mask {
        Mask::M0 => |row: usize, col: usize| (row + col) % 2 == 0,
        Mask::M1 => |row: usize, _: usize| (row) % 2 == 0,
        Mask::M2 => |_: usize, col: usize| (col) % 3 == 0,
        Mask::M3 => |row: usize, col: usize| (row + col) % 3 == 0,
        Mask::M4 => |row: usize, col: usize| ((row / 2) + (col / 3)) % 2 == 0,
        Mask::M5 => |row: usize, col: usize| ((row * col) % 2 + (row * col) % 3) == 0,
        Mask::M6 => |row: usize, col: usize| ((row * col) % 2 + (row * col) % 3) % 2 == 0,
        Mask::M7 => |row: usize, col: usize| ((row + col) % 2 + (row * col) % 3) % 2 == 0,
    };

    for i in 0..matrix.width {
        for j in 0..matrix.width {
            let module = matrix.get(i, j) as u8;
            if module | 1 != Module::DataON as u8 {
                continue;
            }

            matrix.set(
                i,
                j,
                // TODO NOTE THAT ROW=j COL=i, DataOFF = 0, DataON = 1
                Module::DataOFF | (module ^ mask_bit(j, i) as u8),
            );
        }
    }
}
