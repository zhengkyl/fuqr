// Copyright (c) 2026 Kyle Zheng
// Licensed under the MIT License.

#![no_std]

use core::fmt;

/// 1 to 40 inclusive
pub type Version = u8;
/// 0 to 3 inclusive
pub type Ecl = u8;
/// 0 to 7 inclusive
pub type Mask = u8;

#[derive(Clone)]
pub struct QrCode<const MAX_MATRIX_SIZE: usize = { modules_for(40) }> {
    pub matrix: [u8; MAX_MATRIX_SIZE],
    pub version: Version,
    pub ecl: Ecl,
    pub mask: Mask,
}
impl<const N: usize> fmt::Debug for QrCode<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QrCode")
            .field("version", &self.version)
            .field("ecl", &self.ecl)
            .field("mask", &self.mask)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerateOptions {
    pub min_version: Version,
    pub max_version: Version,
    pub min_ecl: Ecl,
    pub max_ecl: Ecl,
    pub mask: Mask,
}
impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            min_version: 1,
            max_version: 40,
            min_ecl: 0,
            max_ecl: 3,
            mask: 2,
        }
    }
}

pub fn generate<const N: usize>(
    content: &str,
    options: GenerateOptions,
    plugins: &mut [&mut dyn Plugin],
) -> Result<QrCode<N>, FuqrError> {
    generate_with_encoder(&mut ByteEncoder::new(content), options, plugins)
}

pub fn generate_with_encoder<const N: usize>(
    encoder: &mut dyn Encoder,
    mut options: GenerateOptions,
    plugins: &mut [&mut dyn Plugin],
) -> Result<QrCode<N>, FuqrError> {
    const { assert!(N >= modules_for(1), "matrix buffer smaller than version 1") };

    options.max_version = options.max_version.min(max_version_for(N));

    let details = determine_details(encoder, options, plugins)?;
    build_matrix(encoder, details, plugins)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SvgOptions<'a> {
    pub margin: i32,
    pub attributes: &'a str,
}
impl Default for SvgOptions<'_> {
    fn default() -> Self {
        Self {
            margin: 2,
            attributes: r#"xmlns="http://www.w3.org/2000/svg" width="300px" height="300px""#,
        }
    }
}

pub fn render_svg_into<W: fmt::Write, const N: usize>(
    qr: &QrCode<N>,
    options: SvgOptions,
    out: &mut W,
) -> fmt::Result {
    let stride = qr.version as usize * 4 + 17;
    let edges = stride + 1;
    let margin = options.margin;

    const RIGHT: u8 = 0;
    const DOWN: u8 = 1;
    const LEFT: u8 = 2;
    const UP: u8 = 3;
    let neighbor = |node: usize, dir: u8| match dir {
        RIGHT => node + edges,
        DOWN => node + 1,
        LEFT => node - edges,
        _ => node - 1,
    };

    // Outgoing edge directions per corner node (node = x * edges + y), one
    // 4-bit nibble per node, two nodes per byte: (stride + 1)^2 nibbles always
    // fit in the N >= stride^2 bytes the matrix itself needs. Adding an edge
    // cancels the opposite edge if present, so only contour edges survive.
    let mut next = [0u8; N];
    let bit = |node: usize, dir: u8| 1u8 << (dir + ((node as u8 & 1) << 2));
    let add_edge = |next: &mut [u8; N], from: usize, dir: u8| {
        let to = neighbor(from, dir);
        let back = bit(to, dir ^ 2);
        if next[to >> 1] & back != 0 {
            next[to >> 1] &= !back;
        } else {
            next[from >> 1] |= bit(from, dir);
        }
    };

    for y in 0..stride {
        for x in 0..stride {
            if qr.matrix[y * stride + x] & Module::ON == 0 {
                continue;
            }
            let tl = x * edges + y;
            add_edge(&mut next, tl, RIGHT);
            add_edge(&mut next, tl + edges, DOWN);
            add_edge(&mut next, tl + edges + 1, LEFT);
            add_edge(&mut next, tl + 1, UP);
        }
    }

    let width = stride as i32 + 2 * margin;
    write!(out, "<svg ")?;
    if !options.attributes.is_empty() {
        write!(out, "{} ", options.attributes)?;
    }
    write!(out, "viewBox=\"0 0 {width} {width}\">")?;
    write!(
        out,
        "<rect width=\"{width}\" height=\"{width}\" fill=\"#fff\"/>"
    )?;
    write!(out, "<path fill=\"#000\" d=\"")?;

    let nibble = |next: &[u8; N], node: usize| (next[node >> 1] >> ((node as u8 & 1) << 2)) & 0xF;
    for start in 0..edges * edges {
        if nibble(&next, start) == 0 {
            continue;
        }
        write!(
            out,
            "M{},{}",
            margin + (start / edges) as i32,
            margin + (start % edges) as i32
        )?;

        let mut curr = start;
        let mut dir = u8::MAX;
        let mut run = 0i32;
        loop {
            let d = nibble(&next, curr).trailing_zeros() as u8;
            next[curr >> 1] &= !bit(curr, d);

            let step = if d < 2 { 1 } else { -1 };
            if d == dir {
                run += step;
            } else {
                if dir != u8::MAX {
                    write!(out, "{}{}", if dir & 1 == 0 { 'h' } else { 'v' }, run)?;
                }
                dir = d;
                run = step;
            }
            curr = neighbor(curr, d);
            if curr == start {
                break;
            }
        }
        write!(out, "{}{}z", if dir & 1 == 0 { 'h' } else { 'v' }, run)?;
    }

    write!(out, "\"/></svg>")
}

pub struct Module;
impl Module {
    pub const ON: u8 = 1 << 0;
    pub const DATA: u8 = 1 << 1;
    pub const FINDER: u8 = 1 << 2;
    pub const ALIGNMENT: u8 = 1 << 3;
    pub const TIMING: u8 = 1 << 4;
    pub const FORMAT: u8 = 1 << 5;
    pub const VERSION: u8 = 1 << 6;
    pub const MODIFIER: u8 = 1 << 7;
}

pub const MAX_DATA_BYTES: usize = 3706; // v40 has no remainder
pub const MAX_MESSAGE_BYTES: usize = 2956;
pub const MAX_EC_PER_BLOCK: usize = 30;

/// Matrix buffer length needed for `version`, for sizing `QrCode<N>`.
pub const fn modules_for(version: Version) -> usize {
    let w = version as usize * 4 + 17;
    w * w
}
/// Largest version whose matrix fits in an `N`-byte buffer (0 if none fit).
pub const fn max_version_for(n: usize) -> Version {
    let mut v = 40;
    while v > 0 {
        if modules_for(v) <= n {
            return v;
        }
        v -= 1;
    }
    0
}

pub trait Encoder {
    fn bit_len(&mut self, version: Version) -> usize;
    fn encode(&mut self, version: Version, push: &mut dyn FnMut(u16, u8));
}

/// Generation parameters, seeded from [`GenerateOptions`] and settled during
/// [`determine_details`] (plugins get the last word via `mutate_details`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Details {
    pub version: Version,
    pub ecl: Ecl,
    pub mask: Mask,
}

#[allow(unused_variables)]
pub trait Plugin {
    fn mutate_details(
        &mut self,
        details: &mut Details,
        options: &GenerateOptions,
    ) -> Result<(), FuqrError> {
        Ok(())
    }
    fn mutate_message(
        &mut self,
        message_bytes: &mut [u8],
        padding_start: usize,
        matrix: &[u8],
        details: &Details,
    ) -> Result<(), FuqrError> {
        Ok(())
    }
    fn mutate_sequence(
        &mut self,
        interleaved: &mut [u8],
        matrix: &[u8],
        details: &Details,
    ) -> Result<(), FuqrError> {
        Ok(())
    }
    fn mutate_matrix(&mut self, matrix: &mut [u8], details: &Details) -> Result<(), FuqrError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuqrError {
    TextTooLong {
        max_version: Version,
    },
    /// For custom plugins. Strings are static; keep dynamic detail as plugin state.
    Plugin {
        code: &'static str,
        message: &'static str,
    },
}
impl FuqrError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::TextTooLong { .. } => "TEXT_TOO_LONG",
            Self::Plugin { code, .. } => code,
        }
    }
}
impl fmt::Display for FuqrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TextTooLong { max_version } => {
                write!(f, "Cannot fit in version {max_version}")
            }
            Self::Plugin { message, .. } => f.write_str(message),
        }
    }
}
impl core::error::Error for FuqrError {}

// ---- INTERNAL API BELOW, USERS YE BE WARNED ----
//
// Definitions:
// A black or white QR square is a bit (sometimes module/pixel).
// A group of eight QR bits is a byte (sometimes symbol/codeword).
// Message and error correction bytes make up a block (sometimes codeword).
// Blocks are interleaved and fill up the QR data section.
//
// |--------------------- data ----------------------|
// |---------------- block ---------------|  ...etc
// |---------- message ----------|-- ec --|
// |--- content ---|-- padding --|

#[derive(Debug)]
pub struct ByteEncoder<'a> {
    pub bytes: &'a [u8],
}
impl<'a> ByteEncoder<'a> {
    pub fn new(content: &'a str) -> Self {
        Self {
            bytes: content.as_bytes(),
        }
    }
}
impl<'a> Encoder for ByteEncoder<'a> {
    fn bit_len(&mut self, version: Version) -> usize {
        let cci = if version < 10 { 8 } else { 16 };
        4 + cci + self.bytes.len() * 8
    }
    fn encode(&mut self, version: Version, push: &mut dyn FnMut(u16, u8)) {
        let cci = if version < 10 { 8 } else { 16 };
        push(0b0100, 4);
        push(self.bytes.len() as u16, cci);
        for &b in self.bytes {
            push(b as u16, 8);
        }
    }
}

pub fn determine_details(
    encoder: &mut dyn Encoder,
    options: GenerateOptions,
    plugins: &mut [&mut dyn Plugin],
) -> Result<Details, FuqrError> {
    for version in options.min_version..=options.max_version {
        let v = version as usize;
        let req_bytes = encoder.bit_len(version).div_ceil(8);
        let data_bytes = (NUM_DATA_BITS[v] >> 3) as usize;

        if req_bytes <= data_bytes - NUM_EC_BYTES[v][options.min_ecl as usize] as usize {
            let mut ecl = options.min_ecl;
            while ecl < options.max_ecl
                && req_bytes <= data_bytes - NUM_EC_BYTES[v][ecl as usize + 1] as usize
            {
                ecl += 1;
            }

            let mut details = Details {
                version,
                ecl,
                mask: options.mask,
            };
            for p in plugins.iter_mut() {
                p.mutate_details(&mut details, &options)?;
            }
            return Ok(details);
        }
    }

    Err(FuqrError::TextTooLong {
        max_version: options.max_version,
    })
}

struct BitSink<'a> {
    out: &'a mut [u8],
    buf: u32,
    buf_len: u32,
    pos: usize,
}

impl BitSink<'_> {
    fn push(&mut self, value: u16, n: u8) {
        self.buf = (self.buf << n) | value as u32;
        self.buf_len += n as u32;
        while self.buf_len >= 8 {
            self.buf_len -= 8;
            self.out[self.pos] = (self.buf >> self.buf_len) as u8;
            self.pos += 1;
        }
    }
}

pub fn build_matrix<const N: usize>(
    encoder: &mut dyn Encoder,
    details: Details,
    plugins: &mut [&mut dyn Plugin],
) -> Result<QrCode<N>, FuqrError> {
    let Details { version, ecl, mask } = details;

    let v = version as usize;
    let num_bits = NUM_DATA_BITS[v] as usize;
    let num_bytes = num_bits / 8;
    let remainder_bits = num_bits % 8;
    let num_ec_bytes = NUM_EC_BYTES[v][ecl as usize] as usize;
    let num_message_bytes = num_bytes - num_ec_bytes;

    let mut message_bytes = [0u8; MAX_MESSAGE_BYTES];
    let padding_start;
    {
        let mut sink = BitSink {
            out: &mut message_bytes[..num_message_bytes],
            buf: 0,
            buf_len: 0,
            pos: 0,
        };
        encoder.encode(version, &mut |bits, len| sink.push(bits, len));

        let remaining_data_bits = num_message_bytes * 8 - sink.pos * 8 - sink.buf_len as usize;
        sink.push(
            0,
            if remaining_data_bits < 4 {
                remaining_data_bits as u8
            } else {
                4
            },
        );
        sink.push(0, ((8 - sink.buf_len) & 7) as u8);

        padding_start = sink.pos;
        let mut alternating: u8 = 0b1110_1100;
        for _ in sink.pos..num_message_bytes {
            sink.push(alternating as u16, 8);
            alternating ^= 0b1111_1101;
        }
    }

    let width = v * 4 + 17;
    let mut matrix = [0u8; N];
    {
        let mut set = |x: usize, y: usize, value: u8| matrix[y * width + x] |= value;
        visit_finder_patterns(width, &mut set);
        visit_timing_patterns(width, &mut set);
        visit_format_info(ecl, mask, width, &mut set);
        visit_alignment_patterns(version, width, &mut set);
        visit_version_info(version, width, &mut set);
    }

    for p in plugins.iter_mut() {
        p.mutate_message(
            &mut message_bytes[..num_message_bytes],
            padding_start,
            &matrix[..width * width],
            &details,
        )?;
    }

    let blocks = NUM_BLOCKS[v][ecl as usize] as usize;
    let g2_blocks = num_bytes % blocks;
    let g1_blocks = blocks - g2_blocks;
    let message_per_g1 = num_message_bytes / blocks;
    let message_per_g2 = message_per_g1 + 1;
    let ec_per_block = num_ec_bytes / blocks;

    let interleaved_len = num_bytes + (remainder_bits > 0) as usize;
    let mut interleaved = [0u8; MAX_DATA_BYTES];

    let num_g1 = g1_blocks * message_per_g1;
    for (i, byte) in interleaved[..num_message_bytes].iter_mut().enumerate() {
        let col = i / blocks;
        let row = i % blocks;
        *byte = if col < message_per_g1 {
            message_bytes[row * message_per_g1 + col + row.saturating_sub(g1_blocks)]
        } else {
            message_bytes[num_g1 + row * message_per_g2 + col]
        };
    }

    let divisor = generator_polynomial(ec_per_block);
    let divisor = &divisor[..ec_per_block];
    let mut ec = [0u8; MAX_EC_PER_BLOCK];
    for i in 0..g1_blocks {
        polynomial_remainder(
            &message_bytes[i * message_per_g1..(i + 1) * message_per_g1],
            divisor,
            &mut ec,
        );
        for j in 0..ec_per_block {
            interleaved[num_message_bytes + j * blocks + i] = ec[j];
        }
    }
    let g2_start = num_g1;
    for i in 0..g2_blocks {
        polynomial_remainder(
            &message_bytes[g2_start + i * message_per_g2..g2_start + (i + 1) * message_per_g2],
            divisor,
            &mut ec,
        );
        for j in 0..ec_per_block {
            interleaved[num_message_bytes + j * blocks + i + g1_blocks] = ec[j];
        }
    }

    for p in plugins.iter_mut() {
        p.mutate_sequence(
            &mut interleaved[..interleaved_len],
            &matrix[..width * width],
            &details,
        )?;
    }

    let mut bit_idx = 0usize;
    let masker = MASKERS[mask as usize];
    iterate_mostly_data_modules(width, |x, y| {
        if matrix[y * width + x] == 0 {
            let bit = (interleaved[bit_idx >> 3] >> (7 - (bit_idx & 7))) & Module::ON;
            matrix[y * width + x] |= Module::DATA | (bit ^ masker(x, y) as u8);
            bit_idx += 1;
        }
    });

    for p in plugins.iter_mut() {
        p.mutate_matrix(&mut matrix[..width * width], &details)?;
    }

    Ok(QrCode {
        matrix,
        version,
        ecl,
        mask,
    })
}

pub fn iterate_mostly_data_modules(width: usize, mut callback: impl FnMut(usize, usize)) {
    let w = width as i32;
    let mut y = w - 1;
    let mut dy = -1i32;
    let mut row_end = 9i32;
    let mut row_steps = w - 10;
    let mut x = w - 1;
    while x > 0 {
        loop {
            callback(x as usize, y as usize);
            callback(x as usize - 1, y as usize);
            if y == row_end {
                break;
            }
            y += dy;
        }
        dy = -dy;
        if x == w - 7 {
            row_steps = w - 1;
            row_end = 0;
        } else if x == 10 {
            row_steps = w - 18;
            row_end = 9;
            y = w - 9;
        } else {
            if x == 8 {
                x -= 1;
            }
            row_end += dy * row_steps;
        }
        x -= 2;
    }
}

pub fn visit_finder_patterns(width: usize, set: &mut impl FnMut(usize, usize, u8)) {
    let mut set_finder = |x: i32, y: i32| {
        for dy in -3..=3i32 {
            for dx in -3..=3i32 {
                let max = dy.abs().max(dx.abs());
                set(
                    (x + dx) as usize,
                    (y + dy) as usize,
                    if max == 3 {
                        Module::FINDER | Module::ON
                    } else if max == 2 {
                        Module::FINDER
                    } else {
                        Module::FINDER | Module::MODIFIER | Module::ON
                    },
                );
            }
        }
    };
    set_finder(3, 3);
    set_finder(3, width as i32 - 4);
    set_finder(width as i32 - 4, 3);
}

pub fn visit_alignment_patterns(
    version: Version,
    width: usize,
    set: &mut impl FnMut(usize, usize, u8),
) {
    if version < 2 {
        return;
    }
    let mut set_alignment = |cx: usize, cy: usize| {
        for dy in -2..=2i32 {
            for dx in -2..=2i32 {
                let max = dy.abs().max(dx.abs());
                set(
                    (cx as i32 + dx) as usize,
                    (cy as i32 + dy) as usize,
                    if max == 2 {
                        Module::ALIGNMENT | Module::ON
                    } else if max == 1 {
                        Module::ALIGNMENT
                    } else {
                        Module::ALIGNMENT | Module::MODIFIER | Module::ON
                    },
                );
            }
        }
    };
    let first = 6usize;
    let last = width - 7;
    let len = version as usize / 7 + 2;
    let mut coords = [0usize; 7];
    let mut n = 0;
    coords[n] = first;
    n += 1;
    if version >= 7 {
        for i in (1..=len - 2).rev() {
            coords[n] = last - i * ALIGN_OFFSETS[version as usize - 7] as usize;
            n += 1;
        }
    }
    coords[n] = last;
    for i in 0..len {
        for j in 0..len {
            if (i == 0 && (j == 0 || j == len - 1)) || (i == len - 1 && j == 0) {
                continue;
            }
            set_alignment(coords[i], coords[j]);
        }
    }
}

pub fn visit_timing_patterns(width: usize, set: &mut impl FnMut(usize, usize, u8)) {
    for i in 8..width - 8 {
        let module = Module::TIMING | ((i as u8 & 1) ^ Module::ON);
        set(6, i, module);
        set(i, 6, module);
    }
}

pub fn visit_version_info(version: Version, width: usize, set: &mut impl FnMut(usize, usize, u8)) {
    if version < 7 {
        return;
    }
    let version_info = VERSION_INFO[version as usize - 7];
    for i in 0..6 {
        for j in 0..3 {
            let bit = ((version_info >> (i * 3 + j)) as u8) & Module::ON;
            set(width - 11 + j, i, Module::VERSION | bit);
            set(i, width - 11 + j, Module::VERSION | Module::MODIFIER | bit);
        }
    }
}

pub fn visit_format_info(
    ecl: Ecl,
    mask: Mask,
    width: usize,
    set: &mut impl FnMut(usize, usize, u8),
) {
    let f = FORMAT_INFO[ecl as usize][mask as usize];
    for i in 0..6 {
        set(8, i, Module::FORMAT | (((f >> i) as u8) & Module::ON));
    }
    set(8, 7, Module::FORMAT | (((f >> 6) as u8) & Module::ON));
    set(8, 8, Module::FORMAT | (((f >> 7) as u8) & Module::ON));
    set(7, 8, Module::FORMAT | (((f >> 8) as u8) & Module::ON));
    for i in 9..15 {
        set(14 - i, 8, Module::FORMAT | (((f >> i) as u8) & Module::ON));
    }

    let f_copy = Module::FORMAT | Module::MODIFIER;
    for i in 0..8 {
        set(width - 1 - i, 8, f_copy | (((f >> i) as u8) & Module::ON));
    }
    for i in 8..15 {
        set(8, width - 15 + i, f_copy | (((f >> i) as u8) & Module::ON));
    }
    set(8, width - 8, f_copy | Module::ON);
}

const fn gf_tables() -> ([u8; 255], [u8; 256]) {
    let mut exp = [0u8; 255];
    let mut log = [0u8; 256];
    let mut x: u32 = 1;
    let mut i = 0;
    while i < 255 {
        exp[i] = x as u8;
        log[x as usize] = i as u8;
        if x & 0b1000_0000 != 0 {
            x = (x << 1) ^ 0b1_0001_1101;
        } else {
            x <<= 1;
        }
        i += 1;
    }
    (exp, log)
}

const GF_TABLES: ([u8; 255], [u8; 256]) = gf_tables();
pub const EXP_TABLE: [u8; 255] = GF_TABLES.0;
pub const LOG_TABLE: [u8; 256] = GF_TABLES.1;

/// Writes the `generator.len()` remainder bytes into `out`.
pub fn polynomial_remainder(data: &[u8], generator: &[u8], out: &mut [u8]) {
    let n = generator.len();
    let out = &mut out[..n];
    out.fill(0);
    for &d in data {
        let factor = d ^ out[0];
        out.copy_within(1.., 0);
        out[n - 1] = 0;
        if factor != 0 {
            let alpha_diff = LOG_TABLE[factor as usize] as u32;
            for j in 0..n {
                out[j] ^= EXP_TABLE[((generator[j] as u32 + alpha_diff) % 255) as usize];
            }
        }
    }
}

/// Coefficient exponents of the degree-`ecc_per_block` generator, leading term
/// omitted. Only the first `ecc_per_block` entries are meaningful.
pub fn generator_polynomial(ecc_per_block: usize) -> [u8; MAX_EC_PER_BLOCK] {
    let mut prev = [0u8; MAX_EC_PER_BLOCK];
    let mut curr = [0u8; MAX_EC_PER_BLOCK];
    for i in 2..=ecc_per_block {
        core::mem::swap(&mut prev, &mut curr);
        let k = i as u32 - 1;
        curr[i - 1] = ((prev[i - 2] as u32 + k) % 255) as u8;
        for j in (1..=i - 2).rev() {
            let exp = (prev[j - 1] as u32 + k) % 255;
            curr[j] = LOG_TABLE[(EXP_TABLE[prev[j] as usize] ^ EXP_TABLE[exp as usize]) as usize];
        }
        curr[0] = LOG_TABLE[(EXP_TABLE[prev[0] as usize] ^ EXP_TABLE[k as usize]) as usize];
    }
    curr
}

#[rustfmt::skip]
pub const NUM_DATA_BITS: [u16; 41] = [
    0,
    208, 359, 567, 807,
    1079, 1383, 1568, 1936,
    2336, 2768, 3232, 3728,
    4256, 4651, 5243, 5867,
    6523, 7211, 7931, 8683,
    9252, 10068, 10916, 11796,
    12708, 13652, 14628, 15371,
    16411, 17483, 18587, 19723,
    20891, 22091, 23008, 24272,
    25568, 26896, 28256, 29648,
];

#[rustfmt::skip]
pub const NUM_EC_BYTES: [[u16; 4]; 41] = [
    [0, 0, 0, 0],
    [7, 10, 13, 17], [10, 16, 22, 28], [15, 26, 36, 44], [20, 36, 52, 64],
    [26, 48, 72, 88], [36, 64, 96, 112], [40, 72, 108, 130], [48, 88, 132, 156],
    [60, 110, 160, 192], [72, 130, 192, 224], [80, 150, 224, 264], [96, 176, 260, 308],
    [104, 198, 288, 352], [120, 216, 320, 384], [132, 240, 360, 432], [144, 280, 408, 480],
    [168, 308, 448, 532], [180, 338, 504, 588], [196, 364, 546, 650], [224, 416, 600, 700],
    [224, 442, 644, 750], [252, 476, 690, 816], [270, 504, 750, 900], [300, 560, 810, 960],
    [312, 588, 870, 1050], [336, 644, 952, 1110], [360, 700, 1020, 1200], [390, 728, 1050, 1260],
    [420, 784, 1140, 1350], [450, 812, 1200, 1440], [480, 868, 1290, 1530], [510, 924, 1350, 1620],
    [540, 980, 1440, 1710], [570, 1036, 1530, 1800], [570, 1064, 1590, 1890], [600, 1120, 1680, 1980],
    [630, 1204, 1770, 2100], [660, 1260, 1860, 2220], [720, 1316, 1950, 2310], [750, 1372, 2040, 2430],
];

#[rustfmt::skip]
pub const NUM_BLOCKS: [[u8; 4]; 41] = [
    [0, 0, 0, 0],
    [1, 1, 1, 1], [1, 1, 1, 1], [1, 1, 2, 2], [1, 2, 2, 4],
    [1, 2, 4, 4], [2, 4, 4, 4], [2, 4, 6, 5], [2, 4, 6, 6],
    [2, 5, 8, 8], [4, 5, 8, 8], [4, 5, 8, 11], [4, 8, 10, 11],
    [4, 9, 12, 16], [4, 9, 16, 16], [6, 10, 12, 18], [6, 10, 17, 16],
    [6, 11, 16, 19], [6, 13, 18, 21], [7, 14, 21, 25], [8, 16, 20, 25],
    [8, 17, 23, 25], [9, 17, 23, 34], [9, 18, 25, 30], [10, 20, 27, 32],
    [12, 21, 29, 35], [12, 23, 34, 37], [12, 25, 34, 40], [13, 26, 35, 42],
    [14, 28, 38, 45], [15, 29, 40, 48], [16, 31, 43, 51], [17, 33, 45, 54],
    [18, 35, 48, 57], [19, 37, 51, 60], [19, 38, 53, 63], [20, 40, 56, 66],
    [21, 43, 59, 70], [22, 45, 62, 74], [24, 47, 65, 77], [25, 49, 68, 81],
];

/// Starts at 7
#[rustfmt::skip]
pub const ALIGN_OFFSETS: [u8; 34] = [
            16, 18,
    20, 22, 24, 26, 
    28, 20, 22, 24, 
    24, 26, 28, 28, 
    22, 24, 24, 26, 
    26, 28, 28, 24, 
    24, 26, 26, 26, 
    28, 28, 24, 26,
    26, 26, 28, 28,
];

/// Starts at 7
#[rustfmt::skip]
pub const VERSION_INFO: [u32; 34] = [
                      0x07c94, 0x085bc,
    0x09a99, 0x0a4d3, 0x0bbf6, 0x0c762,
    0x0d847, 0x0e60d, 0x0f928, 0x10b78,
    0x1145d, 0x12a17, 0x13532, 0x149a6,
    0x15683, 0x168c9, 0x177ec, 0x18ec4,
    0x191e1, 0x1afab, 0x1b08e, 0x1cc1a,
    0x1d33f, 0x1ed75, 0x1f250, 0x209d5,
    0x216f0, 0x228ba, 0x2379f, 0x24b0b,
    0x2542e, 0x26a64, 0x27541, 0x28c69,
];
#[rustfmt::skip]
pub const FORMAT_INFO: [[u16; 8]; 4] = [
    [0x77c4, 0x72f3, 0x7daa, 0x789d, 0x662f, 0x6318, 0x6c41, 0x6976],
    [0x5412, 0x5125, 0x5e7c, 0x5b4b, 0x45f9, 0x40ce, 0x4f97, 0x4aa0],
    [0x355f, 0x3068, 0x3f31, 0x3a06, 0x24b4, 0x2183, 0x2eda, 0x2bed],
    [0x1689, 0x13be, 0x1ce7, 0x19d0, 0x0762, 0x0255, 0x0d0c, 0x083b],
];

pub const MASKERS: [fn(usize, usize) -> bool; 8] = [
    |x, y| (x + y) % 2 == 0,
    |_, y| y % 2 == 0,
    |x, _| x % 3 == 0,
    |x, y| (x + y) % 3 == 0,
    |x, y| (x / 3 + (y >> 1)) % 2 == 0,
    |x, y| (x * y) % 2 + (x * y) % 3 == 0,
    |x, y| ((x * y) % 2 + (x * y) % 3) % 2 == 0,
    |x, y| ((x + y) % 2 + (x * y) % 3) % 2 == 0,
];
