use crate::{
    math::{ANTILOG_TABLE, LOG_TABLE},
    qr::{QRCode, ECL},
};

const fn make_ecc_table() -> [[u16; 41]; 4] {
    let mut table = [[0; 41]; 4];
    table[ECL::Low as usize] = [
        0, 7, 10, 15, 20, 26, 36, 40, 48, 60, 72, 80, 96, 104, 120, 132, 144, 168, 180, 196, 224,
        224, 252, 270, 300, 312, 336, 360, 390, 420, 450, 480, 510, 540, 570, 570, 600, 630, 660,
        720, 750,
    ];
    table[ECL::Medium as usize] = [
        0, 10, 16, 26, 36, 48, 64, 72, 88, 110, 130, 150, 176, 198, 216, 240, 280, 308, 338, 364,
        416, 442, 476, 504, 560, 588, 644, 700, 728, 784, 812, 868, 924, 980, 1036, 1064, 1120,
        1204, 1260, 1316, 1372,
    ];
    table[ECL::Quartile as usize] = [
        0, 13, 22, 36, 52, 72, 96, 108, 132, 160, 192, 224, 260, 288, 320, 360, 408, 448, 504, 546,
        600, 644, 690, 750, 810, 870, 952, 1020, 1050, 1140, 1200, 1290, 1350, 1440, 1530, 1590,
        1680, 1770, 1860, 1950, 2040,
    ];
    table[ECL::High as usize] = [
        0, 17, 28, 44, 64, 88, 112, 130, 156, 192, 224, 264, 308, 352, 384, 432, 480, 532, 588,
        650, 700, 750, 816, 900, 960, 1050, 1110, 1200, 1260, 1350, 1440, 1530, 1620, 1710, 1800,
        1890, 1980, 2100, 2220, 2310, 2430,
    ];
    table
}
pub const ECC_TABLE: [[u16; 41]; 4] = make_ecc_table();

// todo, turn into const or static table
pub fn num_blocks(qrcode: &QRCode) -> u32 {
    if qrcode.ecc == ECL::Medium {
        match qrcode.version.0 {
            15 => return 10,
            19 => return 14,
            38 => return 45,
            _ => (),
        }
    }

    let codewords = ECC_TABLE[qrcode.ecc as usize][qrcode.version.0 as usize] as u32;

    let errors = codewords / 2;
    if errors <= 15 {
        return 1;
    }
    for i in (8..=15).rev() {
        if errors % i == 0 {
            let res = errors / i;
            if res == 3 {
                return 4;
            }
            return res;
        }
    }

    unreachable!("num blocks not found");
}

pub fn get_error_codewords(qrcode: &QRCode) {
    let blocks = num_blocks(qrcode);
    let codewords = qrcode.version.num_codewords();

    // todo, duplicated in caller
    let num_ecc_codewords = ECC_TABLE[qrcode.ecc as usize][qrcode.version.0 as usize] as usize;
    let num_data_codewords = codewords - num_ecc_codewords;

    let group_2_size = codewords % blocks as usize;
    let group_1_size = blocks as usize - group_2_size;

    let binding = qrcode.get_u8_data();
    let data_slice = binding.as_slice();

    let data_per_block = num_data_codewords / blocks as usize;

    // todo choose gen polynomial based on ecc_per_block
    let ecc_per_block = num_ecc_codewords / blocks as usize;

    for i in 0..group_1_size {
        dbg!(remainder(
            &data_slice[(i * 8)..(i + data_per_block)],
            &GEN_7,
        ));
    }
    for i in 0..group_2_size {
        dbg!(remainder(
            &data_slice[(i * 8)..(i + data_per_block + 1)],
            &GEN_7,
        ));
    }
}

fn remainder(data: &[u8], generator: &[u8]) -> Vec<u8> {
    let num_codewords = generator.len() - 1;

    let mut base = [0; 60];
    base[..data.len()].copy_from_slice(data);

    for i in 0..data.len() {
        if base[i] == 0 {
            continue;
        }

        let alpha_diff = ANTILOG_TABLE[data[i] as usize];

        for j in 1..generator.len() {
            base[i + j] ^= LOG_TABLE[generator[j].wrapping_add(alpha_diff) as usize];
        }
    }

    // generator.len()
    // base[0..30].copy
    // base.copy_from_slice(src)
    base[data.len()..(data.len() + num_codewords)].to_vec()
}

// const fn make_generator_polynomials() {
//     let mut base  = [0; 31];
//     base[0] = 1;
//     base[1] = 1;

//     for i in 1..=30 {
//         let mult =
//     }
// }

// temp
pub const GEN_7: [u8; 8] = [0, 87, 229, 146, 149, 238, 102, 21];

// for ref, replace with generator fn
// const GEN_10: [u8; 11] = [0, 251, 67, 46, 61, 118, 70, 64, 94, 32, 45];

// const GEN_13: [u8; 14] = [
//     0, 74, 152, 176, 100, 86, 100, 106, 104, 130, 218, 206, 140, 78,
// ];
// const GEN_16: [i32; 17] = [
//     0, 120, 104, 107, 109, 102, 161, 76, 3, 91, 191, 147, 169, 182, 194, 225, 120,
// ];
// const GEN_17: [i32; 18] = [
//     0, 43, 139, 206, 78, 43, 239, 123, 206, 214, 147, 24, 99, 150, 39, 243, 163, 136,
// ];
// const GEN_15: [i32; 16] = [
//     0, 8, 183, 61, 91, 202, 37, 51, 58, 58, 237, 140, 124, 5, 99, 105,
// ];
