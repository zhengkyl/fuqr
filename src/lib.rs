pub mod codewords;
pub mod constants;
pub mod data;
pub mod encode;
pub mod math;
pub mod matrix;
pub mod qrcode;
pub mod render;

#[cfg(feature = "wasm")]
mod wasm;
