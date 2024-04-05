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

fn ascii_to_b45(c: u8) -> u8 {
    match c {
        x if x >= b'A' => x - b'A' + 10,
        b':' => 44,
        x if x >= b'0' => x - b'0',
        b' ' => 36,
        b'$' => 37,
        b'%' => 38,
        b'*' => 39,
        b'+' => 40,
        b'-' => 41,
        b'.' => 42,
        b'/' => 43,
        _ => unreachable!("Not b45 encodable"),
    }
}
fn encode_alphanumeric(qrcode: &mut QRCode, input: &str) {
    qrcode.push_bits(0b0010, 4);

    let input = input.as_bytes();

    for i in 0..(input.len() / 2) {
        let group =
            ascii_to_b45(input[i * 2]) as usize * 45 + ascii_to_b45(input[i * 2 + 1]) as usize;
        qrcode.push_bits(group, 11);
    }

    if (input.len() & 1) == 1 {
        qrcode.push_bits(ascii_to_b45(input[input.len() - 1]).into(), 6);
    }

    qrcode.push_bits(
        input.len(),
        bits_char_count_indicator(qrcode.version, Mode::Alphanumeric),
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

    #[test]
    fn encode_alphanumeric_works() {
        let mut qrcode = QRCode {
            data: Vec::new(),
            mask: 0,
            version: 1,
            ecc: ECL::Low,
        };

        encode_alphanumeric(&mut qrcode, "1");
        assert_eq!(qrcode.data, get_data_vec("0010 000001 000000001"));

        qrcode.data = Vec::new();
        encode_alphanumeric(&mut qrcode, "99");
        assert_eq!(qrcode.data, get_data_vec("0010 00110011110 000000010"));

        qrcode.data = Vec::new();
        encode_alphanumeric(&mut qrcode, "ABC1::4");
        assert_eq!(
            qrcode.data,
            get_data_vec("0010 00111001101 01000011101 11111101000 000100 000000111")
        );
    }
}
