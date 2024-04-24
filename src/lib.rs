use std::cmp::min;

use ecc::{num_blocks, ECC_TABLE, GEN_POLYNOMIALS};
use math::{ANTILOG_TABLE, LOG_TABLE};
use qr::{encode_alphanumeric, QRCode, ECL};
use version::Version;

pub mod ecc;
pub mod math;
pub mod qr;
pub mod version;

pub fn encode(input: &str) -> QRCode {
    let mut qrcode = QRCode {
        data: Vec::new(),
        ecc: ECL::Low,
        mask: 1,
        version: Version(1),
    };

    // todo, ensure version can contain before encode, mathable

    encode_alphanumeric(&mut qrcode, "GREETINGS TRAVELER");
    let modules = qrcode.version.num_data_modules();
    let codewords = modules / 8;
    let remainder_bits = modules % 8;

    let ec_codewords = ECC_TABLE[qrcode.ecc as usize][qrcode.version.0 as usize] as usize;
    let data_codewords = codewords - ec_codewords;

    // terminator
    let remainder_data_bits = (data_codewords * 8) - (qrcode.data.len());
    qrcode.push_bits(0, min(4, remainder_data_bits));

    // byte align
    let byte_pad = (8 - (qrcode.data.len() % 8)) % 8;
    qrcode.push_bits(0, byte_pad);

    let data_pad = data_codewords - (qrcode.data.len() / 8);

    let mut alternating_byte = 0b11101100;
    for _ in 0..data_pad {
        qrcode.push_bits(alternating_byte, 8);
        alternating_byte ^= 0b11111101;
    }

    let blocks = num_blocks(&qrcode) as usize;

    let num_group_2 = codewords % blocks;
    let num_group_1 = blocks - num_group_2;

    let binding = qrcode.get_u8_data();
    let data_slice = binding.as_slice();

    let data_per_block = data_codewords / blocks;

    let ecc_per_block = ec_codewords / blocks;

    let mut final_sequence = vec![0; codewords];

    for i in 0..num_group_1 {
        let ec_codewords = remainder(
            &data_slice[(i * data_per_block)..(i + data_per_block)],
            &GEN_POLYNOMIALS[ecc_per_block][..ecc_per_block],
        );

        for j in 0..ec_codewords.len() {
            final_sequence[data_codewords + j * blocks + i] = ec_codewords[j];
        }
    }

    let group_2_start = num_group_1 * data_per_block;
    let data_per_block = data_per_block + 1;
    for i in 0..num_group_2 {
        let ec_codewords = remainder(
            &data_slice[(group_2_start + i * data_per_block)..(i + data_per_block)],
            &GEN_POLYNOMIALS[ecc_per_block][..ecc_per_block],
        );

        for j in 0..ec_codewords.len() {
            final_sequence[data_codewords + j * blocks + i + num_group_1] = ec_codewords[j];
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
