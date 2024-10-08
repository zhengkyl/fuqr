use crate::{
    bit_info::BitInfo,
    qr_code::{Mask, Mode, QrCode, Version, ECL},
    QartError, QrError, QrOptions,
};
use wasm_bindgen::prelude::*;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn generate(input: &str, qr_options: &QrOptions) -> Result<JsValue, QrError> {
    console_error_panic_hook::set_once();
    let qr_code = match crate::generate(input, qr_options) {
        Ok(m) => m,
        Err(e) => return Err(e),
    };
    Ok(qr_code_to_obj(qr_code))
}

#[wasm_bindgen]
pub fn generate_qart(
    input: &str,
    qr_options: &QrOptions,
    pixel_weights: &[u8],
) -> Result<JsValue, QartError> {
    console_error_panic_hook::set_once();
    let pixel_weights = unsafe { std::mem::transmute(pixel_weights) };
    let qr_code = match crate::generate_qart(input, qr_options, pixel_weights) {
        Ok(m) => m,
        Err(e) => return Err(e),
    };
    Ok(qr_code_to_obj(qr_code))
}

fn qr_code_to_obj(qr_code: QrCode) -> JsValue {
    let u = js_sys::Uint8Array::new_with_length(qr_code.matrix.value.len() as u32);
    u.copy_from(unsafe { std::mem::transmute(qr_code.matrix.value.as_slice()) });

    let obj = js_sys::Object::new();
    // If these error, it's not recoverable
    let _ = js_sys::Reflect::set(&obj, &"matrix".into(), &u);
    let _ = js_sys::Reflect::set(&obj, &"mode".into(), &JsValue::from(qr_code.mode));
    let _ = js_sys::Reflect::set(&obj, &"version".into(), &JsValue::from(qr_code.version.0));
    let _ = js_sys::Reflect::set(&obj, &"ecl".into(), &JsValue::from(qr_code.ecl));
    let _ = js_sys::Reflect::set(&obj, &"mask".into(), &JsValue::from(qr_code.mask));

    obj.into()
}

#[wasm_bindgen(js_name = internalBitInfo)]
pub fn internal_bit_info(mode: Mode, version: Version, ecl: ECL, mask: Mask) -> JsValue {
    console_error_panic_hook::set_once();

    let bit_info = BitInfo::new(mode, version, ecl, mask);

    let u = js_sys::Uint32Array::new_with_length(bit_info.matrix.value.len() as u32);
    u.copy_from(unsafe { std::mem::transmute(bit_info.matrix.value.as_slice()) });

    let obj = js_sys::Object::new();
    // If these error, it's not recoverable
    let _ = js_sys::Reflect::set(&obj, &"matrix".into(), &u);
    let _ = js_sys::Reflect::set(&obj, &"mode".into(), &JsValue::from(bit_info.mode));
    let _ = js_sys::Reflect::set(&obj, &"version".into(), &JsValue::from(bit_info.version.0));
    let _ = js_sys::Reflect::set(&obj, &"ecl".into(), &JsValue::from(bit_info.ecl));
    let _ = js_sys::Reflect::set(&obj, &"mask".into(), &JsValue::from(bit_info.mask));
    obj.into()
}
