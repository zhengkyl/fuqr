use crate::{
    error_correction::NUM_EC_CODEWORDS,
    qrcode::{Version, ECL},
};

pub struct QRData {
    pub version: Version,
    pub data: Vec<u8>,
    pub bit_len: usize,
}

impl QRData {
    pub fn new(version: Version) -> Self {
        let data_size = (NUM_DATA_MODULES[version.0] / 8)
            - NUM_EC_CODEWORDS[ECL::Low as usize][version.0] as usize;

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

pub const NUM_DATA_MODULES: [usize; 41] = num_data_modules();

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
