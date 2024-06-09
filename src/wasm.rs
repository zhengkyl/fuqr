use wasm_bindgen::prelude::*;

use crate::{
    data::Data,
    encoding::get_encoding_mode,
    matrix::{Margin, Matrix},
    qrcode::{Mask, Mode, Version, ECL},
    render::{svg::render_svg, RenderData, Toggle},
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
}

#[wasm_bindgen]
pub struct SvgOptions {
    margin: f64,
    unit: u8,
    foreground: String,
    background: String,
    scale_x_matrix: Vec<u8>, // scale x 0-200%
    scale_y_matrix: Vec<u8>, // scale y 0-200%
    toggle_options: u8,
}

#[wasm_bindgen]
impl SvgOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        SvgOptions {
            margin: 2.0,
            unit: 1,
            foreground: "#000".into(),
            background: "#fff".into(),
            scale_x_matrix: Vec::new(),
            scale_y_matrix: Vec::new(),
            toggle_options: 0,
        }
        .toggle(Toggle::Background)
        .toggle(Toggle::ForegroundPixels)
    }
    pub fn margin(mut self, margin: f64) -> SvgOptions {
        self.margin = margin;
        self
    }
    pub fn unit(mut self, unit: u8) -> SvgOptions {
        self.unit = unit;
        self
    }
    pub fn foreground(mut self, foreground: String) -> SvgOptions {
        self.foreground = foreground;
        self
    }
    pub fn background(mut self, background: String) -> SvgOptions {
        self.background = background;
        self
    }
    pub fn scale_x_matrix(mut self, scale_matrix: Vec<u8>) -> SvgOptions {
        self.scale_x_matrix = scale_matrix;
        self
    }
    pub fn scale_y_matrix(mut self, scale_matrix: Vec<u8>) -> SvgOptions {
        self.scale_y_matrix = scale_matrix;
        self
    }
    pub fn scale_matrix(mut self, scale_matrix: Vec<u8>) -> SvgOptions {
        // I don't think it's worth worrying about, esp b/c >99% qrcodes are small
        self.scale_x_matrix = scale_matrix;
        self.scale_y_matrix = self.scale_x_matrix.clone();
        self
    }
    pub fn toggle(mut self, toggle: Toggle) -> SvgOptions {
        self.toggle_options ^= 1 << toggle as u8;
        self
    }
}

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

    let data = Data::new(input, mode, qr_options.min_version, qr_options.min_ecl);

    let data = match data {
        Some(x) => x,
        None => return Err(QrError::ExceedsMaxCapacity),
    };

    let matrix = Matrix::new(data, qr_options.mask, qr_options.margin);

    Ok(matrix)
}

// TODO dear kyle, basically do this for everything
pub fn iterate_timing(matrix: Matrix, f: js_sys::Function) {}

#[wasm_bindgen]
pub fn get_svg(
    input: &str,
    qr_options: QrOptions,
    svg_options: SvgOptions,
) -> Result<SvgResult, QrError> {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    let matrix = match get_matrix(input, qr_options) {
        Ok(m) => m,
        Err(e) => return Err(e),
    };

    let render = RenderData::new(&matrix)
        .foreground(svg_options.foreground)
        .background(svg_options.background)
        .scale_x_matrix(svg_options.scale_x_matrix)
        .scale_y_matrix(svg_options.scale_y_matrix)
        .unit(svg_options.unit)
        .toggle_options(svg_options.toggle_options);

    Ok(SvgResult {
        svg: render_svg(&render),
        mode: matrix.mode,
        ecl: matrix.ecl,
        version: matrix.version,
        mask: matrix.mask,
    })
}
