use std::cmp::min;

use data::{QRData, NUM_DATA_MODULES};
use encode::{encode_alphanumeric, encode_byte, encode_numeric};
use error_correction::{GEN_POLYNOMIALS, NUM_BLOCKS, NUM_EC_CODEWORDS};
use math::{ANTILOG_TABLE, LOG_TABLE};
use qrcode::{Mode, QRCode, Version, ECL};

pub mod data;
pub mod encode;
pub mod error_correction;
pub mod math;
pub mod qrcode;
pub mod symbol;

pub struct Segment<'a> {
    pub mode: Mode,
    pub text: &'a str, // max length is 7089 numeric, v40, low
}

pub fn encode(segments: Vec<Segment>, version: Version) -> QRCode {
    let mut qrdata = QRData::new(version);

    // let min_max_bits = (NUM_DATA_MODULES[version.0])
    //     - (NUM_EC_CODEWORDS[ECL::Low as usize][version.0] as usize * 8);

    let max_max_bits =
        (NUM_DATA_MODULES[40]) - (NUM_EC_CODEWORDS[ECL::Low as usize][40] as usize * 8);

    // TODO iff we cross a version with a diff header size, must recalculate everything?

    // todo, ensure version can contain before encode, mathable
    for segment in segments {
        match segment.mode {
            Mode::Numeric => encode_numeric(&mut qrdata, segment.text),
            Mode::Alphanumeric => encode_alphanumeric(&mut qrdata, segment.text),
            Mode::Byte => encode_byte(&mut qrdata, segment.text),
        }
        if qrdata.bit_len > max_max_bits {
            todo!();
        }
    }

    // data codewords depends on version
    // ec codewords depend on data + ECL
    QRCode {
        codewords: calc_ecc_and_sequence(qrdata, ECL::Low),
        version: version,
        ecl: ECL::Low,
    }
}

pub fn calc_ecc_and_sequence(mut qrdata: QRData, ecl: ECL) -> Vec<u8> {
    let modules = NUM_DATA_MODULES[qrdata.version.0];
    let codewords = modules / 8;
    let remainder_bits = modules % 8;

    let num_ec_codewords = NUM_EC_CODEWORDS[ecl as usize][qrdata.version.0] as usize;
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

    let blocks = NUM_BLOCKS[ecl as usize][qrdata.version.0] as usize;

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

    final_sequence
}

const ALIGN_COORD: [usize; 34] = [
    16, 18, 20, 22, 24, 26, 28, // 7-13
    20, 22, 24, 24, 26, 28, 28, // 14-20
    22, 24, 24, 26, 26, 28, 28, // 21-27
    24, 24, 26, 26, 26, 28, 28, // 28-34
    24, 26, 26, 26, 28, 28, // 35-40
];

// todo
// benchmark potential optimizations
pub fn remainder(data: &[u8], generator: &[u8]) -> Vec<u8> {
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
