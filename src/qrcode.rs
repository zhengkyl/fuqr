pub struct QRCode {
    pub codewords: Vec<u8>,
    pub version: Version,
    pub ecl: ECL,
    pub mask: Mask,
}

#[derive(PartialEq, Eq)]
pub enum Mode {
    Numeric,
    Alphanumeric,
    Byte,
    // todo probably won't do
    // Kanji,
    // ECI,
    // StructuredAppend,
    // FNC1,
}

// values used while encoding format
#[derive(Clone, Copy, PartialEq)]
pub enum ECL {
    Low = 1,      // 7
    Medium = 0,   // 15
    Quartile = 3, // 25
    High = 2,     // 30
}

#[derive(Clone, Copy)]
pub struct Version(pub usize);

impl Version {
    pub fn new(version: usize) -> Self {
        assert!(version >= 1 && version <= 40);
        Version(version)
    }
}

#[derive(Clone, Copy)]
pub struct Mask(pub u8);

impl Mask {
    pub fn new(mask: u8) -> Self {
        assert!(mask < 8);
        Mask(mask)
    }
}
