use wasm_bindgen::prelude::*;

use crate::{
    data::Data,
    encoding::get_encoding_mode,
    matrix::{Margin, Matrix},
    qrcode::{Mask, Mode, Version, ECL},
};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub struct QrOptions {
    min_version: Version,
    min_ecl: ECL,
    mode: Option<Mode>,
    mask: Option<Mask>,
    margin: Margin,
    strict_version: bool,
    strict_ecl: bool,
}

#[wasm_bindgen]
impl QrOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        QrOptions {
            min_version: Version(1),
            strict_version: false,
            min_ecl: ECL::Low,
            strict_ecl: false,
            mode: None,
            mask: None,
            margin: Margin::new(2),
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
    pub fn margin(mut self, margin: Margin) -> Self {
        self.margin = margin;
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

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub enum QrError {
    InvalidEncoding,
    ExceedsMaxCapacity,
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn get_matrix(input: &str, qr_options: QrOptions) -> Result<Matrix, QrError> {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

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

    let matrix = Matrix::new(data, qr_options.mask, qr_options.margin);

    Ok(matrix)
}
