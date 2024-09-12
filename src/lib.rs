pub mod constants;
pub mod math;

pub mod data;
pub mod encoding;
pub mod error_correction;

pub mod mask;
pub mod matrix;
pub mod qrcode;

pub mod render;

#[cfg(feature = "wasm")]
mod wasm;

use crate::data::Data;
use crate::qrcode::{Mask, Mode, Version, ECL};
use encoding::get_encoding_mode;
use qrcode::QrCode;
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Debug)]
pub struct QrOptions {
    min_version: Version,
    min_ecl: ECL,
    mode: Option<Mode>,
    mask: Option<Mask>,
    strict_version: bool,
    strict_ecl: bool,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl QrOptions {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new() -> Self {
        QrOptions {
            min_version: Version(1),
            strict_version: false,
            min_ecl: ECL::Low,
            strict_ecl: false,
            mode: None,
            mask: None,
        }
    }
    pub fn min_version(mut self, version: Version) -> Self {
        self.min_version = version;
        self
    }
    pub fn min_ecl(mut self, ecl: ECL) -> Self {
        self.min_ecl = ecl;
        self
    }
    pub fn mode(mut self, mode: Option<Mode>) -> Self {
        self.mode = mode;
        self
    }
    pub fn mask(mut self, mask: Option<Mask>) -> Self {
        self.mask = mask;
        self
    }
    pub fn strict_version(mut self, strict: bool) -> Self {
        self.strict_version = strict;
        self
    }
    pub fn strict_ecl(mut self, strict: bool) -> Self {
        self.strict_ecl = strict;
        self
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QrError {
    InvalidEncoding,
    ExceedsMaxCapacity,
}

pub fn generate(input: &str, qr_options: QrOptions) -> Result<QrCode, QrError> {
    let mut mode = Mode::Byte;

    if let Some(specified) = qr_options.mode {
        if specified != Mode::Byte {
            let lowest = get_encoding_mode(input);
            if (lowest as u8) > (specified as u8) {
                return Err(QrError::InvalidEncoding);
            }
            mode = specified;
        }
    } else {
        mode = get_encoding_mode(input);
    }

    let data = Data::new_verbose(
        input,
        mode,
        qr_options.min_version,
        qr_options.strict_version,
        qr_options.min_ecl,
        qr_options.strict_ecl,
    );

    let data = match data {
        Some(x) => x,
        None => return Err(QrError::ExceedsMaxCapacity),
    };

    let matrix = qrcode::QrCode::new(data, qr_options.mask);

    Ok(matrix)
}
