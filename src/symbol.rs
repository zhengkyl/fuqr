use std::fmt;

use crate::{
    encode::{format_information, version_information},
    qrcode::{Mask, QRCode, Version},
    ALIGN_COORD,
};

#[derive(Clone, Copy, PartialEq)]
pub enum MODULE {
    OFF = 0,
    ON = 1,
    UNSET = 2,
}

pub struct Symbol {
    pub width: usize,
    pub modules: Vec<MODULE>,
}

impl Symbol {
    pub fn new(version: usize) -> Self {
        let width = version * 4 + 17;
        Symbol {
            width: width,
            modules: vec![MODULE::UNSET; width * width],
        }
    }
    pub fn set(&mut self, x: usize, y: usize, on: bool) {
        // todo consider layout
        // Writing data means zigzag up and down, right to left
        let i = x * self.width + y;
        self.modules[i] = if on { MODULE::ON } else { MODULE::OFF };
    }
    pub fn get(&mut self, x: usize, y: usize) -> MODULE {
        // todo consider layout
        // Writing data means zigzag up and down, right to left
        let i = x * self.width + y;
        self.modules[i]
    }
}
impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Err(e) = writeln!(f) {
            return Err(e);
        }
        for y in (0..self.width).step_by(2) {
            for x in 0..self.width {
                let top = self.modules[x * self.width + y];
                let bot = if y < self.width - 1 {
                    self.modules[x * self.width + y + 1]
                } else {
                    MODULE::OFF
                };
                let c = match (top, bot) {
                    (MODULE::ON, MODULE::ON) => '█',
                    (MODULE::ON, MODULE::OFF) => '▀',
                    (MODULE::OFF, MODULE::ON) => '▄',
                    (MODULE::OFF, MODULE::OFF) => ' ',
                    _ => unreachable!("invalid symbol"),
                };
                if let Err(e) = write!(f, "{}", c) {
                    return Err(e);
                }
            }

            if let Err(e) = writeln!(f) {
                return Err(e);
            }
        }

        Ok(())
    }
}

pub fn place(qrcode: &QRCode, mask: Mask) -> Symbol {
    let mut symbol = Symbol::new(qrcode.version.0);
    let width = qrcode.version.0 * 4 + 17;

    fn place_finder(symbol: &mut Symbol, col: usize, mut row: usize) {
        for i in 0..7 {
            symbol.set(col + i, row, true);
        }
        row += 1;

        symbol.set(col + 0, row, true);
        for i in 1..6 {
            symbol.set(col + i, row, false);
        }
        symbol.set(col + 6, row, true);
        row += 1;

        for _ in 0..3 {
            symbol.set(col + 0, row, true);
            symbol.set(col + 1, row, false);
            symbol.set(col + 2, row, true);
            symbol.set(col + 3, row, true);
            symbol.set(col + 4, row, true);
            symbol.set(col + 5, row, false);
            symbol.set(col + 6, row, true);
            row += 1;
        }

        symbol.set(col + 0, row, true);
        for i in 1..6 {
            symbol.set(col + i, row, false);
        }
        symbol.set(col + 6, row, true);
        row += 1;

        for i in 0..7 {
            symbol.set(col + i, row, true);
        }
    }

    fn place_format(symbol: &mut Symbol, format_info: u32) {
        for i in 0..15 {
            let on = (format_info & (1 << i)) != 0;

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
            symbol.set(x, y, on);

            let y = match i {
                i if i < 8 => 8,
                _ => symbol.width - (15 - i),
            };
            let x = match i {
                i if i < 8 => symbol.width - (i + 1),
                _ => 8,
            };
            symbol.set(x, y, on);
        }

        // always set
        symbol.set(8, symbol.width - 8, true);
    }

    fn place_timing(symbol: &mut Symbol) {
        let len = symbol.width - 16;
        for i in 0..len {
            let even = i & 1 == 0;
            symbol.set(8 + i, 6, even);
            symbol.set(6, 8 + i, even);
        }
    }

    fn place_align(symbol: &mut Symbol, version: Version) {
        let version = version.0;
        if version == 1 {
            return;
        }

        let first = 6;
        let last = symbol.width - 7;
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
                    symbol.set(col, row + i, true)
                }

                for i in 1..4 {
                    symbol.set(col + i, row, true);
                    symbol.set(col + i, row + 1, false);
                    symbol.set(col + i, row + 2, false);
                    symbol.set(col + i, row + 3, false);
                    symbol.set(col + i, row + 4, true);
                }

                symbol.set(col + 2, row + 2, true);

                for i in 0..5 {
                    symbol.set(col + 4, row + i, true)
                }
            }
        }
    }

    fn place_version(symbol: &mut Symbol, version: Version) {
        if version.0 < 7 {
            return;
        }
        let info = version_information(version);

        for i in 0..18 {
            let on = info & (1 << i) != 0;

            let x = i / 3;
            let y = i % 3;

            symbol.set(x, y + symbol.width - 11, on);
            symbol.set(y + symbol.width - 11, x, on);
        }
    }

    place_finder(&mut symbol, 0, 0);
    for i in 0..8 {
        symbol.set(i, 7, false);
    }
    for i in 0..7 {
        symbol.set(7, i, false);
    }

    place_finder(&mut symbol, 0, width - 7);
    for i in 0..8 {
        symbol.set(i, symbol.width - 8, false);
    }
    for i in 0..7 {
        symbol.set(7, symbol.width - 1 - i, false);
    }

    place_finder(&mut symbol, width - 7, 0);
    for i in 0..8 {
        symbol.set(symbol.width - 1 - i, 7, false);
    }
    for i in 0..7 {
        symbol.set(symbol.width - 8, i, false);
    }

    let format_info = format_information(qrcode.ecl, mask);
    place_format(&mut symbol, format_info);
    place_timing(&mut symbol);

    place_version(&mut symbol, qrcode.version);
    place_align(&mut symbol, qrcode.version);

    let mut i = 0;

    let mut col = symbol.width - 1;
    let mut row = symbol.width - 1;

    fn place_module(
        symbol: &mut Symbol,
        mask: Mask,
        col: usize,
        row: usize,
        data: &Vec<u8>,
        i: &mut usize,
    ) {
        if symbol.get(col, row) == MODULE::UNSET {
            let on = data[*i / 8] & (1 << (7 - (*i % 8))) != 0;
            *i += 1;

            let mask_bit = match mask {
                Mask(0) => (row + col) % 2 == 0,
                Mask(1) => (row) % 2 == 0,
                Mask(2) => (col) % 3 == 0,
                Mask(3) => (row + col) % 3 == 0,
                Mask(4) => ((row / 2) + (col / 3)) % 2 == 0,
                Mask(5) => ((row * col) % 2 + (row * col) % 3) == 0,
                Mask(6) => ((row * col) % 2 + (row * col) % 3) % 2 == 0,
                Mask(7) => ((row + col) % 2 + (row * col) % 3) % 2 == 0,
                _ => unreachable!("bad mask"),
            };
            symbol.set(col, row, on ^ mask_bit);
        }
    }

    loop {
        loop {
            place_module(&mut symbol, mask, col, row, &qrcode.codewords, &mut i);
            place_module(&mut symbol, mask, col - 1, row, &qrcode.codewords, &mut i);
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
            place_module(&mut symbol, mask, col, row, &qrcode.codewords, &mut i);
            place_module(&mut symbol, mask, col - 1, row, &qrcode.codewords, &mut i);
            if row == symbol.width - 1 {
                break;
            }
            row += 1;
        }

        if col == 1 {
            break;
        }
        col -= 2;
    }

    symbol
}
