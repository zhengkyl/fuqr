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
            self.data.push((data & (1 << i)) != 0);
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
    for i in 0..(input.len() / 3) {
        let group = (input[i * 3] - b'0') as usize * 100
            + (input[i * 3 + 1] - b'0') as usize * 10
            + (input[i * 3 + 2] - b'0') as usize;
        qrcode.push_bits(group, 10);
    }

    match input.len() % 3 {
        1 => {
            let group = input[input.len() - 1] - b'0';
            qrcode.push_bits(group.into(), 4);
        }
        2 => {
            let group = (input[input.len() - 2] - b'0') * 10 + (input[input.len() - 1] - b'0');
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

#[cfg(test)]
mod tests {
    use super::*;
    fn get_data_vec(bits: &str) -> Vec<bool> {
        let mut v = Vec::new();
        for c in bits.chars() {
            match c {
                '0' => v.push(false),
                '1' => v.push(true),
                _ => (),
            }
        }
        v
    }

    #[test]
    fn encode_numeric_works() {
        let mut qrcode = QRCode {
            data: Vec::new(),
            mask: 0,
            version: 1,
            ecc: ECL::Low,
        };

        encode_numeric(&mut qrcode, "1");
        assert_eq!(qrcode.data, get_data_vec("0001 0001 0000000001"));

        qrcode.data = Vec::new();
        encode_numeric(&mut qrcode, "99");
        assert_eq!(qrcode.data, get_data_vec("0001 1100011 0000000010"));

        qrcode.data = Vec::new();
        encode_numeric(&mut qrcode, "123456");
        assert_eq!(
            qrcode.data,
            get_data_vec("0001 0001111011 0111001000 0000000110")
        );
    }
}
