pub mod image;
pub mod svg;
#[cfg(feature = "text")]
pub mod text;

use crate::matrix::{Matrix, QrMatrix};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

pub struct RenderData<'m, 'x, 'y> {
    matrix: &'m Matrix,
    scale_x_matrix: Option<&'x Vec<u8>>, // scale x 0-200%
    scale_y_matrix: Option<&'y Vec<u8>>, // scale y 0-200%
    unit: u8,
    foreground: String,
    background: String,
    toggle_options: u8,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub enum Toggle {
    Background,
    BackgroundPixels,
    ForegroundPixels,
}

impl<'m, 'x, 'y> RenderData<'m, 'x, 'y> {
    pub fn new(matrix: &'m Matrix) -> Self {
        RenderData {
            matrix,
            scale_x_matrix: None,
            scale_y_matrix: None,
            unit: 1,
            foreground: "#000".into(),
            background: "#fff".into(),
            toggle_options: 0,
        }
        .toggle(Toggle::Background)
        .toggle(Toggle::ForegroundPixels)
    }
    pub fn width(&self) -> u32 {
        self.matrix.width() as u32 * self.unit as u32
    }
    pub fn height(&self) -> u32 {
        self.matrix.height() as u32 * self.unit as u32
    }
    pub fn unit(mut self, unit: u8) -> Self {
        self.unit = unit;
        self
    }
    pub fn foreground(mut self, foreground: String) -> RenderData<'m, 'x, 'y> {
        self.foreground = foreground;
        self
    }
    pub fn background(mut self, background: String) -> RenderData<'m, 'x, 'y> {
        self.background = background;
        self
    }
    pub fn scale_x_matrix(mut self, scale_matrix: Option<&'x Vec<u8>>) -> RenderData<'m, 'x, 'y> {
        self.scale_x_matrix = scale_matrix;
        self
    }
    pub fn scale_y_matrix(mut self, scale_matrix: Option<&'y Vec<u8>>) -> RenderData<'m, 'x, 'y> {
        self.scale_y_matrix = scale_matrix;
        self
    }
    pub fn scale_matrix<'xy: 'x + 'y>(
        mut self,
        scale_matrix: Option<&'xy Vec<u8>>,
    ) -> RenderData<'m, 'x, 'y> {
        self.scale_x_matrix = scale_matrix;
        self.scale_y_matrix = scale_matrix;
        self
    }
    pub fn toggle_options(mut self, toggle_options: u8) -> RenderData<'m, 'x, 'y> {
        self.toggle_options = toggle_options;
        self
    }
    pub fn toggle(mut self, toggle: Toggle) -> RenderData<'m, 'x, 'y> {
        self.toggle_options ^= 1 << toggle as u8;
        self
    }
    pub fn get(&self, option: Toggle) -> bool {
        (self.toggle_options >> option as u8) & 1 == 1
    }
}
