use crate::{data::QRData, error_correction::ECL, version::Version};

#[derive(PartialEq, Eq)]
pub enum Mode {
    Numeric,
    Alphanumeric,
    Byte,
    // TODO
    // Kanji,
    // ECI,
    // StructuredAppend,
    // FNC1,
}

impl QRCode {
    pub fn dims(self) -> u8 {
        self.version.0 * 4 + 17
    }
}

pub struct QRCode {
    pub sequenced_data: Vec<u8>,
    pub ecl: ECL,
    pub mask: u8,         // 1  - 8
    pub version: Version, // 1 - 40
}

fn bits_char_count_indicator(version: u8, mode: Mode) -> usize {
    if mode == Mode::Byte {
        return if version < 10 { 8 } else { 16 };
    }

    #[allow(unreachable_code)]
    let mut base = match mode {
        Mode::Numeric => 10,
        Mode::Alphanumeric => 9,
        // Mode::Kanji => 8,
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
pub fn encode_numeric(qrdata: &mut QRData, input: &str) {
    qrdata.push_bits(0b0001, 4);
    qrdata.push_bits(
        input.len(),
        bits_char_count_indicator(qrdata.version.0, Mode::Numeric),
    );

    let input = input.as_bytes();
    for i in 0..(input.len() / 3) {
        let group = (input[i * 3] - b'0') as usize * 100
            + (input[i * 3 + 1] - b'0') as usize * 10
            + (input[i * 3 + 2] - b'0') as usize;
        qrdata.push_bits(group, 10);
    }

    match input.len() % 3 {
        1 => {
            let group = input[input.len() - 1] - b'0';
            qrdata.push_bits(group.into(), 4);
        }
        2 => {
            let group = (input[input.len() - 2] - b'0') * 10 + (input[input.len() - 1] - b'0');
            qrdata.push_bits(group.into(), 7);
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
pub fn encode_alphanumeric(qrdata: &mut QRData, input: &str) {
    qrdata.push_bits(0b0010, 4);
    qrdata.push_bits(
        input.len(),
        bits_char_count_indicator(qrdata.version.0, Mode::Alphanumeric),
    );

    let input = input.as_bytes();

    for i in 0..(input.len() / 2) {
        let group =
            ascii_to_b45(input[i * 2]) as usize * 45 + ascii_to_b45(input[i * 2 + 1]) as usize;
        qrdata.push_bits(group, 11);
    }

    if (input.len() & 1) == 1 {
        qrdata.push_bits(ascii_to_b45(input[input.len() - 1]).into(), 6);
    }
}

// ISO-8859-1 aka first 256 unicode
pub fn encode_byte(qrdata: &mut QRData, input: &str) {
    qrdata.push_bits(0b0100, 4);
    qrdata.push_bits(
        input.len(),
        bits_char_count_indicator(qrdata.version.0, Mode::Byte),
    );
    for c in input.as_bytes() {
        qrdata.push_bits((*c).into(), 8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn get_data_vec(bits: &str) -> Vec<u8> {
        let mut v = Vec::new();

        let mut i = 0;
        let mut num = 0;
        for c in bits.chars() {
            match c {
                '1' => {
                    num += 1 << (7 - i);
                    i += 1;
                }
                '0' => i += 1,
                _ => continue,
            }
            if i == 8 {
                v.push(num);
                num = 0;
                i = 0;
            }
        }

        if i > 0 {
            v.push(num);
        }

        v
    }

    #[test]
    fn encode_numeric_works() {
        let mut qrcode = QRData::new(Version(1));

        encode_numeric(&mut qrcode, "1");
        assert_eq!(qrcode.data, get_data_vec("0001 0000000001 0001"));

        let mut qrcode = QRData::new(Version(1));
        encode_numeric(&mut qrcode, "99");
        assert_eq!(qrcode.data, get_data_vec("0001 0000000010 1100011"));

        let mut qrcode = QRData::new(Version(1));
        encode_numeric(&mut qrcode, "123456");
        assert_eq!(
            qrcode.data,
            get_data_vec("0001 0000000110 0001111011 0111001000")
        );
    }

    #[test]
    fn encode_alphanumeric_works() {
        let mut qrcode = QRData::new(Version(1));

        encode_alphanumeric(&mut qrcode, "1");
        assert_eq!(qrcode.data, get_data_vec("0010 000000001 000001"));

        let mut qrcode = QRData::new(Version(1));
        encode_alphanumeric(&mut qrcode, "99");
        assert_eq!(qrcode.data, get_data_vec("0010 000000010 00110011110"));

        let mut qrcode = QRData::new(Version(1));
        encode_alphanumeric(&mut qrcode, "ABC1::4");
        assert_eq!(
            qrcode.data,
            get_data_vec("0010 000000111 00111001101 01000011101 11111101000 000100")
        );
    }

    #[test]
    fn encode_byte_works() {
        let mut qrcode = QRData::new(Version(1));

        encode_byte(&mut qrcode, "0");
        assert_eq!(qrcode.data, get_data_vec("0100 00000001 00110000"));
    }
}
