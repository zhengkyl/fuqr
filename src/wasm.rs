#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

use crate::{
    codewords::Codewords,
    data::{Data, Segment},
    encode::get_encoding_mode,
    matrix::Matrix,
    qrcode::{Mask, Mode, Version, ECL},
    render::svg::{render_svg, SvgOptions},
};

#[cfg(feature = "wasm")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct QrOptions {
    mode: Mode,
    version: Version,
    ecl: ECL,
    mask: Mask,
}

#[wasm_bindgen]
impl QrOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        QrOptions {
            mode: Mode::Byte,
            version: Version(1),
            ecl: ECL::Low,
            mask: Mask::M0,
        }
    }
    pub fn mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }
    pub fn version(mut self, version: Version) -> Self {
        self.version = version;
        self
    }
    pub fn ecl(mut self, ecl: ECL) -> Self {
        self.ecl = ecl;
        self
    }
    pub fn mask(mut self, mask: Mask) -> Self {
        self.mask = mask;
        self
    }
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn get_svg(input: &str, svg_options: QrOptions, render_options: SvgOptions) -> Option<String> {
    let mode = get_encoding_mode(input);

    let data = Data::new(
        vec![Segment { mode, text: input }],
        svg_options.version,
        svg_options.ecl,
    );

    let data = match data {
        Some(x) => x,
        None => return None,
    };

    let codewords = Codewords::new(data);
    let matrix = Matrix::new(codewords, svg_options.mask);

    Some(render_svg(&matrix, render_options))
}
