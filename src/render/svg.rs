use crate::matrix::{Matrix, Module};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct SvgBuilder {
    margin: f64,
    unit: f64,
    fg_module_size: f64,
    bg_module_size: f64,
    // module_roundness: f64,
    finder_pattern: FinderPattern,
    finder_roundness: f64,
    foreground: String,
    background: String,
    render_mask: u8, // bits represent module types to render
    scale_mask: u8,  // bits represent module types to scale
    toggle_options: u8,
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub enum Toggle {
    Background,
    Invert,
    FinderForeground,
    FinderBackground,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl SvgBuilder {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new() -> Self {
        SvgBuilder {
            margin: 2.0,
            unit: 1.0,
            fg_module_size: 1.0,
            bg_module_size: 0.0,
            finder_pattern: FinderPattern::Square,
            finder_roundness: 0.0,
            foreground: "#000".into(),
            background: "#fff".into(),
            render_mask: 0b0011_1111,
            scale_mask: 0b0011_1111,
            toggle_options: 0b0000_0101,
        }
    }
    pub fn margin(mut self, margin: f64) -> Self {
        self.margin = margin;
        self
    }
    pub fn unit(mut self, unit: f64) -> Self {
        self.unit = unit;
        self
    }
    pub fn fg_module_size(mut self, module_size: f64) -> Self {
        self.fg_module_size = module_size;
        self
    }
    pub fn bg_module_size(mut self, module_size: f64) -> Self {
        self.bg_module_size = module_size;
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
    pub fn foreground(mut self, foreground: String) -> SvgBuilder {
        self.foreground = foreground;
        self
    }
    pub fn background(mut self, background: String) -> SvgBuilder {
        self.background = background;
        self
    }
    pub fn toggle_render(mut self, module: Module) -> SvgBuilder {
        self.render_mask ^= 1 << (module as u8 / 2);
        self
    }
    pub fn toggle_scale(mut self, module: Module) -> SvgBuilder {
        self.scale_mask ^= 1 << (module as u8 / 2);
        self
    }
    pub fn render(&self, module: Module) -> bool {
        (self.render_mask >> (module as u8 / 2)) & 1 == 1
    }
    pub fn scale(&self, module: Module) -> bool {
        (self.scale_mask >> (module as u8 / 2)) & 1 == 1
    }

    pub fn toggle(mut self, toggle: Toggle) -> SvgBuilder {
        self.toggle_options ^= 1 << toggle as u8;
        self
    }
    pub fn get(&self, option: Toggle) -> bool {
        (self.toggle_options >> option as u8) & 1 == 1
    }
}

pub fn render_svg(matrix: &Matrix, options: SvgBuilder) -> String {
    let full_width = matrix.width as f64 * options.unit + (2.0 * options.margin);

    // todo better initial capacity
    // guestimate, roughly half of pixels are black
    let mut result = String::with_capacity(40 * matrix.width * matrix.width / 2);
    result.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {0} {0}">"#,
        full_width
    ));

    if options.get(Toggle::Background) {
        result.push_str(&format!(
            r#"<rect height="{}" width="{}" fill="{}"/>"#,
            full_width, full_width, options.background
        ));
    }

    if options.bg_module_size > 0.0 {
        result.push_str(&format!("<path fill=\"{}\" d=\"", options.background));
        for x in 0..matrix.width {
            for y in 0..matrix.width {
                let module_type = matrix.get(x, y);

                // finder rendered separately
                if (module_type == Module::FinderOFF)
                // skip ON modules
                || (module_type as u8 & 1 == 1)
                // skip toggled off in render_mask
                || (options.render_mask >> (module_type as u8 / 2)) & 1 == 0
                {
                    continue;
                }

                let module_size = if options.scale(module_type) {
                    options.bg_module_size
                } else {
                    options.unit
                };

                // keep module centered if size != scale
                result.push_str(&format!(
                    "M{},{}h{2}v{2}h-{2}z",
                    x as f64 * options.unit + options.margin + (options.unit - module_size) / 2.0,
                    y as f64 * options.unit + options.margin + (options.unit - module_size) / 2.0,
                    module_size
                ));
            }
        }
        result.push_str("\"/>");
    }

    result.push_str(&format!("<path fill=\"{}\" d=\"", options.foreground));

    for x in 0..matrix.width {
        for y in 0..matrix.width {
            let module_type = matrix.get(x, y);

            // finder rendered separately
            if (module_type == Module::FinderON)
                // skip OFF modules
                || (module_type as u8 & 1 == 0)
                // skip toggled off in render_mask
                || (options.render_mask >> (module_type as u8 / 2)) & 1 == 0
            {
                continue;
            }

            let module_size = if options.scale(module_type) {
                options.fg_module_size
            } else {
                options.unit
            };

            // keep module centered if size != scale
            result.push_str(&format!(
                "M{},{}h{2}v{2}h-{2}z",
                x as f64 * options.unit + options.margin + (options.unit - module_size) / 2.0,
                y as f64 * options.unit + options.margin + (options.unit - module_size) / 2.0,
                module_size
            ));
        }
    }
    result.push_str("\"/>");

    if (options.render_mask >> (Module::FinderON as u8) / 2) & 1 == 1 {
        render_finder(matrix, &options, &mut result);
    }

    result.push_str("</svg>");

    result
}

fn render_finder(matrix: &Matrix, options: &SvgBuilder, result: &mut String) {
    let full_width = matrix.width as f64 * options.unit + (2.0 * options.margin);
    let finder_offset = full_width - 7.0 * options.unit - options.margin;

    if options.get(Toggle::FinderBackground) {
        result.push_str(&format!("<path fill=\"{}\" d=\"", options.background));
        for (x, y) in [
            (options.margin, options.margin),
            (options.margin, finder_offset),
            (finder_offset, options.margin),
        ] {
            match options.finder_pattern {
                FinderPattern::Square => {
                    result.push_str(&square(
                        x - options.unit,
                        y - options.unit,
                        9.0 * options.unit,
                        options.finder_roundness,
                    ));
                    result.push_str(&square_hole(
                        x,
                        y,
                        7.0 * options.unit,
                        options.finder_roundness,
                    ));

                    result.push_str(&square(
                        x + options.unit,
                        y + options.unit,
                        5.0 * options.unit,
                        options.finder_roundness,
                    ));
                    result.push_str(&square_hole(
                        x + 2.0 * options.unit,
                        y + 2.0 * options.unit,
                        3.0 * options.unit,
                        options.finder_roundness,
                    ));
                }
                FinderPattern::Cross => {
                    open_square(
                        result,
                        x - options.unit,
                        y - options.unit,
                        options.unit,
                        5.0 * options.unit,
                        9.0 * options.unit,
                        options.finder_roundness,
                    );
                    open_square(
                        result,
                        x + options.unit,
                        y + options.unit,
                        options.unit,
                        3.0 * options.unit,
                        5.0 * options.unit,
                        options.finder_roundness,
                    );
                }
            }
        }
        result.push_str("\"/>");
    }

    if options.get(Toggle::FinderForeground) {
        result.push_str(&format!("<path fill=\"{}\" d=\"", options.foreground));
        for (x, y) in [
            (options.margin, options.margin),
            (options.margin, finder_offset),
            (finder_offset, options.margin),
        ] {
            match options.finder_pattern {
                FinderPattern::Square => {
                    result.push_str(&square(x, y, 7.0 * options.unit, options.finder_roundness));
                    result.push_str(&square_hole(
                        x + options.unit,
                        y + options.unit,
                        5.0 * options.unit,
                        options.finder_roundness,
                    ));

                    result.push_str(&square(
                        x + 2.0 * options.unit,
                        y + 2.0 * options.unit,
                        3.0 * options.unit,
                        options.finder_roundness,
                    ));
                }
                FinderPattern::Cross => {
                    open_square(
                        result,
                        x,
                        y,
                        options.unit,
                        3.0 * options.unit,
                        7.0 * options.unit,
                        options.finder_roundness,
                    );
                    result.push_str(&square(
                        x + 2.0 * options.unit,
                        y + 2.0 * options.unit,
                        3.0 * options.unit,
                        options.finder_roundness,
                    ))
                }
            }
        }
        result.push_str("\"/>");
    }
}

#[derive(Clone, Copy)]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub enum FinderPattern {
    Square,
    Cross,
}

// https://pomax.github.io/bezierinfo/#arcapproximation
// bottom of section 42
const QUARTER_CIRCLE_K: f64 = 0.551785;

fn square(x: f64, y: f64, width: f64, roundness: f64) -> String {
    if roundness == 0.0 {
        return format!("M{x},{y}h{width}v{width}h-{width}z");
    }

    let radius = width * roundness / 2.0;
    let control = QUARTER_CIRCLE_K * radius;
    let control_inv = radius - control;
    let side = width - 2.0 * radius;
    format!(
        "M{:.2},{:.2}c0,-{control:.2} {control_inv:.2},-{radius:.2} {radius:.2},-{radius:.2} h{side:.2}c{control:.2},0 {radius:.2},{control_inv:.2} {radius:.2},{radius:.2}v{side:.2}c0,{control:.2} -{control_inv:.2},{radius:.2} -{radius:.2},{radius:.2}h-{side:.2}c-{control:.2},0 -{radius:.2},-{control_inv:.2} -{radius:.2},-{radius:.2}z",
        x, y + radius
    )
}
fn square_hole(x: f64, y: f64, width: f64, roundness: f64) -> String {
    if roundness == 0.0 {
        return format!("M{x},{y}v{width}h{width}v-{width}z");
    }

    let radius = width * roundness / 2.0;
    let control = QUARTER_CIRCLE_K * radius;
    let control_inv = radius - control;
    let side = width - 2.0 * radius;
    format!(
        "M{:.2},{:.2}c-{control:.2},0 -{radius:.2},{control_inv:.2} -{radius:.2},{radius:.2} v{side:.2}c0,{control:.2} {control_inv:.2},{radius:.2} {radius:.2},{radius:.2}h{side:.2}c{control:.2},0 {radius:.2},-{control_inv:.2} {radius:.2},-{radius:.2}v-{side:.2}c0,-{control:.2} -{control_inv:.2},-{radius:.2} -{radius:.2},-{radius:.2}z",
        x + radius, y
    )
}

fn open_square(
    result: &mut String,
    x: f64,
    y: f64,
    short_len: f64,
    long_len: f64,
    width: f64,
    roundness: f64,
) {
    let offset = (width - long_len) / 2.0;
    let across = width - short_len;

    if roundness == 0.0 {
        result.push_str(&format!(
            "M{},{}h{long_len}v{short_len}h-{long_len}z",
            x + offset,
            y
        ));

        result.push_str(&format!(
            "M{},{}h{long_len}v{short_len}h-{long_len}z",
            x + offset,
            y + across
        ));

        result.push_str(&format!(
            "M{},{}h{short_len}v{long_len}h-{short_len}z",
            x,
            y + offset
        ));

        result.push_str(&format!(
            "M{},{}h{short_len}v{long_len}h-{short_len}z",
            x + across,
            y + offset
        ));

        return;
    }

    let l_radius = long_len * roundness / 2.0;
    let l_control = QUARTER_CIRCLE_K * l_radius;
    let l_control_inv = l_radius - l_control;

    let max_s_radius = short_len / 2.0;
    let s_radius = if l_radius < max_s_radius {
        l_radius
    } else {
        max_s_radius
    };
    let s_control = QUARTER_CIRCLE_K * s_radius;
    let s_control_inv = s_radius - s_control;

    let short = short_len - 2.0 * s_radius;
    let long = long_len - 2.0 * l_radius;

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
