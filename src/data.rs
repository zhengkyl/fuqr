use crate::{
    constants::{NUM_DATA_MODULES, NUM_EC_CODEWORDS},
    encode::{bits_char_count_indicator, encode_alphanumeric, encode_byte, encode_numeric},
    qrcode::{Mode, Version, ECL},
};

pub struct Data {
    pub value: Vec<u8>,
    pub bit_len: usize,
    pub version: Version,
    pub ecl: ECL,
}

impl Data {
    pub fn new(segments: Vec<Segment>, version: Version, ecl: ECL) -> Option<Self> {
        let data = Data::strict_new(segments, version, ecl);

        let data = match data {
            Ok(x) => x,
            Err((segments, min_version)) if min_version <= 40 => {
                let ec_codewords = NUM_EC_CODEWORDS[ecl as usize];
                let data_codewords =
                    (NUM_DATA_MODULES[version.0] / 8) - ec_codewords[version.0] as usize;
                let mut data = Data {
                    value: Vec::with_capacity(data_codewords),
                    bit_len: 0,
                    version,
                    ecl,
                };
                encode(&mut data, segments);
                data
            }
            Err(_) => return None,
        };

        Some(data)
    }

    pub fn strict_new(
        segments: Vec<Segment>,
        version: Version,
        ecl: ECL,
    ) -> Result<Self, (Vec<Segment>, usize)> {
        let ec_codewords = NUM_EC_CODEWORDS[ecl as usize];
        let data_codewords = (NUM_DATA_MODULES[version.0] / 8) - ec_codewords[version.0] as usize;

        let mut bits = 0;
        for segment in &segments {
            bits += 4 + bits_char_count_indicator(version, segment.mode);
            let char_len = segment.text.len();
            match segment.mode {
                Mode::Numeric => {
                    bits += (char_len / 3) * 10;
                    // (char_len % 3) * 3 + 1 * (char_len % 3 + 1) / 2
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
        }

        if ((bits + 7) / 8) > data_codewords {
            let mut version = version.0 + 1;
            while version <= 40 {
                let codewords = (NUM_DATA_MODULES[version] / 8) - ec_codewords[version] as usize;
                for segment in &segments {
                    match segment.mode {
                        Mode::Byte => {
                            if version == 10 {
                                bits += 8;
                            }
                        }
                        _ => {
                            if version == 10 || version == 27 {
                                bits += 2;
                            }
                        }
                    }
                }
                if (bits + 7) / 8 <= codewords {
                    break;
                }
                version += 1;
            }

            return Err((segments, version));
        }

        let mut data = Data {
            value: Vec::with_capacity(data_codewords),
            bit_len: 0,
            version,
            ecl,
        };
        encode(&mut data, segments);
        Ok(data)
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

pub struct Segment<'a> {
    pub mode: Mode,
    pub text: &'a str, // max length is 7089 numeric, v40, low
}

fn encode(data: &mut Data, segments: Vec<Segment>) {
    for segment in segments {
        match segment.mode {
            Mode::Numeric => encode_numeric(data, segment.text),
            Mode::Alphanumeric => encode_alphanumeric(data, segment.text),
            Mode::Byte => encode_byte(data, segment.text),
        }
    }
}
