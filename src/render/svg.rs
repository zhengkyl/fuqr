use crate::matrix::{Matrix, Module};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct SvgOptions {
    margin: f64,
    scale: f64,
    module_size: f64,
    // module_roundness: f64,
    finder_pattern: FinderPattern,
    finder_roundness: f64,
    foreground: String,
    background: String,
    render_type: u8, // bits represent module types to render
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl SvgOptions {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new() -> Self {
        SvgOptions {
            margin: 2.0,
            scale: 1.0,
            module_size: 1.0,
            finder_pattern: FinderPattern::Square,
            finder_roundness: 0.0,
            foreground: "#000".into(),
            background: "#fff".into(),
            render_type: 0b0011_1111,
        }
    }
    pub fn margin(mut self, margin: f64) -> Self {
        self.margin = margin;
        self
    }
    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }
    pub fn module_size(mut self, module_size: f64) -> Self {
        self.module_size = module_size;
        self
    }
    pub fn finder_pattern(mut self, finder_pattern: FinderPattern) -> Self {
        self.finder_pattern = finder_pattern;
        self
    }
    pub fn finder_roundness(mut self, finder_roundness: f64) -> Self {
        self.finder_roundness = finder_roundness;
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
    pub fn toggle_render_modules(mut self, module: Module) -> SvgOptions {
        self.render_type ^= 1 << (module as u8 / 2);
        self
    }
    pub fn toggle_invert_modules(mut self) -> SvgOptions {
        self.render_type ^= 1 << 7;
        self
    }
}

#[derive(Clone, Copy)]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub enum FinderPattern {
    Square,
    Cross,
}

pub fn render_svg(matrix: &Matrix, options: SvgOptions) -> String {
    let full_width = matrix.width as f64 * options.scale + (2.0 * options.margin);

    // todo better initial capacity
    // guestimate, roughly half of pixels are black
    let mut result = String::with_capacity(40 * matrix.width * matrix.width / 2);
    result.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {0} {0}" fill="{1}">"#,
        full_width, options.foreground
    ));
    result.push_str(&format!(
        r#"<rect height="{}" width="{}" fill="{}"/>"#,
        full_width, full_width, options.background
    ));
    
    result.push_str("<path d=\"");

    // render inverted, wrap everything in counterclockwise path
    if options.render_type >> 7 == 1 {
        result.push_str("M0,0v");
        result.push_str(&full_width.to_string());
        result.push_str("h");
        result.push_str(&full_width.to_string());
        result.push_str("v-");
        result.push_str(&full_width.to_string());
        result.push_str("z");
    }

    if (options.render_type >> (Module::FinderON as u8) / 2) & 1 == 1 {
        render_finder(matrix, &options, &mut result);
    }

    for x in 0..matrix.width {
        for y in 0..matrix.width {
            let module_type = matrix.get(x, y) as u8;

            // finder rendered separately
            if (module_type == (Module::FinderON as u8) ) 
                // skip OFF modules
                || (module_type & 1 == 0)
                // skip toggled off in render_type
                || (options.render_type >> (module_type / 2)) & 1 == 0
            {
                continue;
            }

            // keep module centered if size != scale
            result.push_str(&format!(
                r#"M{},{}h{2}v{2}h-{2}z"#,
                x as f64 * options.scale
                    + options.margin
                    + (options.scale - options.module_size) / 2.0,
                y as f64 * options.scale
                    + options.margin
                    + (options.scale - options.module_size) / 2.0,
                options.module_size
            ));
        }
    }

    result.push_str("\"/>");

    result.push_str("</svg>");

    result
}


// https://pomax.github.io/bezierinfo/#arcapproximation
// bottom of section 42
const QUARTER_CIRCLE_K: f64 = 0.551785;

fn render_finder(matrix: &Matrix, options: &SvgOptions, result: &mut String) {
    let full_width = matrix.width as f64 * options.scale + (2.0 * options.margin);
    let outer = 7.0 * options.scale;
    let middle = 5.0 * options.scale;
    let inner = 3.0 * options.scale;
    let finder_offset = full_width - outer - options.margin;

    for (x, y) in [
        (options.margin, options.margin),
        (options.margin, finder_offset),
        (finder_offset, options.margin),
    ] {
        match options.finder_pattern {
            FinderPattern::Square => {
                let radius = outer * options.finder_roundness / 2.0;
                let control = QUARTER_CIRCLE_K * radius;
                let control_inv = radius - control;
                let side = outer - 2.0 * radius;
                result.push_str(&format!(
                    "M{:.2},{:.2}c0,-{control:.2} {control_inv:.2},-{radius:.2} {radius:.2},-{radius:.2} h{side:.2}c{control:.2},0 {radius:.2},{control_inv:.2} {radius:.2},{radius:.2}v{side:.2}c0,{control:.2} -{control_inv:.2},{radius:.2} -{radius:.2},{radius:.2}h-{side:.2}c-{control:.2},0 -{radius:.2},-{control_inv:.2} -{radius:.2},-{radius:.2}z",
                    x, y + radius
                ));

                let radius = middle * options.finder_roundness / 2.0;
                let control = QUARTER_CIRCLE_K * radius;
                let control_inv = radius - control;
                let side = middle - 2.0 * radius;
                result.push_str(&format!(
                    "M{:.2},{:.2}c-{control:.2},0 -{radius:.2},{control_inv:.2} -{radius:.2},{radius:.2} v{side:.2}c0,{control:.2} {control_inv:.2},{radius:.2} {radius:.2},{radius:.2}h{side:.2}c{control:.2},0 {radius:.2},-{control_inv:.2} {radius:.2},-{radius:.2}v-{side:.2}c0,-{control:.2} -{control_inv:.2},-{radius:.2} -{radius:.2},-{radius:.2}z",
                    x + options.scale + radius, y + options.scale
                ));

            }
            FinderPattern::Cross => {
                let l_radius = 3.0 * options.scale * options.finder_roundness / 2.0;
                let l_control = QUARTER_CIRCLE_K * l_radius;
                let l_control_inv = l_radius - l_control;

                let max_s_radius = options.scale / 2.0;
                let s_radius = if l_radius < max_s_radius {l_radius} else {max_s_radius};
                let s_control = QUARTER_CIRCLE_K * s_radius;
                let s_control_inv = s_radius - s_control;

                let short = options.scale - 2.0 * s_radius;
                let long = 3.0 * options.scale - 2.0 * l_radius;

                result.push_str(&format!(
                    "M{:.2},{:.2}c0,-{s_control:.2} {l_control_inv:.2},-{s_radius:.2} {l_radius:.2},-{s_radius:.2} h{long:.2}c{l_control:.2},0 {l_radius:.2},{s_control_inv:.2} {l_radius:.2},{s_radius:.2}v{short:.2}c0,{s_control:.2} -{l_control_inv:.2},{s_radius:.2} -{l_radius:.2},{s_radius:.2}h-{long:.2}c-{l_control:.2},0 -{l_radius:.2},-{s_control_inv:.2} -{l_radius:.2},-{s_radius:.2}z",
                    x + 2.0 * options.scale, y + s_radius
                ));

                result.push_str(&format!(
                    "M{:.2},{:.2}c0,-{s_control:.2} {l_control_inv:.2},-{s_radius:.2} {l_radius:.2},-{s_radius:.2} h{long:.2}c{l_control:.2},0 {l_radius:.2},{s_control_inv:.2} {l_radius:.2},{s_radius:.2}v{short:.2}c0,{s_control:.2} -{l_control_inv:.2},{s_radius:.2} -{l_radius:.2},{s_radius:.2}h-{long:.2}c-{l_control:.2},0 -{l_radius:.2},-{s_control_inv:.2} -{l_radius:.2},-{s_radius:.2}z",
                    x + 2.0 * options.scale, y + 6.0 * options.scale + s_radius
                ));

                result.push_str(&format!(
                    "M{:.2},{:.2}c0,-{l_control:.2} {s_control_inv:.2},-{l_radius:.2} {s_radius:.2},-{l_radius:.2} h{short:.2}c{s_control:.2},0 {s_radius:.2},{l_control_inv:.2} {s_radius:.2},{l_radius:.2}v{long:.2}c0,{l_control:.2} -{s_control_inv:.2},{l_radius:.2} -{s_radius:.2},{l_radius:.2}h-{short:.2}c-{s_control:.2},0 -{s_radius:.2},-{l_control_inv:.2} -{s_radius:.2},-{l_radius:.2}z",
                    x , y + 2.0 * options.scale + l_radius
                ));

                result.push_str(&format!(
                    "M{:.2},{:.2}c0,-{l_control:.2} {s_control_inv:.2},-{l_radius:.2} {s_radius:.2},-{l_radius:.2} h{short:.2}c{s_control:.2},0 {s_radius:.2},{l_control_inv:.2} {s_radius:.2},{l_radius:.2}v{long:.2}c0,{l_control:.2} -{s_control_inv:.2},{l_radius:.2} -{s_radius:.2},{l_radius:.2}h-{short:.2}c-{s_control:.2},0 -{s_radius:.2},-{l_control_inv:.2} -{s_radius:.2},-{l_radius:.2}z",
                    x + 6.0 * options.scale, y + 2.0 * options.scale + l_radius
                ));
            }
        }

        
        let radius = inner * options.finder_roundness / 2.0;
        let control = QUARTER_CIRCLE_K * radius;
        let control_inv = radius - control;
        let side = inner - 2.0 * radius;
        result.push_str(&format!(
            "M{:.2},{:.2}c0,-{control:.2} {control_inv:.2},-{radius:.2} {radius:.2},-{radius:.2} h{side:.2}c{control:.2},0 {radius:.2},{control_inv:.2} {radius:.2},{radius:.2}v{side:.2}c0,{control:.2} -{control_inv:.2},{radius:.2} -{radius:.2},{radius:.2}h-{side:.2}c-{control:.2},0 -{radius:.2},-{control_inv:.2} -{radius:.2},-{radius:.2}z",
            x + 2.0 * options.scale, y + 2.0 * options.scale + radius
        ));
    }
}
