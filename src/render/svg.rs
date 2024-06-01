use std::rc::Rc;

use crate::matrix::Matrix;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

pub struct SvgBuilder<'a> {
    matrix: &'a Matrix,
    margin: f64,
    unit: f64,
    foreground: String,
    background: String,
    scale_x_matrix: Rc<Vec<u8>>, // scale x 0-200%
    scale_y_matrix: Rc<Vec<u8>>, // scale y 0-200%
    toggle_options: u8,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub enum Toggle {
    Background,
    BackgroundPixels,
    ForegroundPixels,
}

impl<'a> SvgBuilder<'a> {
    pub fn new(matrix: &'a Matrix) -> Self {
        let default_margin = 2;
        let full_width = matrix.width * 4 + 17 + 2 * (default_margin);

        let scale_x_matrix = Rc::new(vec![100; full_width * full_width]);
        let scale_y_matrix = scale_x_matrix.clone();

        SvgBuilder {
            matrix,
            margin: default_margin as f64,
            unit: 1.0,
            foreground: "#000".into(),
            background: "#fff".into(),
            scale_x_matrix,
            scale_y_matrix,
            toggle_options: 0,
        }
        .toggle(Toggle::Background)
        .toggle(Toggle::ForegroundPixels)
    }
    pub fn margin(mut self, margin: f64) -> Self {
        self.margin = margin;
        self
    }
    pub fn unit(mut self, unit: f64) -> Self {
        self.unit = unit;
        self
    }
    pub fn foreground(mut self, foreground: String) -> SvgBuilder<'a> {
        self.foreground = foreground;
        self
    }
    pub fn background(mut self, background: String) -> SvgBuilder<'a> {
        self.background = background;
        self
    }
    pub fn scale_x_matrix(mut self, scale_matrix: Vec<u8>) -> SvgBuilder<'a> {
        self.scale_x_matrix = Rc::new(scale_matrix);
        self
    }
    pub fn scale_y_matrix(mut self, scale_matrix: Vec<u8>) -> SvgBuilder<'a> {
        self.scale_y_matrix = Rc::new(scale_matrix);
        self
    }
    pub fn scale_matrix(mut self, scale_matrix: Vec<u8>) -> SvgBuilder<'a> {
        self.scale_x_matrix = Rc::new(scale_matrix);
        self.scale_y_matrix = Rc::clone(&self.scale_x_matrix);
        self
    }
    pub fn toggle_options(mut self, toggle_options: u8) -> SvgBuilder<'a> {
        self.toggle_options = toggle_options;
        self
    }
    pub fn toggle(mut self, toggle: Toggle) -> SvgBuilder<'a> {
        self.toggle_options ^= 1 << toggle as u8;
        self
    }
    pub fn get(&self, option: Toggle) -> bool {
        (self.toggle_options >> option as u8) & 1 == 1
    }

    pub fn render_svg(&self) -> String {
        let full_width = self.matrix.width as f64 * self.unit + (2.0 * self.margin);

        // TODO better initial capacity
        // guestimate, roughly half of pixels are black
        let mut result = String::with_capacity(40 * self.matrix.width * self.matrix.width / 2);
        result.push_str(&format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {0} {0}">"#,
            full_width
        ));

        if self.get(Toggle::Background) {
            result.push_str(&format!(
                r#"<rect height="{}" width="{}" fill="{}"/>"#,
                full_width, full_width, self.background
            ));
        }

        if self.get(Toggle::BackgroundPixels) {
            result.push_str(&format!("<path fill=\"{}\" d=\"", self.background));
            for x in 0..self.matrix.width {
                for y in 0..self.matrix.width {
                    let x_scale = self.scale_x_matrix[y * self.matrix.width + x];
                    let y_scale = self.scale_y_matrix[y * self.matrix.width + x];

                    let module_type = self.matrix.get(x, y);

                    // skip ON modules
                    if (module_type as u8 & 1 == 1) || x_scale == 0 || y_scale == 0 {
                        continue;
                    }

                    let x_module_size = (x_scale as f64) / 100.0 * self.unit;
                    let y_module_size = (y_scale as f64) / 100.0 * self.unit;

                    // keep module centered if size != scale
                    result.push_str(&format!(
                        "M{},{}h{}v{}h-{}z",
                        x as f64 * self.unit + self.margin + (self.unit - x_module_size) / 2.0,
                        y as f64 * self.unit + self.margin + (self.unit - y_module_size) / 2.0,
                        x_module_size,
                        y_module_size,
                        x_module_size
                    ));
                }
            }
            result.push_str("\"/>");
        }

        if self.get(Toggle::ForegroundPixels) {
            result.push_str(&format!("<path fill=\"{}\" d=\"", self.foreground));

            for x in 0..self.matrix.width {
                for y in 0..self.matrix.width {
                    let x_scale = self.scale_x_matrix[y * self.matrix.width + x];
                    let y_scale = self.scale_y_matrix[y * self.matrix.width + x];

                    let module_type = self.matrix.get(x, y);

                    // skip OFF modules
                    if (module_type as u8 & 1 == 0) || x_scale == 0 || y_scale == 0 {
                        continue;
                    }

                    let x_module_size = (x_scale as f64) / 100.0 * self.unit;
                    let y_module_size = (y_scale as f64) / 100.0 * self.unit;

                    // keep module centered if size != scale
                    result.push_str(&format!(
                        "M{},{}h{}v{}h-{}z",
                        x as f64 * self.unit + self.margin + (self.unit - x_module_size) / 2.0,
                        y as f64 * self.unit + self.margin + (self.unit - y_module_size) / 2.0,
                        x_module_size,
                        y_module_size,
                        x_module_size
                    ));
                }
            }
            result.push_str("\"/>");
        }

        // render_finder(matrix, &options, &mut result);

        result.push_str("</svg>");

        result
    }
}
