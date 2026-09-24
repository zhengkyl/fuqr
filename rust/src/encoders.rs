use crate::{ByteMode, Encoder, FuqrError, Mode, Version};

const fn cci_diff(version: Version) -> u8 {
    (version > 9) as u8 * 2 + (version > 26) as u8 * 2
}

#[derive(Debug, Clone, Copy)]
pub struct NumericMode;
impl Mode for NumericMode {
    fn indicator(&self) -> u16 {
        0b0001
    }
    fn cci_len(&self, version: Version) -> u8 {
        10 + cci_diff(version)
    }
    // 10 bits per 3 digits, 4 or 7 bits for 1 or 2 leftover digits
    fn seg_len(&self, len: usize, version: Version) -> usize {
        4 + self.cci_len(version) as usize + (len * 10).div_ceil(3)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AlphanumericMode;
impl Mode for AlphanumericMode {
    fn indicator(&self) -> u16 {
        0b0010
    }
    fn cci_len(&self, version: Version) -> u8 {
        9 + cci_diff(version)
    }
    // 11 bits per 2 chars, 6 bits for 1 leftover char
    fn seg_len(&self, len: usize, version: Version) -> usize {
        4 + self.cci_len(version) as usize + (len * 11).div_ceil(2)
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
            return Err(FuqrError::InvalidEncoding {
                message: "Content is not numeric",
            });
        }
        Ok(Self { bytes })
    }
}
impl<'a> Encoder for NumericEncoder<'a> {
    fn bit_len(&mut self, version: Version) -> usize {
        NumericMode.seg_len(self.bytes.len(), version)
    }
    fn encode(&mut self, version: Version, push: &mut dyn FnMut(u16, u8)) {
        let bytes = self.bytes;

        push(NumericMode.indicator(), 4);
        push(bytes.len() as u16, NumericMode.cci_len(version));
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

// Alphanumeric value of each byte, or 255 if not alphanumeric
const B45: [u8; 256] = {
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
        if bytes.iter().any(|&b| B45[b as usize] == 255) {
            return Err(FuqrError::InvalidEncoding {
                message: "Content is not alphanumeric",
            });
        }
        Ok(Self { bytes })
    }
    pub fn byte_to_b45(c: u8) -> u8 {
        B45[c as usize]
    }
}
impl<'a> Encoder for AlphanumericEncoder<'a> {
    fn bit_len(&mut self, version: Version) -> usize {
        AlphanumericMode.seg_len(self.bytes.len(), version)
    }
    fn encode(&mut self, version: Version, push: &mut dyn FnMut(u16, u8)) {
        let bytes = self.bytes;

        push(AlphanumericMode.indicator(), 4);
        push(bytes.len() as u16, AlphanumericMode.cci_len(version));
        let mut pairs = bytes.chunks_exact(2);
        for p in &mut pairs {
            push(
                B45[p[0] as usize] as u16 * 45 + B45[p[1] as usize] as u16,
                11,
            );
        }
        if let [a] = *pairs.remainder() {
            push(B45[a as usize] as u16, 6);
        }
    }
}

// Cheapest mode each byte fits in: 0 numeric, 1 alphanumeric, 2 byte
// All multibyte UTF-8 bytes look like 1xxx_xxxx, so they are always 2
const MODE: [u8; 256] = {
    let mut table = [2; 256];
    let mut i = 0;
    while i < 256 {
        if B45[i] < 10 {
            table[i] = 0;
        } else if B45[i] != 255 {
            table[i] = 1;
        }
        i += 1;
    }
    table
};

fn seg_len(mode: u8, len: usize, version: Version) -> usize {
    match mode {
        0 => NumericMode.seg_len(len, version),
        1 => AlphanumericMode.seg_len(len, version),
        _ => ByteMode.seg_len(len, version),
    }
}

// Versions 1-9, 10-26, 27-40 have different char count indicator lengths
fn cci_group(version: Version) -> usize {
    (version > 9) as usize + (version > 26) as usize
}

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

    fn fill(&mut self, version: Version) {
        let group = cci_group(version);
        if self.filled != group {
            self.bits[group] = self.segment(version);
            self.filled = group;
        }
    }

    // Costs are in sixths of a bit so each char has a fixed cost:
    // numeric 20, alphanumeric 33, byte 48. Segments round up when closed.
    fn segment(&mut self, version: Version) -> usize {
        let bytes = self.bytes;
        let modes = &mut *self.modes;
        let n = bytes.len();

        if n == 0 {
            return ByteMode.seg_len(0, version);
        }

        // One mode throughout is optimal as a single segment
        let first = MODE[bytes[0] as usize];
        if bytes.iter().all(|&b| MODE[b as usize] == first) {
            modes.fill(first);
            return seg_len(first, n, version);
        }

        // header + first char
        let start0 = (4 + NumericMode.cci_len(version) as usize) * 6 + 20;
        let start1 = (4 + AlphanumericMode.cci_len(version) as usize) * 6 + 33;
        let start2 = (4 + ByteMode.cci_len(version) as usize) * 6 + 48;

        let mut c0 = if first == 0 { start0 } else { INF };
        let mut c1 = if first <= 1 { start1 } else { INF };
        let mut c2 = start2;

        // modes[i] packs the mode at i - 1 for each mode at i, 2 bits per mode
        for i in 1..n {
            let mode = MODE[bytes[i] as usize];

            // Only byte mode is possible, so nothing to compare
            if mode == 2 && c0 == INF && c1 == INF {
                c2 += 48;
                modes[i] = 2 << 4;
                continue;
            }

            // Cost of closing a segment in each mode
            let r0 = c0.div_ceil(6) * 6;
            let r1 = c1.div_ceil(6) * 6;
            let r2 = c2.div_ceil(6) * 6;
            let mut p = 0;

            // Staying wins ties to avoid pointless segments
            if mode == 0 {
                let (from, r) = if r1 <= r2 { (1, r1) } else { (2, r2) };
                let stay = c0 + 20;
                if stay <= r + start0 {
                    c0 = stay;
                } else {
                    c0 = r + start0;
                    p = from;
                }
            } else {
                c0 = INF;
            }

            if mode <= 1 {
                let (from, r) = if r0 <= r2 { (0, r0) } else { (2, r2) };
                let stay = c1 + 33;
                if stay <= r + start1 {
                    c1 = stay;
                    p |= 1 << 2;
                } else {
                    c1 = r + start1;
                    p |= from << 2;
                }
            } else {
                c1 = INF;
            }

            let (from, r) = if r0 <= r1 { (0, r0) } else { (1, r1) };
            let stay = c2 + 48;
            if stay <= r + start2 {
                c2 = stay;
                p |= 2 << 4;
            } else {
                c2 = r + start2;
                p |= from << 4;
            }

            modes[i] = p;
        }

        let (mut m, cost) = if c0 <= c1 {
            if c0 <= c2 {
                (0, c0)
            } else {
                (2, c2)
            }
        } else if c1 <= c2 {
            (1, c1)
        } else {
            (2, c2)
        };

        // Replace packed modes with the chosen mode of each byte
        for i in (1..n).rev() {
            let p = (modes[i] >> (m * 2)) & 0b11;
            modes[i] = m;
            m = p;
        }
        modes[0] = m;

        cost.div_ceil(6)
    }
}
impl<'a> Encoder for MixedEncoder<'a> {
    fn bit_len(&mut self, version: Version) -> usize {
        let group = cci_group(version);
        if self.bits[group] == UNKNOWN {
            self.fill(version);
        }
        self.bits[group]
    }

    fn encode(&mut self, version: Version, push: &mut dyn FnMut(u16, u8)) {
        self.fill(version);

        let bytes = self.bytes;
        let modes = &*self.modes;
        let n = bytes.len();

        if n == 0 {
            push(ByteMode.indicator(), 4);
            push(0, ByteMode.cci_len(version));
            return;
        }

        let mut start = 0;
        while start < n {
            let mode = modes[start];
            let mut end = start + 1;
            while end < n && modes[end] == mode {
                end += 1;
            }
            let segment = &bytes[start..end];

            match mode {
                0 => {
                    push(NumericMode.indicator(), 4);
                    push(segment.len() as u16, NumericMode.cci_len(version));
                    let mut groups = segment.chunks_exact(3);
                    for g in &mut groups {
                        push(
                            (g[0] - b'0') as u16 * 100
                                + (g[1] - b'0') as u16 * 10
                                + (g[2] - b'0') as u16,
                            10,
                        );
                    }
                    match *groups.remainder() {
                        [a] => push((a - b'0') as u16, 4),
                        [a, b] => push((a - b'0') as u16 * 10 + (b - b'0') as u16, 7),
                        _ => {}
                    }
                }
                1 => {
                    push(AlphanumericMode.indicator(), 4);
                    push(segment.len() as u16, AlphanumericMode.cci_len(version));
                    let mut pairs = segment.chunks_exact(2);
                    for p in &mut pairs {
                        push(
                            B45[p[0] as usize] as u16 * 45 + B45[p[1] as usize] as u16,
                            11,
                        );
                    }
                    if let [a] = *pairs.remainder() {
                        push(B45[a as usize] as u16, 6);
                    }
                }
                _ => {
                    push(ByteMode.indicator(), 4);
                    push(segment.len() as u16, ByteMode.cci_len(version));
                    for &b in segment {
                        push(b as u16, 8);
                    }
                }
            }

            start = end;
        }
    }
}
