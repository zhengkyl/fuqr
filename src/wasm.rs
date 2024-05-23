#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

use crate::{
    data::{Data, Segment},
    encoding::get_encoding_mode,
    matrix::Matrix,
    qrcode::{Mask, Mode, Version, ECL},
    render::svg::{render_svg, SvgBuilder},
};

#[cfg(feature = "wasm")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct QrOptions {
    min_version: Version,
    min_ecl: ECL,
    mode: Option<Mode>,
    mask: Option<Mask>,
}

#[wasm_bindgen]
impl QrOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        QrOptions {
            min_version: Version(1),
            min_ecl: ECL::Low,
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
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct SvgResult {
    #[wasm_bindgen(getter_with_clone)]
    pub svg: String,
    // These may not match input, so return final values
    pub mode: Mode,
    pub ecl: ECL,
    pub version: Version,
    pub mask: Mask,
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub enum SvgError {
    InvalidEncoding,
    ExceedsMaxCapacity,
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn get_svg(
    input: &str,
    qr_options: QrOptions,
    render_options: SvgBuilder,
) -> Result<SvgResult, SvgError> {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    let mut mode = Mode::Byte;

    if let Some(specified) = qr_options.mode {
        if specified != Mode::Byte {
            let lowest = get_encoding_mode(input);
            if (lowest as u8) > (specified as u8) {
                return Err(SvgError::InvalidEncoding);
            }
            mode = specified;
        }
    } else {
        mode = get_encoding_mode(input);
    }

    let data = Data::new(
        vec![Segment { mode, text: input }],
        qr_options.min_version,
        qr_options.min_ecl,
    );

    let data = match data {
        Some(x) => x,
        None => return Err(SvgError::ExceedsMaxCapacity),
    };

    let matrix = Matrix::new(data, qr_options.mask);

    Ok(SvgResult {
        svg: render_svg(&matrix, render_options),
        mode,
        ecl: matrix.ecl,
        version: matrix.version,
        mask: matrix.mask,
    })
}
