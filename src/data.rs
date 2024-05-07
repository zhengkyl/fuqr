use crate::{
    encode::{encode_alphanumeric, encode_byte, encode_numeric},
    error_correction::NUM_EC_CODEWORDS,
    qrcode::{Mode, Version, ECL, NUM_DATA_MODULES},
};

pub struct Data {
    pub value: Vec<u8>,
    pub bit_len: usize,
    pub version: Version,
}

impl Data {
    pub fn new(segments: Vec<Segment>, version: Version) -> Self {
        let data_size = (NUM_DATA_MODULES[version.0] / 8)
            - NUM_EC_CODEWORDS[ECL::Low as usize][version.0] as usize;

        let mut data = Data {
            version,
            value: Vec::with_capacity(data_size),
            bit_len: 0,
        };
        encode(&mut data, segments);

        data
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
    // let min_max_bits = (NUM_DATA_MODULES[version.0])
    //     - (NUM_EC_CODEWORDS[ECL::Low as usize][version.0] as usize * 8);

    let max_max_bits =
        (NUM_DATA_MODULES[40]) - (NUM_EC_CODEWORDS[ECL::Low as usize][40] as usize * 8);

    // TODO iff we cross a version with a diff header size, must recalculate everything?

    // todo, ensure version can contain before encode, mathable
    for segment in segments {
        match segment.mode {
            Mode::Numeric => encode_numeric(data, segment.text),
            Mode::Alphanumeric => encode_alphanumeric(data, segment.text),
            Mode::Byte => encode_byte(data, segment.text),
        }
        if data.bit_len > max_max_bits {
            todo!();
        }
    }
}
