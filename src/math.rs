// note: a 256 * 256 mult table is a possible alternative
pub const EXP_TABLE: [u8; 255] = exp_table();
pub const LOG_TABLE: [u8; 256] = log_table();

/// 2 ^ x for 0 to 254
const fn exp_table() -> [u8; 255] {
    let mut array = [0; 255];
    array[0] = 1;
    let mut i = 1;
    while i < 255 {
        array[i] = array[i - 1] << 1;
        if array[i - 1] & 0b1000_0000 != 0 {
            // 2^4 + 2^3 + 2^2 + 2^0
            array[i] ^= 0b0001_1101;
        }
        i += 1;
    }
    array
}
/// log_2 of x for 1 to 255
const fn log_table() -> [u8; 256] {
    let mut array = [0; 256];
    let mut i = 1;
    while i < 256 {
        let mut j = 0;
        while j < 256 {
            if EXP_TABLE[j] == i as u8 {
                array[i] = j as u8;
                break;
            }
            j += 1;
        }
        i += 1;
    }
    array
}
