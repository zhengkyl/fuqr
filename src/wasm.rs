#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

use crate::{
    codewords::Codewords,
    data::{Data, Segment},
    matrix::Matrix,
    qrcode::{Mask, Mode, Version, ECL},
    render::svg::render_svg,
};

#[cfg(feature = "wasm")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct SvgOptions {
    mode: Mode,
    margin: usize,
    pixel_size: usize,
    version: Version,
    ecl: ECL,
    mask: Mask,
    module_shape: usize,
    finderPattern: usize,
    alignmentPattern: usize,
}

#[wasm_bindgen]
impl SvgOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        SvgOptions {
            mode: Mode::Byte,
            margin: 2,
            pixel_size: 1,
            version: Version(1),
            ecl: ECL::Low,
            mask: Mask(0),
            module_shape: 0,
            finderPattern: 0,
            alignmentPattern: 0,
        }
    }
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn get_svg(text: &str, options: SvgOptions) -> String {
    // todo, only one segment for now

    let data = Data::new(
        vec![Segment {
            mode: options.mode,
            text,
        }],
        options.version,
    );
    let codewords = Codewords::new(data, options.ecl);
    let matrix = Matrix::new(codewords, options.mask);

    // render_svg(&matrix)
    "".into()
}
