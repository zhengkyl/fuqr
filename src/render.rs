#[cfg(feature = "image")]
pub mod image;
#[cfg(feature = "svg")]
pub mod svg;
#[cfg(feature = "text")]
pub mod text;

use crate::matrix::Matrix;

pub struct RenderData<'m> {
    matrix: &'m Matrix,
    foreground: String,
    background: String,
    unit: u8,
    margin: u8,
    toggle_options: u8,
}

pub enum Toggle {
    Background,
    BackgroundPixels,
    ForegroundPixels,
}

impl<'m> RenderData<'m> {
    pub fn new(matrix: &'m Matrix) -> Self {
        RenderData {
            matrix,
            foreground: "#000".into(),
            background: "#fff".into(),
            unit: 1,
            margin: 2,
            toggle_options: 0,
        }
        .toggle(Toggle::Background)
        .toggle(Toggle::ForegroundPixels)
    }
    pub fn width(&self) -> u32 {
        (self.matrix.width() as u32 + self.margin as u32 * 2) * self.unit as u32
    }
    pub fn unit(mut self, unit: u8) -> Self {
        self.unit = unit;
        self
    }
    pub fn margin(mut self, margin: u8) -> Self {
        self.margin = margin;
        self
    }
    pub fn foreground(mut self, foreground: String) -> Self {
        self.foreground = foreground;
        self
    }
    pub fn background(mut self, background: String) -> Self {
        self.background = background;
        self
    }
    pub fn toggle_options(mut self, toggle_options: u8) -> Self {
        self.toggle_options = toggle_options;
        self
    }
    pub fn toggle(mut self, toggle: Toggle) -> Self {
        self.toggle_options ^= 1 << toggle as u8;
        self
    }
    pub fn toggled(&self, option: Toggle) -> bool {
        (self.toggle_options >> option as u8) & 1 == 1
    }
}
