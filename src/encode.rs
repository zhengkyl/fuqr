use crate::{
    data::Data,
    qrcode::{Mask, Mode, Version, ECL},
};

// input fits in u8 b/c numeric
pub fn encode_numeric(qrdata: &mut Data, input: &str) {
    qrdata.push_bits(0b0001, 4);
    qrdata.push_bits(
        input.len(),
        bits_char_count_indicator(qrdata.version, Mode::Numeric),
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

pub fn encode_alphanumeric(qrdata: &mut Data, input: &str) {
    qrdata.push_bits(0b0010, 4);
    qrdata.push_bits(
        input.len(),
        bits_char_count_indicator(qrdata.version, Mode::Alphanumeric),
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
pub fn encode_byte(qrdata: &mut Data, input: &str) {
    qrdata.push_bits(0b0100, 4);
    qrdata.push_bits(
        input.len(),
        bits_char_count_indicator(qrdata.version, Mode::Byte),
    );
    for c in input.as_bytes() {
        qrdata.push_bits((*c).into(), 8);
    }
}

// maybe just hardcode this
pub fn version_information(version: Version) -> usize {
    let shifted_version = version.0 << 12;
    let mut dividend = shifted_version;

    while dividend >= 0b1_0000_0000_0000 {
        let mut divisor = 0b1_1111_0010_0101;
        divisor <<= (usize::BITS - dividend.leading_zeros()) - 13; // diff of highest set bit

        dividend ^= divisor;
    }
    shifted_version | dividend
}

pub fn format_information(ecl: ECL, mask: Mask) -> u32 {
    let format = ((((ecl as u8) << 3) | mask.0) as u32) << 10;
    let mut dividend = format;

    while dividend >= 0b100_0000_0000 {
        let mut divisor = 0b101_0011_0111;
        divisor <<= (32 - dividend.leading_zeros()) - 11;

        dividend ^= divisor;
    }
    (format | dividend) ^ 0b10101_0000010010
}

fn bits_char_count_indicator(version: Version, mode: Mode) -> usize {
    if mode == Mode::Byte {
        return if version.0 < 10 { 8 } else { 16 };
    }

    #[allow(unreachable_code)]
    let mut base = match mode {
        Mode::Numeric => 10,
        Mode::Alphanumeric => 9,
        // Mode::Kanji => 8,
        _ => unreachable!("Unknown mode"),
    };
    if version.0 > 9 {
        base += 2
    }
    if version.0 > 26 {
        base += 2
    }
    base
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

#[cfg(test)]
mod tests {
    use crate::data::Segment;

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
        let data = Data::new(
            vec![Segment {
                mode: Mode::Numeric,
                text: "1",
            }],
            Version(1),
        );

        assert_eq!(data.value, get_data_vec("0001 0000000001 0001"));

        let data = Data::new(
            vec![Segment {
                mode: Mode::Numeric,
                text: "99",
            }],
            Version(1),
        );
        assert_eq!(data.value, get_data_vec("0001 0000000010 1100011"));

        let data = Data::new(
            vec![Segment {
                mode: Mode::Numeric,
                text: "123456",
            }],
            Version(1),
        );
        assert_eq!(
            data.value,
            get_data_vec("0001 0000000110 0001111011 0111001000")
        );
    }

    #[test]
    fn encode_alphanumeric_works() {
        let data = Data::new(
            vec![Segment {
                mode: Mode::Alphanumeric,
                text: "1",
            }],
            Version(1),
        );
        assert_eq!(data.value, get_data_vec("0010 000000001 000001"));

        let data = Data::new(
            vec![Segment {
                mode: Mode::Alphanumeric,
                text: "99",
            }],
            Version(1),
        );
        assert_eq!(data.value, get_data_vec("0010 000000010 00110011110"));

        let data = Data::new(
            vec![Segment {
                mode: Mode::Alphanumeric,
                text: "ABC1::4",
            }],
            Version(1),
        );
        assert_eq!(
            data.value,
            get_data_vec("0010 000000111 00111001101 01000011101 11111101000 000100")
        );
    }

    #[test]
    fn encode_byte_works() {
        let data = Data::new(
            vec![Segment {
                mode: Mode::Byte,
                text: "0",
            }],
            Version(1),
        );

        assert_eq!(data.value, get_data_vec("0100 00000001 00110000"));
    }

    #[test]
    fn information_works() {
        assert_eq!(version_information(Version::new(7)), 0x07C94);
        assert_eq!(version_information(Version::new(21)), 0x15683);
        assert_eq!(version_information(Version::new(40)), 0x28C69);
    }

    #[test]
    fn format_information_works() {
        assert_eq!(format_information(ECL::Medium, Mask::new(0)), 0x5412);
        assert_eq!(format_information(ECL::High, Mask::new(0)), 0x1689);
        assert_eq!(format_information(ECL::High, Mask::new(7)), 0x083B);
    }
}
