// values used while encoding format
#[derive(Clone, Copy)]
pub enum ECL {
    Low = 1,      // 7
    Medium = 0,   // 15
    Quartile = 3, // 25
    High = 2,     // 30
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
    // Terminator 0000, but optional and can be truncated
}

// pad codewords 11101100 00010001

// this is fine for now
impl QRCode {
    fn push_bits(&mut self, data: usize, len: usize) {
        for i in (0..len).rev() {
            self.data.push((data & (1 << i)) != 0);
        }
    }
    fn dims(self) -> u8 {
        self.version * 4 + 17
    }
}
pub struct QRCode {
    pub data: Vec<bool>,
    pub ecc: ECL,
    pub mask: u8,    // 1  - 8
    pub version: u8, // 1 - 40
}

// size = 4 * version + 17

struct Segment {
    mode: Mode,
    data: Vec<bool>,
}

fn encode_data(data: &str) {}

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

// input fits in u8 b/c numeric
fn encode_numeric(qrcode: &mut QRCode, input: &str) {
    qrcode.push_bits(0b0001, 4);
    qrcode.push_bits(
        input.len(),
        bits_char_count_indicator(qrcode.version, Mode::Numeric),
    );

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
    qrcode.push_bits(
        input.len(),
        bits_char_count_indicator(qrcode.version, Mode::Alphanumeric),
    );

    let input = input.as_bytes();

    for i in 0..(input.len() / 2) {
        let group =
            ascii_to_b45(input[i * 2]) as usize * 45 + ascii_to_b45(input[i * 2 + 1]) as usize;
        qrcode.push_bits(group, 11);
    }

    if (input.len() & 1) == 1 {
        qrcode.push_bits(ascii_to_b45(input[input.len() - 1]).into(), 6);
    }
}

// ISO-8859-1 aka first 256 unicode
fn encode_byte(qrcode: &mut QRCode, input: &str) {
    qrcode.push_bits(0b0100, 4);
    qrcode.push_bits(
        input.len(),
        bits_char_count_indicator(qrcode.version, Mode::Byte),
    );
    for c in input.as_bytes() {
        qrcode.push_bits((*c).into(), 8);
    }
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
        assert_eq!(qrcode.data, get_data_vec("0001 0000000001 0001"));

        qrcode.data = Vec::new();
        encode_numeric(&mut qrcode, "99");
        assert_eq!(qrcode.data, get_data_vec("0001 0000000010 1100011"));

        qrcode.data = Vec::new();
        encode_numeric(&mut qrcode, "123456");
        assert_eq!(
            qrcode.data,
            get_data_vec("0001 0000000110 0001111011 0111001000")
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
        assert_eq!(qrcode.data, get_data_vec("0010 000000001 000001"));

        qrcode.data = Vec::new();
        encode_alphanumeric(&mut qrcode, "99");
        assert_eq!(qrcode.data, get_data_vec("0010 000000010 00110011110"));

        qrcode.data = Vec::new();
        encode_alphanumeric(&mut qrcode, "ABC1::4");
        assert_eq!(
            qrcode.data,
            get_data_vec("0010 000000111 00111001101 01000011101 11111101000 000100")
        );
    }

    #[test]
    fn encode_byte_works() {
        let mut qrcode = QRCode {
            data: Vec::new(),
            mask: 0,
            version: 1,
            ecc: ECL::Low,
        };

        encode_byte(&mut qrcode, "0");
        assert_eq!(qrcode.data, get_data_vec("0100 00000001 00110000"));
    }
}
