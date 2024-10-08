use crate::{
    data::Data,
    qr_code::{Mode, Version},
};

pub fn encoding_mode(input: &str) -> Mode {
    let mut mode = Mode::Numeric;
    for b in input.bytes() {
        if b >= b'0' && b <= b'9' {
            continue;
        }
        if byte_to_b45(b) < 45 {
            mode = Mode::Alphanumeric;
        } else {
            mode = Mode::Byte;
            break;
        }
    }
    mode
}

// input fits in u8 b/c numeric
pub fn encode_numeric(data: &mut Data, input: &str) {
    data.bits.push_n(0b0001, 4);
    data.bits
        .push_n(input.len(), num_cci_bits(data.version, Mode::Numeric));

    let input = input.as_bytes();
    for i in 0..(input.len() / 3) {
        let group = (input[i * 3] - b'0') as usize * 100
            + (input[i * 3 + 1] - b'0') as usize * 10
            + (input[i * 3 + 2] - b'0') as usize;
        data.bits.push_n(group, 10);
    }

    match input.len() % 3 {
        1 => {
            let group = input[input.len() - 1] - b'0';
            data.bits.push_n(group.into(), 4);
        }
        2 => {
            let group = (input[input.len() - 2] - b'0') * 10 + (input[input.len() - 1] - b'0');
            data.bits.push_n(group.into(), 7);
        }
        _ => (),
    }
}

pub fn encode_alphanumeric(qrdata: &mut Data, input: &str) {
    qrdata.bits.push_n(0b0010, 4);
    qrdata.bits.push_n(
        input.len(),
        num_cci_bits(qrdata.version, Mode::Alphanumeric),
    );

    let input = input.as_bytes();

    for i in 0..(input.len() / 2) {
        let group =
            byte_to_b45(input[i * 2]) as usize * 45 + byte_to_b45(input[i * 2 + 1]) as usize;
        qrdata.bits.push_n(group, 11);
    }

    if (input.len() & 1) == 1 {
        qrdata
            .bits
            .push_n(byte_to_b45(input[input.len() - 1]).into(), 6);
    }
}

pub fn encode_byte(qrdata: &mut Data, input: &str) {
    qrdata.bits.push_n(0b0100, 4);
    qrdata
        .bits
        .push_n(input.len(), num_cci_bits(qrdata.version, Mode::Byte));
    for c in input.as_bytes() {
        qrdata.bits.push_n((*c).into(), 8);
    }
}

pub fn num_cci_bits(version: Version, mode: Mode) -> usize {
    if mode == Mode::Byte {
        return if version.0 < 10 { 8 } else { 16 };
    }

    #[allow(unreachable_code)]
    let mut base = match mode {
        Mode::Numeric => 10,
        Mode::Alphanumeric => 9,
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

fn byte_to_b45(c: u8) -> u8 {
    match c {
        x if x >= b'A' && x <= b'Z' => x - b'A' + 10,
        b':' => 44,
        x if x >= b'0' && x <= b'9' => x - b'0',
        b' ' => 36,
        b'$' => 37,
        b'%' => 38,
        b'*' => 39,
        b'+' => 40,
        b'-' => 41,
        b'.' => 42,
        b'/' => 43,
        // All other values are invalid
        // can use byte_to_b45 < 45 if validation needed
        _ => 255,
    }
}

#[cfg(test)]
mod tests {
    use crate::{data::BitVec, qr_code::ECL};

    use super::*;
    fn get_data_bits(bits: &str) -> BitVec {
        let mut v = BitVec::new();

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
                v.push_n(num, 1);
                num = 0;
                i = 0;
            }
        }

        if i > 0 {
            v.push_n(num, 1);
        }

        v
    }

    #[test]
    fn encode_numeric_works() {
        let data = Data::new("1", Mode::Numeric, Version(1), ECL::Low).unwrap();

        assert_eq!(data.bits, get_data_bits("0001 0000000001 0001"));

        let data = Data::new("99", Mode::Numeric, Version(1), ECL::Low).unwrap();
        assert_eq!(data.bits, get_data_bits("0001 0000000010 1100011"));

        let data = Data::new("123456", Mode::Numeric, Version(1), ECL::Low).unwrap();
        assert_eq!(
            data.bits,
            get_data_bits("0001 0000000110 0001111011 0111001000")
        );
    }

    #[test]
    fn encode_alphanumeric_works() {
        let data = Data::new("1", Mode::Alphanumeric, Version(1), ECL::Low).unwrap();
        assert_eq!(data.bits, get_data_bits("0010 000000001 000001"));

        let data = Data::new("99", Mode::Alphanumeric, Version(1), ECL::Low).unwrap();
        assert_eq!(data.bits, get_data_bits("0010 000000010 00110011110"));

        let data = Data::new("ABC1::4", Mode::Alphanumeric, Version(1), ECL::Low).unwrap();
        assert_eq!(
            data.bits,
            get_data_bits("0010 000000111 00111001101 01000011101 11111101000 000100")
        );
    }

    #[test]
    fn encode_byte_works() {
        let data = Data::new("0", Mode::Byte, Version(1), ECL::Low).unwrap();

        assert_eq!(data.bits, get_data_bits("0100 00000001 00110000"));
    }
}
