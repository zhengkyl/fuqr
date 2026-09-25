#[cfg(feature = "extras")]
use fuqr::extras::encoders::{
    AlphanumericEncoder, AlphanumericMode, MixedEncoder, NumericEncoder, NumericMode,
};
use fuqr::{
    generate_with_encoder, ByteEncoder, ByteMode, Encoder, GenerateOptions, Mode, QrCode,
    NUM_DATA_BITS, NUM_EC_BYTES,
};

// Version groups with different char count indicator lengths
const VERSIONS: [u8; 6] = [1, 9, 10, 26, 27, 40];

const ALPHANUMERIC: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

const CONTENTS: &[&str] = &[
    "",
    "0",
    "8675309",
    "HELLO WORLD",
    "hello",
    "Hello, World!",
    "https://example.com/2470295/manuals/525322511#step-3",
    "HTTPS://EXAMPLE.COM/PATH $%*+-./:",
    "ORDER 4200 $9.99 SHIP-TO 90210",
    "héllo wörld, grüße",
    "日本語のテキストです 123 ABC",
    "\u{1f389}\u{1f680} party \u{1f44d}\u{1f3fd}",
    "Order #42 costs $9.99 — ship to 90210 \u{1f4e6}",
];

fn mode_of(byte: u8) -> u8 {
    if byte.is_ascii_digit() {
        0
    } else if ALPHANUMERIC.contains(&byte) {
        1
    } else {
        2
    }
}

fn seg_len(mode: u8, len: usize, version: u8) -> usize {
    match mode {
        0 => NumericMode.seg_len(len, version),
        1 => AlphanumericMode.seg_len(len, version),
        _ => ByteMode.seg_len(len, version),
    }
}

// Obviously correct O(n^2) search over every possible last segment
fn shortest_bit_len(content: &str, version: u8) -> usize {
    let bytes = content.as_bytes();
    let n = bytes.len();
    if n == 0 {
        return ByteMode.seg_len(0, version);
    }
    let mut best = vec![usize::MAX; n + 1];
    best[0] = 0;
    for end in 1..=n {
        for mode in 0..3 {
            let mut start = end;
            while start > 0 && mode_of(bytes[start - 1]) <= mode {
                start -= 1;
                best[end] = best[end].min(best[start] + seg_len(mode, end - start, version));
            }
        }
    }
    best[n]
}

fn encode_bits(encoder: &mut dyn Encoder, version: u8) -> Vec<bool> {
    let mut bits = Vec::new();
    encoder.encode(version, &mut |value, len| {
        for i in (0..len).rev() {
            bits.push((value >> i) & 1 == 1);
        }
    });
    bits
}

struct Reader<'a> {
    bits: &'a [bool],
    pos: usize,
}
impl Reader<'_> {
    fn read(&mut self, len: u8) -> usize {
        let mut value = 0;
        for _ in 0..len {
            value = value << 1 | self.bits[self.pos] as usize;
            self.pos += 1;
        }
        value
    }
}

// Parses segments back into content bytes
fn decode(bits: &[bool], version: u8) -> Vec<u8> {
    let mut r = Reader { bits, pos: 0 };
    let mut out = Vec::new();
    while r.pos < bits.len() {
        match r.read(4) {
            0b0001 => {
                let mut count = r.read(NumericMode.cci_len(version));
                while count >= 3 {
                    out.extend(format!("{:03}", r.read(10)).bytes());
                    count -= 3;
                }
                match count {
                    1 => out.extend(format!("{}", r.read(4)).bytes()),
                    2 => out.extend(format!("{:02}", r.read(7)).bytes()),
                    _ => {}
                }
            }
            0b0010 => {
                let mut count = r.read(AlphanumericMode.cci_len(version));
                while count >= 2 {
                    let pair = r.read(11);
                    out.push(ALPHANUMERIC[pair / 45]);
                    out.push(ALPHANUMERIC[pair % 45]);
                    count -= 2;
                }
                if count == 1 {
                    out.push(ALPHANUMERIC[r.read(6)]);
                }
            }
            0b0100 => {
                for _ in 0..r.read(ByteMode.cci_len(version)) {
                    out.push(r.read(8) as u8);
                }
            }
            mode => panic!("bad mode indicator {mode:#06b}"),
        }
    }
    out
}

// Returns a description of what is wrong, or None if nothing
fn check_mixed(content: &str, version: u8) -> Option<String> {
    let mut scratch = vec![0; content.len()];
    let mut encoder = MixedEncoder::new(content, &mut scratch);
    let bits = encoder.bit_len(version);
    let shortest = shortest_bit_len(content, version);
    let encoded = encode_bits(&mut encoder, version);
    if bits != shortest || encoded.len() != bits {
        let pushed = encoded.len();
        return Some(format!(
            "{content:?} v{version} bit_len {bits}, shortest {shortest}, pushed {pushed}"
        ));
    }
    // Content that can't fit may overflow its char count indicator, but is never encoded
    let v = version as usize;
    if bits > (NUM_DATA_BITS[v] as usize / 8 - NUM_EC_BYTES[v][0] as usize) * 8 {
        return None;
    }
    let decoded = decode(&encoded, version);
    if decoded != content.as_bytes() {
        let decoded = String::from_utf8_lossy(&decoded);
        return Some(format!("{content:?} v{version} decoded {decoded:?}"));
    }
    None
}

// All strings of length up to max_len made from alphabet
fn strings(alphabet: &[&str], max_len: usize) -> Vec<String> {
    let mut all = Vec::new();
    let mut layer = vec![String::new()];
    for _ in 0..max_len {
        layer = layer
            .iter()
            .flat_map(|s| alphabet.iter().map(move |c| format!("{s}{c}")))
            .collect();
        all.extend(layer.iter().cloned());
    }
    all
}

#[test]
fn mixed_regressions() {
    // 7 numeric + 2 alphanumeric beats 9 alphanumeric
    assert_eq!(MixedEncoder::new("0000000A0", &mut [0; 9]).bit_len(1), 62);
    assert_eq!(MixedEncoder::new("", &mut []).bit_len(1), 12);
}

#[test]
fn mixed_is_optimal() {
    let mut contents = strings(&["0", "A", "a"], 10);
    contents.extend(strings(&["7", ":", " ", "b", "é"], 5));
    contents.extend(CONTENTS.iter().map(|s| s.to_string()));
    contents.extend(CONTENTS.iter().map(|s| s.repeat(20)));

    for version in VERSIONS {
        let errors: Vec<_> = contents
            .iter()
            .filter_map(|content| check_mixed(content, version))
            .take(10)
            .collect();
        assert!(errors.is_empty(), "{errors:#?}");
    }
}

#[test]
fn single_mode_round_trip() {
    for version in VERSIONS {
        for content in CONTENTS {
            let encoder = &mut ByteEncoder::new(content);
            let bits = encode_bits(encoder, version);
            assert_eq!(bits.len(), encoder.bit_len(version));
            assert_eq!(decode(&bits, version), content.as_bytes());

            if let Ok(encoder) = &mut AlphanumericEncoder::new(content) {
                let bits = encode_bits(encoder, version);
                assert_eq!(bits.len(), encoder.bit_len(version));
                assert_eq!(decode(&bits, version), content.as_bytes());
            }
            if let Ok(encoder) = &mut NumericEncoder::new(content) {
                let bits = encode_bits(encoder, version);
                assert_eq!(bits.len(), encoder.bit_len(version));
                assert_eq!(decode(&bits, version), content.as_bytes());
            }
        }
    }
}

#[test]
fn rejects_invalid_content() {
    let err = NumericEncoder::new("12a").unwrap_err();
    assert_eq!(err.code(), "INVALID_ENCODING");
    assert!(NumericEncoder::new("١٢").is_err());

    let err = AlphanumericEncoder::new("Hello").unwrap_err();
    assert_eq!(err.code(), "INVALID_ENCODING");
    assert!(AlphanumericEncoder::new("HELLO 42").is_ok());
}

#[test]
#[should_panic(expected = "scratch buffer shorter than content")]
fn mixed_scratch_too_short() {
    MixedEncoder::new("é", &mut [0; 1]);
}

#[test]
fn generates_with_encoders() {
    let content = "HTTPS://EXAMPLE.COM/1234567890/ABCDEFGHIJ/1234567890";
    let byte: QrCode = generate_with_encoder(
        &mut ByteEncoder::new(content),
        GenerateOptions::default(),
        &mut [],
    )
    .unwrap();
    let mut scratch = [0; 64];
    let mixed: QrCode = generate_with_encoder(
        &mut MixedEncoder::new(content, &mut scratch),
        GenerateOptions::default(),
        &mut [],
    )
    .unwrap();
    assert_eq!(byte.version, 3);
    assert_eq!(mixed.version, 3);
    assert!(mixed.ecl > byte.ecl);
}
