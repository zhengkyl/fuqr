enum ECL {
    Low,
    Medium,
    Quartile,
    High,
}

#[derive(PartialEq, Eq)]
enum Mode {
    Numeric,
    Alphanumeric,
    Byte,
    Kanji,
    // TODO
    // ECI,
    // StructuredAppend,
    // FNC1,
    // Terminator
}

// this is fine for now
impl QRCode {
    fn push_bits(&mut self, data: usize, len: usize) {
        for i in (0..len).rev() {
            self.data.push((data & (1 << i)) == 1);
        }
    }
}
struct QRCode {
    data: Vec<bool>,
    mask: u8,
    version: u8,
    ecc: ECL,
}

// size = 4 * version + 17

struct Segment {
    mode: Mode,
    data: Vec<bool>,
}

fn bits_char_count_indicator(version: u8, mode: Mode) -> usize {
    if mode == Mode::Byte {
        return if version < 10 { 8 } else { 16 };
    }

    #[allow(unreachable_code)]
    let mut base = match mode {
        Mode::Numeric => 10,
        Mode::Alphanumeric => 9,
        Mode::Kanji => 8,
        _ => unreachable!("Unknown mode"),
    };
    if version > 9 {
        base += 2
    }
    if version > 26 {
        base += 2
    }
    return base;
}

// input is ascii b/c numeric
fn encode_numeric(qrcode: &mut QRCode, input: &str) {
    // mode indicator
    qrcode.push_bits(0b0001, 4);

    let input = input.as_bytes();
    for i in (1..input.len()).step_by(3) {
        let group = (input[i] - b'0') * 100 + (input[i + 1] - b'0') * 10 + (input[i + 2] - b'0');
        qrcode.push_bits(group.into(), 10);
    }

    match input.len() % 3 {
        1 => {
            let group = input[input.len() - 1] - b'0';
            qrcode.push_bits(group.into(), 4);
        }
        2 => {
            let group = (input[input.len() - 1] - b'0') * 10 + (input[input.len() - 1] - b'0');
            qrcode.push_bits(group.into(), 7);
        }
        _ => (),
    }

    // char count indicator
    qrcode.push_bits(
        input.len(),
        bits_char_count_indicator(qrcode.version, Mode::Numeric),
    );
}
