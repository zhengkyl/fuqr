#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

use crate::{
    codewords::Codewords,
    data::{Data, Segment},
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
pub fn get_svg(text: &str, svg_options: QrOptions, render_options: SvgOptions) -> String {
    // todo, only one segment for now

    let data = Data::new(
        vec![Segment {
            mode: svg_options.mode,
            text,
        }],
        svg_options.version,
    );
    let codewords = Codewords::new(data, svg_options.ecl);
    let matrix = Matrix::new(codewords, svg_options.mask);

    render_svg(&matrix, render_options)
}
