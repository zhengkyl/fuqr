use crate::{
    math::{ANTILOG_TABLE, LOG_TABLE},
    qrcode::ECL,
};

pub const NUM_DATA_MODULES: [usize; 41] = num_data_modules();
pub const NUM_EC_CODEWORDS: [[u16; 41]; 4] = num_ec_codewords();
pub const NUM_BLOCKS: [[u8; 41]; 4] = num_blocks();

/// All generator polynomials for up to 30 error correction codewords.
/// The coefficients are stored as their exponent, starting from the second largest degree.
/// This EXCLUDES the coefficient of the largest degree, which is a^0.
pub const GEN_POLYNOMIALS: [[u8; 30]; 31] = gen_polynomials();

pub const VERSION_INFO: [usize; 41] = version_info();
pub const FORMAT_INFO: [[u32; 8]; 4] = format_info();

const fn num_data_modules() -> [usize; 41] {
    let mut table = [0; 41];

    let mut version = 1;
    while version <= 40 {
        let width = 4 * version + 17;
        let mut modules = width * width;

        modules -= 64 * 3; // finder markers + separator
        modules -= 31; // format
        modules -= 2 * (width - 16); // timing

        let (align, overlap) = match version {
            1 => (0, 0),
            x if x <= 6 => (1, 0),
            x if x <= 13 => (6, 2),
            x if x <= 20 => (13, 4),
            x if x <= 27 => (22, 6),
            x if x <= 34 => (33, 8),
            x if x <= 40 => (46, 10),
            _ => unreachable!(),
        };
        modules -= align * 25;
        modules += overlap * 5;

        if version >= 7 {
            modules -= 36; // 2 version
        }

        table[version] = modules;
        version += 1;
    }
    table
}

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

const fn num_blocks() -> [[u8; 41]; 4] {
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

const fn version_info() -> [usize; 41] {
    let mut array = [0; 41];

    let mut version = 1;
    while version <= 40 {
        let shifted_version = version << 12;
        let mut dividend: usize = shifted_version;

        while dividend >= 0b1_0000_0000_0000 {
            let mut divisor = 0b1_1111_0010_0101;
            divisor <<= (usize::BITS - dividend.leading_zeros()) - 13; // diff of highest set bit

            dividend ^= divisor;
        }
        array[version] = shifted_version | dividend;
        version += 1;
    }
    array
}

const fn format_info() -> [[u32; 8]; 4] {
    let mut array = [[0; 8]; 4];

    let mut i = 0;
    let ecls = [ECL::Low, ECL::Medium, ECL::Quartile, ECL::High];
    while i < 4 {
        let ecl = ecls[i];
        let value = match ecl {
            ECL::Low => 1,
            ECL::Medium => 0,
            ECL::Quartile => 3,
            ECL::High => 2,
        };

        let mut mask = 0;
        while mask < 8 {
            let format = ((((value) << 3) | mask as u8) as u32) << 10;
            let mut dividend = format;

            while dividend >= 0b100_0000_0000 {
                let mut divisor = 0b101_0011_0111;
                divisor <<= (32 - dividend.leading_zeros()) - 11;

                dividend ^= divisor;
            }

            array[i][mask] = (format | dividend) ^ 0b10101_0000010010;
            mask += 1;
        }

        i += 1;
    }

    array
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qrcode::Mask;

    #[test]
    fn information_works() {
        assert_eq!(VERSION_INFO[7], 0x07C94);
        assert_eq!(VERSION_INFO[21], 0x15683);
        assert_eq!(VERSION_INFO[40], 0x28C69);
    }

    #[test]
    fn format_information_works() {
        assert_eq!(FORMAT_INFO[ECL::Medium as usize][Mask::M0 as usize], 0x5412);
        assert_eq!(FORMAT_INFO[ECL::High as usize][Mask::M0 as usize], 0x1689);
        assert_eq!(FORMAT_INFO[ECL::High as usize][Mask::M7 as usize], 0x083B);
    }
}
