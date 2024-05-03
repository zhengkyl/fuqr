use crate::qr::QRCode;

#[derive(Clone, Copy)]
pub struct Version(pub u8);

impl Version {
    // todo, debating where to put bound check
    pub fn new(version: u8) -> Self {
        assert!(version >= 1 && version <= 40);
        Version(version)
    }
}

// See Annex D for explaination
// TLDR (18,6) Golay Code, take version, append remainder after polynomial division
// maybe just hardcode this
pub fn version_information(version: usize) -> usize {
    let shifted_version = version << 12;
    let mut dividend = shifted_version;

    while dividend >= 0b1_0000_0000_0000 {
        let mut divisor = 0b1_1111_0010_0101;
        divisor <<= (usize::BITS - dividend.leading_zeros()) - 13; // diff of highest set bit

        dividend ^= divisor;
    }
    shifted_version | dividend
}

pub fn format_information(qrcode: &QRCode) -> u32 {
    let format = ((((qrcode.ecl as u8) << 3) | qrcode.mask) as u32) << 10;
    let mut dividend = format;

    while dividend >= 0b100_0000_0000 {
        let mut divisor = 0b101_0011_0111;
        divisor <<= (32 - dividend.leading_zeros()) - 11;

        dividend ^= divisor;
    }
    (format | dividend) ^ 0b10101_0000010010
}

#[cfg(test)]
mod tests {
    use crate::error_correction::ECL;

    use super::*;
    #[test]
    fn information_works() {
        assert_eq!(version_information(7), 0x07C94);
        assert_eq!(version_information(21), 0x15683);
        assert_eq!(version_information(40), 0x28C69);
    }

    #[test]
    fn format_information_works() {
        let mut qrcode = QRCode {
            sequenced_data: Vec::new(),
            version: Version(1),
            ecl: ECL::Medium,
            mask: 0,
        };

        assert_eq!(format_information(&qrcode), 0x5412);

        qrcode.ecl = ECL::High;
        assert_eq!(format_information(&qrcode), 0x1689);

        qrcode.mask = 7;
        assert_eq!(format_information(&qrcode), 0x083B);
    }
}
