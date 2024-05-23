pub mod constants;
pub mod math;
pub mod qrcode;

pub mod data;
pub mod encoding;
pub mod error_correction;

pub mod mask;
pub mod matrix;

pub mod render;

#[cfg(feature = "wasm")]
mod wasm;
