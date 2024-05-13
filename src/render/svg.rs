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
    render_mask: u8, // bits represent module types to render
    render_options: u8, // fill-rule, background, invert, negative
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
            render_mask: 0b0011_1111,
            render_options: 0b0000_0001,
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
    pub fn toggle_modules(mut self, module: Module) -> SvgOptions {
        self.render_mask ^= 1 << (module as u8 / 2);
        self
    }
    pub fn toggle_background(mut self) -> SvgOptions {
        self.render_options ^= 1;
        self
    }
    pub fn toggle_invert(mut self) -> SvgOptions {
        self.render_options ^= 1 << 1;
        self
    }
    pub fn toggle_negative(mut self) -> SvgOptions {
        self.render_options ^= 1 << 2;
        self
    }
    pub fn background_set(&self) -> bool {
        self.render_options & 1 == 1
    }
    pub fn invert_set(&self) -> bool {
        self.render_options >> 1 & 1== 1
    }
    pub fn negative_set(&self) -> bool {
        self.render_options >> 2 & 1 == 1
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
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {0} {0}">"#,
        full_width
    ));

    if options.background_set() {
        result.push_str(&format!(
            r#"<rect height="{}" width="{}" fill="{}"/>"#,
            full_width, full_width, options.background
        ));
    }
    
    result.push_str(&format!("<path fill=\"{}\" d=\"", options.foreground ));

    // render inverted, wrap everything in counterclockwise path
    if options.invert_set() {
        result.push_str("M0,0v");
        result.push_str(&full_width.to_string());
        result.push_str("h");
        result.push_str(&full_width.to_string());
        result.push_str("v-");
        result.push_str(&full_width.to_string());
        result.push_str("z");
    }

    if (options.render_mask >> (Module::FinderON as u8) / 2) & 1 == 1 {
        render_finder(matrix, &options, &mut result);
    }

    for x in 0..matrix.width {
        for y in 0..matrix.width {
            let module_type = matrix.get(x, y) as u8 ^ (options.render_options >> 2) & 1;

            // finder rendered separately
            if (module_type == (Module::FinderON as u8) ) 
                // skip OFF modules
                || (module_type & 1 == 0)
                // skip toggled off in render_type
                || (options.render_mask >> (module_type / 2)) & 1 == 0
            {
                continue;
            }

            // keep module centered if size != scale
            result.push_str(&format!(
                "M{},{}h{2}v{2}h-{2}z",
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
    
    if options.negative_set() {
        for (x, y) in [
            (options.margin, options.margin),
            (options.margin, finder_offset),
            (finder_offset, options.margin),
        ] {
            match options.finder_pattern {
                FinderPattern::Square => {
                result.push_str(&square(x - options.scale, y - options.scale, 9.0 * options.scale, options.finder_roundness));
                result.push_str(&square_hole(x, y, outer, options.finder_roundness));
                result.push_str(&square(x + options.scale, y + options.scale, middle, options.finder_roundness));
                result.push_str(&square_hole(x + 2.0 * options.scale, y + 2.0 * options.scale, inner, options.finder_roundness))
                },
                FinderPattern::Cross => {
                    // todo long_len could be passed in but... idk maybe too much choice
                    open_square(result, x - options.scale, y - options.scale, options.scale, 5.0 * options.scale, 9.0 * options.scale, options.finder_roundness);
                    open_square(result, x + options.scale, y + options.scale, options.scale, 3.0 * options.scale, 5.0 * options.scale, options.finder_roundness);
                },
            }
        }
        return
    }


    for (x, y) in [
        (options.margin, options.margin),
        (options.margin, finder_offset),
        (finder_offset, options.margin),
    ] {
        match options.finder_pattern {
            FinderPattern::Square => {
                result.push_str(&square(x, y, outer, options.finder_roundness));
                result.push_str(&square_hole(x + options.scale, y + options.scale, middle, options.finder_roundness));
        result.push_str(&square(x + 2.0 * options.scale, y + 2.0 * options.scale, inner, options.finder_roundness))
            }
            FinderPattern::Cross => {
                open_square(result, x, y, options.scale, 3.0 * options.scale, 7.0 * options.scale, options.finder_roundness);
        result.push_str(&square(x + 2.0 * options.scale, y + 2.0 * options.scale, inner, options.finder_roundness))
            }
        }

    }
}


fn square(x:f64, y:f64, width: f64, roundness: f64) -> String {
    let radius = width * roundness / 2.0;
    let control = QUARTER_CIRCLE_K * radius;
    let control_inv = radius - control;
    let side = width - 2.0 * radius;
    format!(
        "M{:.2},{:.2}c0,-{control:.2} {control_inv:.2},-{radius:.2} {radius:.2},-{radius:.2} h{side:.2}c{control:.2},0 {radius:.2},{control_inv:.2} {radius:.2},{radius:.2}v{side:.2}c0,{control:.2} -{control_inv:.2},{radius:.2} -{radius:.2},{radius:.2}h-{side:.2}c-{control:.2},0 -{radius:.2},-{control_inv:.2} -{radius:.2},-{radius:.2}z",
        x, y + radius
    )
}
fn square_hole(x:f64, y:f64, width: f64, roundness: f64) -> String {
    let radius = width * roundness / 2.0;
    let control = QUARTER_CIRCLE_K * radius;
    let control_inv = radius - control;
    let side = width - 2.0 * radius;
    format!(
        "M{:.2},{:.2}c-{control:.2},0 -{radius:.2},{control_inv:.2} -{radius:.2},{radius:.2} v{side:.2}c0,{control:.2} {control_inv:.2},{radius:.2} {radius:.2},{radius:.2}h{side:.2}c{control:.2},0 {radius:.2},-{control_inv:.2} {radius:.2},-{radius:.2}v-{side:.2}c0,-{control:.2} -{control_inv:.2},-{radius:.2} -{radius:.2},-{radius:.2}z",
        x + radius, y
    )
}

fn open_square(result: &mut String, x:f64, y:f64, short_len: f64, long_len: f64, width: f64, roundness: f64)  {
    let l_radius = long_len * roundness / 2.0;
    let l_control = QUARTER_CIRCLE_K * l_radius;
    let l_control_inv = l_radius - l_control;

    let max_s_radius = short_len / 2.0;
    let s_radius = if l_radius < max_s_radius {l_radius} else {max_s_radius};
    let s_control = QUARTER_CIRCLE_K * s_radius;
    let s_control_inv = s_radius - s_control;

    let short = short_len - 2.0 * s_radius;
    let long = long_len - 2.0 * l_radius;

    let offset = (width - long_len) / 2.0;
    let across = width - short_len;

    result.push_str(&format!(
        "M{:.2},{:.2}c0,-{s_control:.2} {l_control_inv:.2},-{s_radius:.2} {l_radius:.2},-{s_radius:.2} h{long:.2}c{l_control:.2},0 {l_radius:.2},{s_control_inv:.2} {l_radius:.2},{s_radius:.2}v{short:.2}c0,{s_control:.2} -{l_control_inv:.2},{s_radius:.2} -{l_radius:.2},{s_radius:.2}h-{long:.2}c-{l_control:.2},0 -{l_radius:.2},-{s_control_inv:.2} -{l_radius:.2},-{s_radius:.2}z",
        x + offset, y + s_radius
    ));

    result.push_str(&format!(
        "M{:.2},{:.2}c0,-{s_control:.2} {l_control_inv:.2},-{s_radius:.2} {l_radius:.2},-{s_radius:.2} h{long:.2}c{l_control:.2},0 {l_radius:.2},{s_control_inv:.2} {l_radius:.2},{s_radius:.2}v{short:.2}c0,{s_control:.2} -{l_control_inv:.2},{s_radius:.2} -{l_radius:.2},{s_radius:.2}h-{long:.2}c-{l_control:.2},0 -{l_radius:.2},-{s_control_inv:.2} -{l_radius:.2},-{s_radius:.2}z",
        x + offset, y + across + s_radius
    ));

    result.push_str(&format!(
        "M{:.2},{:.2}c0,-{l_control:.2} {s_control_inv:.2},-{l_radius:.2} {s_radius:.2},-{l_radius:.2} h{short:.2}c{s_control:.2},0 {s_radius:.2},{l_control_inv:.2} {s_radius:.2},{l_radius:.2}v{long:.2}c0,{l_control:.2} -{s_control_inv:.2},{l_radius:.2} -{s_radius:.2},{l_radius:.2}h-{short:.2}c-{s_control:.2},0 -{s_radius:.2},-{l_control_inv:.2} -{s_radius:.2},-{l_radius:.2}z",
        x , y + offset + l_radius
    ));

    result.push_str(&format!(
        "M{:.2},{:.2}c0,-{l_control:.2} {s_control_inv:.2},-{l_radius:.2} {s_radius:.2},-{l_radius:.2} h{short:.2}c{s_control:.2},0 {s_radius:.2},{l_control_inv:.2} {s_radius:.2},{l_radius:.2}v{long:.2}c0,{l_control:.2} -{s_control_inv:.2},{l_radius:.2} -{s_radius:.2},{l_radius:.2}h-{short:.2}c-{s_control:.2},0 -{s_radius:.2},-{l_control_inv:.2} -{s_radius:.2},-{l_radius:.2}z",
        x + across, y + offset + l_radius
    ));
}