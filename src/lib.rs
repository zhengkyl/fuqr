pub mod constants;
pub mod math;

pub mod data;
pub mod encoding;
pub mod error_correction;

pub mod mask;
pub mod matrix;
pub mod qr_code;

pub mod bit_info;
pub mod qart;

pub mod render;

#[cfg(feature = "wasm")]
mod wasm;

use crate::data::Data;
use crate::qr_code::{Mask, Mode, Version, ECL};
use encoding::encoding_mode;
use qart::{Qart, WeightPixel};
use qr_code::QrCode;

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

pub fn generate(input: &str, qr_options: &QrOptions) -> Result<QrCode, QrError> {
    match resolve_data(input, qr_options) {
        Ok(data) => Ok(QrCode::new(data, qr_options.mask)),
        Err(err) => Err(err),
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QartError {
    InvalidEncoding,
    ExceedsMaxCapacity,
    InvalidPixelWeights,
}

impl From<QrError> for QartError {
    fn from(value: QrError) -> Self {
        match value {
            QrError::InvalidEncoding => QartError::InvalidEncoding,
            QrError::ExceedsMaxCapacity => QartError::ExceedsMaxCapacity,
        }
    }
}

pub fn generate_qart(
    input: &str,
    qr_options: &QrOptions,
    pixel_weights: &[WeightPixel],
) -> Result<QrCode, QartError> {
    let data = match resolve_data(input, qr_options) {
        Ok(data) => data,
        Err(err) => return Err(err.into()),
    };

    let qr_width = data.version.0 * 4 + 17;
    if pixel_weights.len() != qr_width * qr_width {
        return Err(QartError::InvalidPixelWeights);
    }

    let qart = Qart::new(data, qr_options.mask.unwrap_or(Mask::M0));
    let qr_code = qart.to_qr_code(&pixel_weights);
    Ok(qr_code)
}

fn resolve_data(input: &str, qr_options: &QrOptions) -> Result<Data, QrError> {
    let mut mode = Mode::Byte;

    if let Some(specified) = qr_options.mode {
        if specified != Mode::Byte {
            let lowest = encoding_mode(input);
            if (lowest as u8) > (specified as u8) {
                return Err(QrError::InvalidEncoding);
            }
            mode = specified;
        }
    } else {
        mode = encoding_mode(input);
    }

    let data = Data::new_verbose(
        input,
        mode,
        qr_options.min_version,
        qr_options.strict_version,
        qr_options.min_ecl,
        qr_options.strict_ecl,
    );

    match data {
        Some(x) => Ok(x),
        None => Err(QrError::ExceedsMaxCapacity),
    }
}
