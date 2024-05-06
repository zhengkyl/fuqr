use crate::{
    math::{ANTILOG_TABLE, LOG_TABLE},
    qrcode::ECL,
};

pub const NUM_EC_CODEWORDS: [[u16; 41]; 4] = num_ec_codewords();

const fn num_ec_codewords() -> [[u16; 41]; 4] {
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

pub const NUM_BLOCKS: [[u8; 41]; 4] = num_blocks();

pub const fn num_blocks() -> [[u8; 41]; 4] {
    let mut table = [[0; 41]; 4];

    let mut ecl = 0;
    while ecl < NUM_EC_CODEWORDS.len() {
        let mut version = 1;

        while version <= 40 {
            let codewords = NUM_EC_CODEWORDS[ecl][version];

            let correctable = codewords / 2;
            if correctable <= 15 {
                table[ecl][version] = 1;
                version += 1;
                continue;
            }

            let mut per_block = 15;

            while per_block >= 8 {
                if correctable % per_block == 0 {
                    let mut blocks = correctable / per_block;
                    if blocks == 3 {
                        // Edgecase: there are never 3 blocks
                        blocks += 1;
                    }
                    table[ecl][version] = blocks as u8; // max is 81
                    break;
                }
                per_block -= 1;
            }

            version += 1;
        }

        ecl += 1;
    }

    // More edgecases
    table[ECL::Medium as usize][15] = 10;
    table[ECL::Medium as usize][19] = 14;
    table[ECL::Medium as usize][38] = 45;

    table
}

/// All generator polynomials for up to 30 error correction codewords.
/// The coefficients are stored as their exponent, starting from the second largest degree.
/// This EXCLUDES the coefficient of the largest degree, which is a^0.
pub const GEN_POLYNOMIALS: [[u8; 30]; 31] = gen_polynomials();

const fn gen_polynomials() -> [[u8; 30]; 31] {
    let mut table = [[0; 30]; 31];

    // In this loop, i is the number of error correcting codewords this polynomial is for
    // So, each loop multiplies the previous polynomial by x - a^(i-1)
    let mut i = 2;
    while i <= 30 {
        // Multiply prev last coefficent by a^(i-1)
        table[i][i - 1] = ((table[i - 1][i - 2] as usize + i - 1) % 255) as u8;

        let mut j = i - 2;
        while j > 0 {
            // Add like terms
            // coefficient of same power from previous polynomial (multiplied by a^i-1)
            let exp = ((table[i - 1][j - 1] as usize + i - 1) % 255) as u8;
            // coefficient of 1 lesser power from previous polynomial (multiplied by x)
            let coeff = LOG_TABLE[table[i - 1][j] as usize] ^ LOG_TABLE[exp as usize];
            table[i][j] = ANTILOG_TABLE[coeff as usize];
            j -= 1;
        }
        // Same logic as above, b/c first coefficient always 0
        let coeff = LOG_TABLE[table[i - 1][0] as usize] ^ LOG_TABLE[i - 1];
        table[i][0] = ANTILOG_TABLE[coeff as usize];

        i += 1;
    }
    table
}
