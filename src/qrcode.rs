#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg_attr(feature = "wasm", wasm_bindgen)]
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

pub const NUM_DATA_MODULES: [usize; 41] = num_data_modules();

const fn num_data_modules() -> [usize; 41] {
    let mut table = [0; 41];

    let mut version = 1;
    while version <= 40 {
        let width = 4 * version + 17;
        let mut modules = width * width;

        modules -= 64 * 3; // finder markers + separator
        modules -= 31; // format
        modules -= 2 * (width - 16); // timing

        let (align, overlap) = match version {
            1 => (0, 0),
            x if x <= 6 => (1, 0),
            x if x <= 13 => (6, 2),
            x if x <= 20 => (13, 4),
            x if x <= 27 => (22, 6),
            x if x <= 34 => (33, 8),
            x if x <= 40 => (46, 10),
            _ => unreachable!(),
        };
        modules -= align * 25;
        modules += overlap * 5;

        if version >= 7 {
            modules -= 36; // 2 version
        }

        table[version] = modules;
        version += 1;
    }
    table
}
