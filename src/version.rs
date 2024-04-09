struct Version(u8);

impl Version {
    // todo, debating where to put bound check
    fn new(version: u8) -> Self {
        Version(version)
    }
    // fn new(version: u8) -> Option<Self> {
    //   Version(version)
    // }

    // See Annex D for explaination
    // TLDR (18,6) Golay Code, take version, append remainder after polynomial division
    // maybe just hardcode this
    fn information(self) -> u32 {
        let version = (self.0 as u32) << 12;
        let mut dividend = version;

        while dividend >= 0b1_0000_0000_0000 {
            let mut divisor = 0b1_1111_0010_0101;
            divisor <<= (32 - dividend.leading_zeros()) - 13; // diff of highest set bit

            dividend = dividend ^ divisor;
        }
        version | dividend
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn information_works() {
        assert_eq!(Version::new(7).information(), 0x07C94);
        assert_eq!(Version::new(21).information(), 0x15683);
        assert_eq!(Version::new(40).information(), 0x28C69);
    }
}
