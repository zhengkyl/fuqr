use std::cmp::min;

use data::QRData;
use error_correction::{ECL, GEN_POLYNOMIALS, NUM_BLOCKS, NUM_CODEWORDS};
use math::{ANTILOG_TABLE, LOG_TABLE};
use qr::{encode_alphanumeric, Mode, QRCode};
use version::Version;

use crate::{
    symbol::{Symbol, MODULE},
    version::{format_information, version_information},
};

pub mod data;
pub mod error_correction;
pub mod math;
pub mod qr;
pub mod symbol;
pub mod version;

pub struct Segment<'a> {
    pub mode: Mode,
    pub text: &'a str, // max length is 7089 numeric, v40, low
}

pub fn encode(segments: Vec<Segment>, min_version: Version, max_versio: Version) -> QRCode {
    let mut qrcode = QRCode {
        sequenced_data: Vec::new(),
        ecl: ECL::Low,
        mask: 0,
        version: Version(1),
    };

    let mut qrdata = QRData::new(Version(1));

    // todo, ensure version can contain before encode, mathable
    encode_alphanumeric(&mut qrdata, "GREETINGS TRAVELER");

    let modules = qrdata.version.num_data_modules();
    let codewords = modules / 8;
    let remainder_bits = modules % 8;

    let num_ec_codewords = NUM_CODEWORDS[qrcode.ecl as usize][qrdata.version.0 as usize] as usize;
    let num_data_codewords = codewords - num_ec_codewords;

    // terminator
    let remainder_data_bits = (num_data_codewords * 8) - (qrdata.data.len());
    qrdata.push_bits(0, min(4, remainder_data_bits));

    // byte align
    let byte_pad = (8 - (qrdata.data.len() % 8)) % 8;
    qrdata.push_bits(0, byte_pad);

    // fill data capacity
    let data_pad = num_data_codewords - (qrdata.data.len() / 8);
    let mut alternating_byte = 0b11101100;
    for _ in 0..data_pad {
        qrdata.push_bits(alternating_byte, 8);
        alternating_byte ^= 0b11111101;
    }

    let blocks = NUM_BLOCKS[qrcode.ecl as usize][qrdata.version.0 as usize] as usize;

    let group_2_blocks = codewords % blocks;
    let group_1_blocks = blocks - group_2_blocks;

    let data_codewords = qrdata.data;

    let data_per_g1_block = num_data_codewords / blocks;
    let data_per_g2_block = data_per_g1_block + 1;

    let ecc_per_block = num_ec_codewords / blocks;
    let mut final_sequence = vec![0; codewords + (remainder_bits + 7) / 8];

    for i in 0..group_1_blocks * data_per_g1_block {
        let col = i % data_per_g1_block;
        let row = i / data_per_g1_block;
        final_sequence[col * blocks + row] = data_codewords[i];
    }
    for i in 0..group_2_blocks * data_per_g2_block {
        let col = i % data_per_g2_block;
        let row = i / data_per_g2_block;

        // 0 iff last column, else group_1_blocks
        let row_offset = (1 - (col / (data_per_g2_block - 1))) * group_1_blocks;
        final_sequence[col * blocks + row + row_offset] =
            data_codewords[i + (group_1_blocks * data_per_g1_block)];
    }

    for i in 0..group_1_blocks {
        let start = i * data_per_g1_block;
        let ec_codewords = remainder(
            &data_codewords[(start)..(start + data_per_g1_block)],
            &GEN_POLYNOMIALS[ecc_per_block][..ecc_per_block],
        );

        for j in 0..ec_codewords.len() {
            final_sequence[num_data_codewords + j * blocks + i] = ec_codewords[j];
        }
    }

    let group_2_start = group_1_blocks * data_per_g1_block;

    for i in 0..group_2_blocks {
        let start = group_2_start + i * data_per_g2_block;
        let ec_codewords = remainder(
            &data_codewords[(start)..(start + data_per_g2_block)],
            &GEN_POLYNOMIALS[ecc_per_block][..ecc_per_block],
        );

        for j in 0..ec_codewords.len() {
            final_sequence[num_data_codewords + j * blocks + i + group_1_blocks] = ec_codewords[j];
        }
    }
    // todo
    qrcode.sequenced_data = final_sequence;
    qrcode
}

pub fn place(qrcode: &QRCode) -> Symbol {
    let mut symbol = Symbol::new(qrcode.version.0);
    let width = qrcode.version.0 as usize * 4 + 17;

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

    fn place_align(symbol: &mut Symbol, version: usize) {
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

    fn place_version(symbol: &mut Symbol, version: usize) {
        if version < 7 {
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

    let format_info = format_information(qrcode);
    place_format(&mut symbol, format_info);
    place_timing(&mut symbol);

    place_version(&mut symbol, qrcode.version.0 as usize);
    place_align(&mut symbol, qrcode.version.0 as usize);

    let mut i = 0;

    let mut col = symbol.width - 1;
    let mut row = symbol.width - 1;

    fn place_module(symbol: &mut Symbol, col: usize, row: usize, data: &Vec<u8>, i: &mut usize) {
        if symbol.get(col, row) == MODULE::UNSET {
            let on = data[*i / 8] & (1 << (7 - (*i % 8))) != 0;
            *i += 1;

            let mask = (col + row) & 1 == 0;
            symbol.set(col, row, on ^ mask);
        }
    }

    loop {
        loop {
            place_module(&mut symbol, col, row, &qrcode.sequenced_data, &mut i);
            place_module(&mut symbol, col - 1, row, &qrcode.sequenced_data, &mut i);
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
            place_module(&mut symbol, col, row, &qrcode.sequenced_data, &mut i);
            place_module(&mut symbol, col - 1, row, &qrcode.sequenced_data, &mut i);
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

    // TODO masking -> blindly mask then reapply fixed patterns?
    // or use enum for modules? 0, 1, 2=empty?

    symbol
}

const ALIGN_COORD: [usize; 34] = [
    16, 18, 20, 22, 24, 26, 28, // 7-13
    20, 22, 24, 24, 26, 28, 28, // 14-20
    22, 24, 24, 26, 26, 28, 28, // 21-27
    24, 24, 26, 26, 26, 28, 28, // 28-34
    24, 26, 26, 26, 28, 28, // 35-40
];

fn remainder(data: &[u8], generator: &[u8]) -> Vec<u8> {
    let num_codewords = generator.len();

    // todo double check this length
    let mut base = [0; 124 + 30];

    base[..data.len()].copy_from_slice(data);

    for i in 0..data.len() {
        if base[i] == 0 {
            continue;
        }

        let alpha_diff = ANTILOG_TABLE[base[i] as usize];

        for j in 0..generator.len() {
            base[i + j + 1] ^= LOG_TABLE[(generator[j] as usize + alpha_diff as usize) % 255];
        }
    }

    base[data.len()..(data.len() + num_codewords)].to_vec()
}
