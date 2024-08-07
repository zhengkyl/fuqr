use crate::{
    constants::{NUM_DATA_MODULES, NUM_EC_CODEWORDS},
    encoding::{bits_char_count_indicator, encode_alphanumeric, encode_byte, encode_numeric},
    qrcode::{Mode, Version, ECL},
};

pub struct Data {
    pub value: Vec<u8>,
    pub bit_len: usize,
    pub mode: Mode,
    pub version: Version,
    pub ecl: ECL,
}

impl Data {
    pub fn new(text: &str, mode: Mode, min_version: Version, min_ecl: ECL) -> Option<Self> {
        Self::new_verbose(text, mode, min_version, false, min_ecl, false)
    }

    pub fn new_verbose(
        text: &str,
        mode: Mode,
        min_version: Version,
        strict_version: bool,
        min_ecl: ECL,
        strict_ecl: bool,
    ) -> Option<Self> {
        let mut bits = 0;
        bits += 4 + bits_char_count_indicator(min_version, mode);
        let char_len = text.len();
        match mode {
            Mode::Numeric => {
                bits += (char_len / 3) * 10;
                match char_len % 3 {
                    2 => bits += 7,
                    1 => bits += 4,
                    _ => (),
                }
            }
            Mode::Alphanumeric => {
                bits += (char_len / 2) * 11 + (char_len % 2) * 6;
            }
            Mode::Byte => {
                bits += char_len * 8;
            }
        }

        let mut data_codewords = (NUM_DATA_MODULES[min_version.0] / 8) as usize;

        let mut min_version = min_version.0;
        let mut req_codewords = (bits + 7) / 8;

        while req_codewords
            > (data_codewords - NUM_EC_CODEWORDS[min_version][min_ecl as usize] as usize)
            && min_version < 40
        {
            if strict_version {
                return None
            }

            min_version += 1;

            data_codewords = (NUM_DATA_MODULES[min_version] / 8) as usize;
            // char count indicator length increase
            match mode {
                Mode::Byte => {
                    if min_version == 10 {
                        bits += 8;
                    }
                }
                _ => {
                    if min_version == 10 || min_version == 27 {
                        bits += 2;
                    }
                }
            }
            req_codewords = (bits + 7) / 8;
        }

        if min_version > 40 {
            return None;
        }

        let mut max_ecl = min_ecl;

        if !strict_ecl {
            // this is literally to avoid having to impl TryFrom<usize> for ECL
            let ecls = [ECL::Low, ECL::Medium, ECL::Quartile, ECL::High];
            for new_ecl in (min_ecl as usize + 1..ecls.len()).rev() {
                if req_codewords <= data_codewords - NUM_EC_CODEWORDS[min_version][new_ecl] as usize {
                    max_ecl = ecls[new_ecl];
                    break;
                }
            }
        }

        let mut data = Data {
            value: Vec::with_capacity(data_codewords),
            bit_len: 0,
            mode,
            version: Version(min_version),
            ecl: max_ecl,
        };

        match mode {
            Mode::Numeric => encode_numeric(&mut data, text),
            Mode::Alphanumeric => encode_alphanumeric(&mut data, text),
            Mode::Byte => encode_byte(&mut data, text),
        }
        Some(data)
    }

    pub fn push_bits(&mut self, input: usize, len: usize) {
        let gap = (8 - (self.bit_len % 8)) % 8;
        self.bit_len += len;

        if gap >= len {
            let i = self.value.len() - 1;
            self.value[i] |= (input << (gap - len)) as u8;
            return;
        }

        let mut len = len - gap;
        if gap > 0 {
            let i = self.value.len() - 1;
            self.value[i] |= (input >> len) as u8;
        }

        while len >= 8 {
            len -= 8;
            self.value.push((input >> len) as u8);
        }

        if len > 0 {
            self.value.push((input << (8 - len)) as u8);
        }
    }
}
