use std::cmp::min;

use error_correction::{ECL, GEN_POLYNOMIALS, NUM_BLOCKS, NUM_CODEWORDS};
use math::{ANTILOG_TABLE, LOG_TABLE};
use qr::{encode_alphanumeric, QRCode};
use version::Version;

pub mod error_correction;
pub mod math;
pub mod qr;
pub mod version;

pub fn encode(input: &str) -> QRCode {
    let mut qrcode = QRCode {
        data: Vec::new(),
        ecl: ECL::Low,
        mask: 1,
        version: Version(1),
    };

    // todo, ensure version can contain before encode, mathable

    encode_alphanumeric(&mut qrcode, "GREETINGS TRAVELER");
    let modules = qrcode.version.num_data_modules();
    let codewords = modules / 8;
    let remainder_bits = modules % 8;

    let num_ec_codewords = NUM_CODEWORDS[qrcode.ecl as usize][qrcode.version.0 as usize] as usize;
    let num_data_codewords = codewords - num_ec_codewords;

    // terminator
    let remainder_data_bits = (num_data_codewords * 8) - (qrcode.data.len());
    qrcode.push_bits(0, min(4, remainder_data_bits));

    // byte align
    let byte_pad = (8 - (qrcode.data.len() % 8)) % 8;
    qrcode.push_bits(0, byte_pad);

    let data_pad = num_data_codewords - (qrcode.data.len() / 8);

    let mut alternating_byte = 0b11101100;
    for _ in 0..data_pad {
        qrcode.push_bits(alternating_byte, 8);
        alternating_byte ^= 0b11111101;
    }

    let blocks = NUM_BLOCKS[qrcode.ecl as usize][qrcode.version.0 as usize] as usize;

    let group_2_blocks = codewords % blocks;
    let group_1_blocks = blocks - group_2_blocks;

    let data_codeword_vec = qrcode.get_u8_data();
    let data_codewords = data_codeword_vec.as_slice();

    let data_per_g1_block = num_data_codewords / blocks;
    let data_per_g2_block = data_per_g1_block + 1;

    let ecc_per_block = num_ec_codewords / blocks;
    let mut final_sequence = vec![0; codewords];

    for i in 0..group_1_blocks * data_per_g1_block {
        let col = i % data_per_g1_block;
        let row = i / data_per_g1_block;
        final_sequence[col * blocks + row] = data_codewords[i];
    }
    for i in 0..group_2_blocks * data_per_g2_block {
        let col = i % data_per_g2_block;
        let row = i / data_per_g2_block;
        final_sequence[col * blocks + row + group_1_blocks] = data_codewords[i];
    }

    for i in 0..group_1_blocks {
        let ec_codewords = remainder(
            &data_codewords[(i * data_per_g1_block)..(i + data_per_g1_block)],
            &GEN_POLYNOMIALS[ecc_per_block][..ecc_per_block],
        );

        for j in 0..ec_codewords.len() {
            final_sequence[num_data_codewords + j * blocks + i] = ec_codewords[j];
        }
    }

    let group_2_start = group_1_blocks * data_per_g1_block;

    for i in 0..group_2_blocks {
        let ec_codewords = remainder(
            &data_codewords[(group_2_start + i * data_per_g2_block)..(i + data_per_g2_block)],
            &GEN_POLYNOMIALS[ecc_per_block][..ecc_per_block],
        );

        for j in 0..ec_codewords.len() {
            final_sequence[num_data_codewords + j * blocks + i + group_1_blocks] = ec_codewords[j];
        }
    }

    qrcode
}
fn remainder(data: &[u8], generator: &[u8]) -> Vec<u8> {
    let num_codewords = generator.len();

    let mut base = [0; 60];
    base[..data.len()].copy_from_slice(data);

    for i in 0..data.len() {
        if base[i] == 0 {
            continue;
        }

        let alpha_diff = ANTILOG_TABLE[data[i] as usize];

        for j in 0..generator.len() {
            base[i + j + 1] ^= LOG_TABLE[(generator[j] as usize + alpha_diff as usize) % 255];
        }
    }

    base[data.len()..(data.len() + num_codewords)].to_vec()
}
