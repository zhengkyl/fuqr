use crate::{ByteMode, Encoder, FuqrError, Mode, Version};

#[derive(Debug, Clone, Copy)]
pub struct NumericMode;
impl Mode for NumericMode {
    fn indicator(&self) -> u16 {
        0b0001
    }
    fn cci_len(&self, version: Version) -> u8 {
        10 + (version > 9) as u8 * 2 + (version > 26) as u8 * 2
    }
    // 10 bits per 3 digits, 4 or 7 bits for 1 or 2 leftover digits
    fn seg_len(&self, version: Version, len: usize) -> usize {
        4 + self.cci_len(version) as usize + (len * 10).div_ceil(3)
    }
    fn encode_utf8(&self, version: Version, bytes: &[u8], push: &mut dyn FnMut(u16, u8)) {
        push(self.indicator(), 4);
        push(bytes.len() as u16, self.cci_len(version));
        let mut groups = bytes.chunks_exact(3);
        for g in &mut groups {
            push(
                (g[0] - b'0') as u16 * 100 + (g[1] - b'0') as u16 * 10 + (g[2] - b'0') as u16,
                10,
            );
        }
        match *groups.remainder() {
            [a] => push((a - b'0') as u16, 4),
            [a, b] => push((a - b'0') as u16 * 10 + (b - b'0') as u16, 7),
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AlphanumericMode;
impl Mode for AlphanumericMode {
    fn indicator(&self) -> u16 {
        0b0010
    }
    fn cci_len(&self, version: Version) -> u8 {
        9 + (version > 9) as u8 * 2 + (version > 26) as u8 * 2
    }
    // 11 bits per 2 chars, 6 bits for 1 leftover char
    fn seg_len(&self, version: Version, len: usize) -> usize {
        4 + self.cci_len(version) as usize + (len * 11).div_ceil(2)
    }
    fn encode_utf8(&self, version: Version, bytes: &[u8], push: &mut dyn FnMut(u16, u8)) {
        push(self.indicator(), 4);
        push(bytes.len() as u16, self.cci_len(version));
        let mut pairs = bytes.chunks_exact(2);
        for p in &mut pairs {
            push(
                B45_LUT[p[0] as usize] as u16 * 45 + B45_LUT[p[1] as usize] as u16,
                11,
            );
        }
        if let [a] = *pairs.remainder() {
            push(B45_LUT[a as usize] as u16, 6);
        }
    }
}

#[derive(Debug)]
pub struct NumericEncoder<'a> {
    pub bytes: &'a [u8],
}
impl<'a> NumericEncoder<'a> {
    pub fn new(content: &'a str) -> Result<Self, FuqrError> {
        let bytes = content.as_bytes();
        if !bytes.iter().all(u8::is_ascii_digit) {
            return Err(FuqrError::EncodingFailure {
                message: "Content is not numeric",
            });
        }
        Ok(Self { bytes })
    }
}
impl<'a> Encoder for NumericEncoder<'a> {
    fn bit_len(&mut self, version: Version) -> usize {
        NumericMode.seg_len(version, self.bytes.len())
    }
    fn encode(&mut self, version: Version, push: &mut dyn FnMut(u16, u8)) {
        NumericMode.encode_utf8(version, self.bytes, push);
    }
}

/// Alphanumeric value of each byte, or 255 if not alphanumeric.
pub const B45_LUT: [u8; 256] = {
    let chars = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";
    let mut table = [255; 256];
    let mut i = 0;
    while i < chars.len() {
        table[chars[i] as usize] = i as u8;
        i += 1;
    }
    table
};

#[derive(Debug)]
pub struct AlphanumericEncoder<'a> {
    pub bytes: &'a [u8],
}
impl<'a> AlphanumericEncoder<'a> {
    pub fn new(content: &'a str) -> Result<Self, FuqrError> {
        let bytes = content.as_bytes();
        if bytes.iter().any(|&b| B45_LUT[b as usize] == 255) {
            return Err(FuqrError::EncodingFailure {
                message: "Content is not alphanumeric",
            });
        }
        Ok(Self { bytes })
    }
}
impl<'a> Encoder for AlphanumericEncoder<'a> {
    fn bit_len(&mut self, version: Version) -> usize {
        AlphanumericMode.seg_len(version, self.bytes.len())
    }
    fn encode(&mut self, version: Version, push: &mut dyn FnMut(u16, u8)) {
        AlphanumericMode.encode_utf8(version, self.bytes, push);
    }
}

const MODES: [&dyn Mode; 3] = [&NumericMode, &AlphanumericMode, &ByteMode];

// Cheapest mode each byte fits in: 0 numeric, 1 alphanumeric, 2 byte
const MODE: [u8; 256] = {
    let mut table = [2; 256];
    let mut i = 0;
    while i < 256 {
        if B45_LUT[i] < 10 {
            table[i] = 0;
        } else if B45_LUT[i] != 255 {
            table[i] = 1;
        }
        i += 1;
    }
    table
};

// Open segment states: numeric with length 1, 2, 0 mod 3, alphanumeric with
// length 1, 0 mod 2, then byte. Each continues the one before it in its mode.
const STATE_MODE: [u8; 6] = [0, 0, 0, 1, 1, 2];
const PREV: [usize; 6] = [2, 0, 1, 4, 3, 5];
const CHAR_BITS: [usize; 6] = [4, 3, 3, 6, 5, 8];
// Only a mode's first state opens a segment
const FIRST: [usize; 3] = [0, 3, 5];

const UNKNOWN: usize = usize::MAX;
const INF: usize = usize::MAX / 4;

/// Encodes content in the shortest mix of numeric, alphanumeric and byte segments.
///
/// `scratch` must be at least as long as the content in UTF-8 bytes. It holds
/// the chosen mode of each byte.
#[derive(Debug)]
pub struct MixedEncoder<'a> {
    pub bytes: &'a [u8],
    modes: &'a mut [u8],
    // Segmentation only depends on char count indicator lengths, so there are
    // only 3 distinct results, one per cci_group
    bits: [usize; 3],
    // Which group's segmentation `modes` currently holds
    filled: usize,
}
impl<'a> MixedEncoder<'a> {
    pub fn new(content: &'a str, scratch: &'a mut [u8]) -> Self {
        let bytes = content.as_bytes();
        assert!(
            scratch.len() >= bytes.len(),
            "scratch buffer shorter than content"
        );
        Self {
            bytes,
            modes: &mut scratch[..bytes.len()],
            bits: [UNKNOWN; 3],
            filled: UNKNOWN,
        }
    }

    // Viterbi over open segment states, exact since a state fixes the bits of the
    // next char. Fewest bits win, then fewest segments, then the earliest start.
    fn segment(&mut self, version: Version) -> usize {
        let bytes = self.bytes;
        let trace = &mut *self.modes;
        let n = bytes.len();

        if n == 0 {
            return ByteMode.seg_len(version, 0);
        }

        let headers = MODES.map(|m| 4 + m.cci_len(version) as usize);
        // Best path into each state as (bits, segment count, segment start)
        let mut paths = [(INF, 0, 0); 6];
        let mut closed = (0, 0);

        // Low 3 bits are the best state at i, then 1 bit per mode if it opened at i
        for i in 0..n {
            let byte_mode = MODE[bytes[i] as usize];
            let prev = paths;
            let mut t = 0;
            let mut best = 0;
            for k in 0..6 {
                let mode = STATE_MODE[k];
                let (bits, count, start) = prev[PREV[k]];
                let bits = if byte_mode > mode {
                    INF
                } else {
                    bits + CHAR_BITS[k]
                };
                paths[k] = (bits, count, start);

                // Continuing starts earlier, so opening must be strictly better
                let open = (
                    closed.0 + headers[mode as usize] + CHAR_BITS[k],
                    closed.1 + 1,
                    i,
                );
                if k == FIRST[mode as usize] && byte_mode <= mode && open < paths[k] {
                    paths[k] = open;
                    t |= 8 << mode;
                }

                if paths[k] < paths[best] {
                    best = k;
                }
            }
            trace[i] = t | best as u8;
            closed = (paths[best].0, paths[best].1);
        }

        // Replace the trace with the chosen mode of each byte
        let mut k = (trace[n - 1] & 7) as usize;
        for i in (1..n).rev() {
            let mode = STATE_MODE[k];
            let opened = k == FIRST[mode as usize] && trace[i] & (8 << mode) != 0;
            trace[i] = mode;
            k = if opened {
                (trace[i - 1] & 7) as usize
            } else {
                PREV[k]
            };
        }
        trace[0] = STATE_MODE[k];

        closed.0
    }
}
impl<'a> Encoder for MixedEncoder<'a> {
    fn bit_len(&mut self, version: Version) -> usize {
        let group = (version > 9) as usize + (version > 26) as usize;
        if self.bits[group] == UNKNOWN {
            self.bits[group] = self.segment(version);
            self.filled = group;
        }
        self.bits[group]
    }

    fn encode(&mut self, version: Version, push: &mut dyn FnMut(u16, u8)) {
        let group = (version > 9) as usize + (version > 26) as usize;
        if self.filled != group {
            self.bits[group] = self.segment(version);
            self.filled = group;
        }

        let bytes = self.bytes;
        let modes = &*self.modes;
        let n = bytes.len();

        if n == 0 {
            ByteMode.encode_utf8(version, &[], push);
            return;
        }

        let mut start = 0;
        while start < n {
            let mode = modes[start];
            let mut end = start + 1;
            while end < n && modes[end] == mode {
                end += 1;
            }
            MODES[mode as usize].encode_utf8(version, &bytes[start..end], push);
            start = end;
        }
    }
}
