use crate::{
    constants::{NUM_DATA_MODULES, NUM_EC_CODEWORDS},
    encoding::{encode_alphanumeric, encode_byte, encode_numeric, num_cci_bits},
    qr_code::{Mode, Version, ECL},
};

#[derive(Debug)]
pub struct Data {
    pub bits: BitVec,
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
        bits += 4 + num_cci_bits(min_version, mode);
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
                return None;
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
            let ecls = [ECL::Low, ECL::Medium, ECL::Quartile, ECL::High];
            for new_ecl in (min_ecl as usize + 1..ecls.len()).rev() {
                if req_codewords <= data_codewords - NUM_EC_CODEWORDS[min_version][new_ecl] as usize
                {
                    max_ecl = ecls[new_ecl];
                    break;
                }
            }
        }

        let mut data = Data {
            bits: BitVec::with_capacity(data_codewords * 8),
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
}

#[derive(Debug, PartialEq, Eq)]
pub struct BitVec {
    value: Vec<u8>,
    len: usize,
}

impl BitVec {
    pub fn new() -> Self {
        BitVec {
            value: Vec::new(),
            len: 0,
        }
    }
    pub fn with_capacity(capacity: usize) -> Self {
        BitVec {
            value: Vec::with_capacity((capacity + 7) / 8),
            len: 0,
        }
    }
    pub fn resize(&mut self, new_len: usize) {
        self.value.resize((new_len + 7) / 8, 0);
        self.len = new_len;
    }
    /// self must be byte aligned
    pub fn append(&mut self, other: &mut Vec<u8>) {
        self.value.append(other);
        self.len += other.len() * 8;
    }
    pub fn to_bytes(self) -> Vec<u8> {
        self.value
    }
    pub fn set(&mut self, i: usize) {
        self.value[i / 8] = 1 << (7 - (i % 8));
    }
    pub fn get(&self, i: usize) -> bool {
        ((self.value[i / 8] >> (7 - (i % 8))) & 1) == 1
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn push_n(&mut self, input: usize, n: usize) {
        let gap = (8 - (self.len % 8)) % 8;
        self.len += n;

        if gap >= n {
            let i = self.value.len() - 1;
            self.value[i] |= (input << (gap - n)) as u8;
            return;
        }

        let mut n = n - gap;
        if gap > 0 {
            let i = self.value.len() - 1;
            self.value[i] |= (input >> n) as u8;
        }

        while n >= 8 {
            n -= 8;
            self.value.push((input >> n) as u8);
        }

        if n > 0 {
            self.value.push((input << (8 - n)) as u8);
        }
    }
}

impl From<Vec<u8>> for BitVec {
    fn from(value: Vec<u8>) -> Self {
        BitVec {
            len: value.len() * 8,
            value,
        }
    }
}

impl AsRef<[u8]> for BitVec {
    fn as_ref(&self) -> &[u8] {
        &self.value
    }
}

impl AsMut<[u8]> for BitVec {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.value
    }
}
