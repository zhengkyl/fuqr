const fn generate_log_table() -> [u8; 256] {
    let mut array = [0; 256];
    array[0] = 1;
    // const for tracked here https://github.com/rust-lang/rust/issues/87575
    let mut i = 1;
    while i < 256 {
        array[i] = array[i - 1] << 1;
        if array[i - 1] & 0b1000_0000 != 0 {
            // 2^4 + 2^3 + 2^2 + 2^0
            array[i] ^= 0b0001_1101;
        }
        i += 1;
    }
    array
}

const fn generate_antilog_table() -> [u8; 256] {
    let mut array = [0; 256];
    let mut i = 1;
    while i < 256 {
        let mut j = 0;
        while j < 256 {
            if LOG_TABLE[j] == i as u8 {
                array[i] = j as u8;
                break;
            }
            j += 1;
        }
        i += 1;
    }
    array
}

pub const LOG_TABLE: [u8; 256] = generate_log_table();
pub const ANTILOG_TABLE: [u8; 256] = generate_antilog_table();
