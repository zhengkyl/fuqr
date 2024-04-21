use std::cmp::min;

use ecc::{get_error_codewords, ECC_TABLE};
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
    let num_ecc_codewords = ECC_TABLE[qrcode.ecc as usize][qrcode.version.0 as usize] as usize;
    let num_data_codewords = qrcode.version.num_codewords() - num_ecc_codewords;

    // terminator
    let remaining_data_bits = (num_data_codewords * 8) - (qrcode.data.len());
    qrcode.push_bits(0, min(4, remaining_data_bits));

    // byte align
    let byte_pad = (8 - (qrcode.data.len() % 8)) % 8;
    qrcode.push_bits(0, byte_pad);

    let extra_codewords = num_data_codewords - (qrcode.data.len() / 8);

    let mut alternating_byte = 0b11101100;
    for _ in 0..extra_codewords {
        qrcode.push_bits(alternating_byte, 8);
        alternating_byte ^= 0b11111101;
    }

    get_error_codewords(&qrcode);

    qrcode
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }
