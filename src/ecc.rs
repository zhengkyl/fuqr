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
            &GEN_POLYNOMIALS[ecc_per_block][..ecc_per_block],
        ));
    }
    for i in 0..group_2_size {
        dbg!(remainder(
            &data_slice[(i * 8)..(i + data_per_block + 1)],
            &GEN_POLYNOMIALS[ecc_per_block][..ecc_per_block],
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
            base[i + j] ^= LOG_TABLE[(generator[j] as usize + alpha_diff as usize) % 255];
        }
    }

    // generator.len()
    // base[0..30].copy
    // base.copy_from_slice(src)
    base[data.len()..(data.len() + num_codewords)].to_vec()
}

const GEN_POLYNOMIALS: [[u8; 31]; 31] = make_polynomials();

pub const fn make_polynomials() -> [[u8; 31]; 31] {
    let mut table = [[0; 31]; 31];

    let mut i = 2;
    while i <= 30 {
        let mut j = i - 1;

        table[i][j + 1] = ((table[i - 1][j] as usize + i - 1) % 255) as u8;

        while j > 0 {
            let exp = ((table[i - 1][j - 1] as usize + i - 1) % 255) as u8;

            let coeff = LOG_TABLE[exp as usize] ^ LOG_TABLE[table[i - 1][j] as usize];
            table[i][j] = ANTILOG_TABLE[coeff as usize];
            j -= 1;
        }

        i += 1;
    }
    table
}
