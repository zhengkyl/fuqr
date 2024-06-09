pub mod image;
pub mod svg;
#[cfg(feature = "text")]
pub mod text;

use std::rc::Rc;

use crate::matrix::{Matrix, QrMatrix};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

pub struct RenderData<'a> {
    matrix: &'a Matrix,
    unit: u8,
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

impl<'a> RenderData<'a> {
    pub fn new(matrix: &'a Matrix) -> Self {
        let scale_x_matrix = Rc::new(vec![100; matrix.width() * matrix.height()]);
        let scale_y_matrix = scale_x_matrix.clone();

        RenderData {
            matrix,
            unit: 1,
            foreground: "#000".into(),
            background: "#fff".into(),
            scale_x_matrix,
            scale_y_matrix,
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
    pub fn foreground(mut self, foreground: String) -> RenderData<'a> {
        self.foreground = foreground;
        self
    }
    pub fn background(mut self, background: String) -> RenderData<'a> {
        self.background = background;
        self
    }
    pub fn scale_x_matrix(mut self, scale_matrix: Vec<u8>) -> RenderData<'a> {
        self.scale_x_matrix = Rc::new(scale_matrix);
        self
    }
    pub fn scale_y_matrix(mut self, scale_matrix: Vec<u8>) -> RenderData<'a> {
        self.scale_y_matrix = Rc::new(scale_matrix);
        self
    }
    pub fn scale_matrix(mut self, scale_matrix: Vec<u8>) -> RenderData<'a> {
        self.scale_x_matrix = Rc::new(scale_matrix);
        self.scale_y_matrix = Rc::clone(&self.scale_x_matrix);
        self
    }
    pub fn toggle_options(mut self, toggle_options: u8) -> RenderData<'a> {
        self.toggle_options = toggle_options;
        self
    }
    pub fn toggle(mut self, toggle: Toggle) -> RenderData<'a> {
        self.toggle_options ^= 1 << toggle as u8;
        self
    }
    pub fn get(&self, option: Toggle) -> bool {
        (self.toggle_options >> option as u8) & 1 == 1
    }
}
