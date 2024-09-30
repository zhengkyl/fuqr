use crate::{
    math::{ANTILOG_TABLE, LOG_TABLE},
    qr_code::ECL,
};

pub const NUM_DATA_MODULES: [u16; 41] = num_data_modules();
pub const NUM_EC_CODEWORDS: [[u16; 4]; 41] = num_ec_codewords();
pub const NUM_BLOCKS: [[u8; 4]; 41] = num_blocks();

/// All generator polynomials for up to 30 error correction codewords.
/// The coefficients are stored as their exponent, starting from the second largest degree.
/// This EXCLUDES the coefficient of the largest degree, which is a^0.
pub const GEN_POLYNOMIALS: [[u8; 30]; 31] = gen_polynomials();

pub const VERSION_INFO: [usize; 41] = version_info();
pub const FORMAT_INFO: [[u32; 8]; 4] = format_info();

const fn num_data_modules() -> [u16; 41] {
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

        table[version] = modules as u16;
        version += 1;
    }
    table
}

const fn num_ec_codewords() -> [[u16; 4]; 41] {
    let mut table = [[0; 4]; 41];
    table[1] = [7, 10, 13, 17];
    table[2] = [10, 16, 22, 28];
    table[3] = [15, 26, 36, 44];
    table[4] = [20, 36, 52, 64];
    table[5] = [26, 48, 72, 88];
    table[6] = [36, 64, 96, 112];
    table[7] = [40, 72, 108, 130];
    table[8] = [48, 88, 132, 156];
    table[9] = [60, 110, 160, 192];
    table[10] = [72, 130, 192, 224];
    table[11] = [80, 150, 224, 264];
    table[12] = [96, 176, 260, 308];
    table[13] = [104, 198, 288, 352];
    table[14] = [120, 216, 320, 384];
    table[15] = [132, 240, 360, 432];
    table[16] = [144, 280, 408, 480];
    table[17] = [168, 308, 448, 532];
    table[18] = [180, 338, 504, 588];
    table[19] = [196, 364, 546, 650];
    table[20] = [224, 416, 600, 700];
    table[21] = [224, 442, 644, 750];
    table[22] = [252, 476, 690, 816];
    table[23] = [270, 504, 750, 900];
    table[24] = [300, 560, 810, 960];
    table[25] = [312, 588, 870, 1050];
    table[26] = [336, 644, 952, 1110];
    table[27] = [360, 700, 1020, 1200];
    table[28] = [390, 728, 1050, 1260];
    table[29] = [420, 784, 1140, 1350];
    table[30] = [450, 812, 1200, 1440];
    table[31] = [480, 868, 1290, 1530];
    table[32] = [510, 924, 1350, 1620];
    table[33] = [540, 980, 1440, 1710];
    table[34] = [570, 1036, 1530, 1800];
    table[35] = [570, 1064, 1590, 1890];
    table[36] = [600, 1120, 1680, 1980];
    table[37] = [630, 1204, 1770, 2100];
    table[38] = [660, 1260, 1860, 2220];
    table[39] = [720, 1316, 1950, 2310];
    table[40] = [750, 1372, 2040, 2430];
    table
}

const fn num_blocks() -> [[u8; 4]; 41] {
    let mut table = [[0; 4]; 41];

    let mut version = 1;
    while version <= 40 {
        let mut ecl = 0;
        while ecl < 4 {
            let codewords = NUM_EC_CODEWORDS[version][ecl];

            let correctable = codewords / 2;
            if correctable <= 15 {
                table[version][ecl] = 1;
                ecl += 1;
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
                    table[version][ecl] = blocks as u8; // max is 81
                    break;
                }
                per_block -= 1;
            }

            ecl += 1;
        }

        version += 1;
    }

    // More edgecases
    table[15][ECL::Medium as usize] = 10;
    table[19][ECL::Medium as usize] = 14;
    table[38][ECL::Medium as usize] = 45;

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
    use crate::qr_code::Mask;

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
