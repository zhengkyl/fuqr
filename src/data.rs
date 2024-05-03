use crate::{
    error_correction::{ECL, NUM_CODEWORDS},
    version::Version,
};

pub struct QRData {
    pub version: Version,
    pub data: Vec<u8>,
    bit_len: usize,
}

impl QRData {
    pub fn new(version: Version) -> Self {
        let data_size = version.num_data_modules()
            - NUM_CODEWORDS[ECL::Low as usize][version.0 as usize] as usize;

        QRData {
            version: version,
            data: Vec::with_capacity(data_size),
            bit_len: 0,
        }
    }
    pub fn push_bits(&mut self, input: usize, len: usize) {
        let gap = (8 - (self.bit_len % 8)) % 8;
        self.bit_len += len;

        if gap >= len {
            let i = self.data.len() - 1;
            self.data[i] |= (input << (gap - len)) as u8;
            self.bit_len += len;
            return;
        }

        let mut len = len - gap;
        if gap > 0 {
            let i = self.data.len() - 1;
            self.data[i] |= (input >> len) as u8;
        }

        while len >= 8 {
            len -= 8;
            self.data.push((input >> len) as u8);
        }

        if len > 0 {
            self.data.push((input << (8 - len)) as u8);
        }
    }
}
