#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy, PartialEq, Eq)]
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

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy, PartialEq)]
pub enum ECL {
    Low,      // 7
    Medium,   // 15
    Quartile, // 25
    High,     // 30
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy)]
pub struct Version(pub usize);

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Version {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new(version: usize) -> Self {
        assert!(version >= 1 && version <= 40);
        Version(version)
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy)]
pub enum Mask {
    M0,
    M1,
    M2,
    M3,
    M4,
    M5,
    M6,
    M7,
}
