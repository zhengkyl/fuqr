use crate::{
    encode::{format_information, version_information},
    qrcode::{Mask, QRCode, Version, ECL},
    ALIGN_COORD,
};

// todo, should be possible to set branchless
// roughly in order of most to least
#[derive(Clone, Copy, PartialEq)]
pub enum Module {
    DataOFF,
    DataON,

    FinderOFF,
    FinderON,

    AlignmentOFF,
    AlignmentON,

    FormatOFF,
    FormatON,

    TimingOFF,
    TimingON,

    VersionOFF,
    VersionON,

    Separator,
    Unset,
}

pub struct Matrix {
    pub width: usize,
    pub modules: Vec<Module>,
}

impl Matrix {
    pub fn new(version: usize) -> Self {
        let width = version * 4 + 17;
        Matrix {
            width: width,
            modules: vec![Module::Unset; width * width],
        }
    }
    pub fn set(&mut self, x: usize, y: usize, module: Module) {
        // todo consider layout
        // Writing data means zigzag up and down, right to left
        let i = x * self.width + y;
        self.modules[i] = module;
    }
    pub fn get(&self, x: usize, y: usize) -> Module {
        let i = x * self.width + y;
        self.modules[i]
    }
}

fn place_finder(matrix: &mut Matrix) {
    fn place_pattern(matrix: &mut Matrix, col: usize, mut row: usize) {
        for i in 0..7 {
            matrix.set(col + i, row, Module::FinderON);
        }
        row += 1;

        matrix.set(col + 0, row, Module::FinderON);
        for i in 1..6 {
            matrix.set(col + i, row, Module::FinderOFF);
        }
        matrix.set(col + 6, row, Module::FinderON);
        row += 1;

        for _ in 0..3 {
            matrix.set(col + 0, row, Module::FinderON);
            matrix.set(col + 1, row, Module::FinderOFF);
            matrix.set(col + 2, row, Module::FinderON);
            matrix.set(col + 3, row, Module::FinderON);
            matrix.set(col + 4, row, Module::FinderON);
            matrix.set(col + 5, row, Module::FinderOFF);
            matrix.set(col + 6, row, Module::FinderON);
            row += 1;
        }

        matrix.set(col + 0, row, Module::FinderON);
        for i in 1..6 {
            matrix.set(col + i, row, Module::FinderOFF);
        }
        matrix.set(col + 6, row, Module::FinderON);
        row += 1;

        for i in 0..7 {
            matrix.set(col + i, row, Module::FinderON);
        }
    }

    place_pattern(matrix, 0, 0);
    for i in 0..8 {
        matrix.set(i, 7, Module::Separator);
    }
    for i in 0..7 {
        matrix.set(7, i, Module::Separator);
    }

    let width = matrix.width;
    place_pattern(matrix, 0, width - 7);
    for i in 0..8 {
        matrix.set(i, matrix.width - 8, Module::Separator);
    }
    for i in 0..7 {
        matrix.set(7, matrix.width - 1 - i, Module::Separator);
    }

    place_pattern(matrix, width - 7, 0);
    for i in 0..8 {
        matrix.set(matrix.width - 1 - i, 7, Module::Separator);
    }
    for i in 0..7 {
        matrix.set(matrix.width - 8, i, Module::Separator);
    }
}

fn place_format(matrix: &mut Matrix, ecl: ECL, mask: Mask) {
    let format_info = format_information(ecl, mask);
    for i in 0..15 {
        let module = if ((format_info >> i) & 1) == 1 {
            Module::FormatON
        } else {
            Module::FormatOFF
        };

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
        matrix.set(x, y, module);

        let y = match i {
            i if i < 8 => 8,
            _ => matrix.width - (15 - i),
        };
        let x = match i {
            i if i < 8 => matrix.width - (i + 1),
            _ => 8,
        };
        matrix.set(x, y, module);
    }

    // always set
    matrix.set(8, matrix.width - 8, Module::FormatON);
}

fn place_version(matrix: &mut Matrix, version: Version) {
    if version.0 < 7 {
        return;
    }
    let info = version_information(version);

    for i in 0..18 {
        let module = if (info >> i) & 1 == 1 {
            Module::VersionON
        } else {
            Module::VersionOFF
        };

        let x = i / 3;
        let y = i % 3;

        matrix.set(x, y + matrix.width - 11, module);
        matrix.set(y + matrix.width - 11, x, module);
    }
}

fn place_timing(matrix: &mut Matrix) {
    let len = matrix.width - 16;
    for i in 0..len {
        let module = if i & 1 == 0 {
            Module::TimingON
        } else {
            Module::TimingOFF
        };
        matrix.set(8 + i, 6, module);
        matrix.set(6, 8 + i, module);
    }
}

fn place_alignment(matrix: &mut Matrix, version: Version) {
    let version = version.0;
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
            coords.push(last - i * ALIGN_COORD[version - 7]);
        }
    }
    coords.push(last);

    for i in 0..len {
        for j in 0..len {
            if (i == 0 && j == 0) || (i == 0 && j == len - 1) || (i == len - 1 && j == 0) {
                continue;
            }

            let col = coords[i] - 2;
            let row = coords[j] - 2;

            for i in 0..5 {
                matrix.set(col, row + i, Module::AlignmentON)
            }

            for i in 1..4 {
                matrix.set(col + i, row, Module::AlignmentON);
                matrix.set(col + i, row + 1, Module::AlignmentOFF);
                matrix.set(col + i, row + 2, Module::AlignmentOFF);
                matrix.set(col + i, row + 3, Module::AlignmentOFF);
                matrix.set(col + i, row + 4, Module::AlignmentON);
            }

            matrix.set(col + 2, row + 2, Module::AlignmentON);

            for i in 0..5 {
                matrix.set(col + 4, row + i, Module::AlignmentON)
            }
        }
    }
}

pub fn place_all(matrix: &mut Matrix, qrcode: &QRCode) {
    place_finder(matrix);
    place_format(matrix, qrcode.ecl, qrcode.mask);
    place_timing(matrix);
    place_version(matrix, qrcode.version);
    place_alignment(matrix, qrcode.version);
    place_data(matrix, qrcode);
}

// This depends on all placements occuring beforehand
pub fn place_data(matrix: &mut Matrix, qrcode: &QRCode) {
    let mut i = 0;

    let mut col = matrix.width - 1;
    let mut row = matrix.width - 1;

    fn place_module(matrix: &mut Matrix, qrcode: &QRCode, col: usize, row: usize, i: &mut usize) {
        if matrix.get(col, row) == Module::Unset {
            let mask_bit = match qrcode.mask {
                Mask(0) => (row + col) % 2 == 0,
                Mask(1) => (row) % 2 == 0,
                Mask(2) => (col) % 3 == 0,
                Mask(3) => (row + col) % 3 == 0,
                Mask(4) => ((row / 2) + (col / 3)) % 2 == 0,
                Mask(5) => ((row * col) % 2 + (row * col) % 3) == 0,
                Mask(6) => ((row * col) % 2 + (row * col) % 3) % 2 == 0,
                Mask(7) => ((row + col) % 2 + (row * col) % 3) % 2 == 0,
                _ => unreachable!("bad mask"),
            } as u8;
            let module = if ((qrcode.codewords[*i / 8] >> (7 - (*i % 8))) & 1) ^ mask_bit == 1 {
                Module::DataON
            } else {
                Module::DataOFF
            };
            *i += 1;

            matrix.set(col, row, module);
        }
    }

    loop {
        loop {
            place_module(matrix, qrcode, col, row, &mut i);
            place_module(matrix, qrcode, col - 1, row, &mut i);
            if row == 0 {
                break;
            }
            row -= 1;
        }

        col -= 2;
        if col == 6 {
            col -= 1;
        }

        loop {
            place_module(matrix, qrcode, col, row, &mut i);
            place_module(matrix, qrcode, col - 1, row, &mut i);
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
