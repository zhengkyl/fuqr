use wasm_bindgen::prelude::*;
use crate::{QrError, QrOptions};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn generate(input: &str, qr_options: QrOptions) -> Result<JsValue, QrError> {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    let qr_code = match crate::generate(input, qr_options) {
        Ok(m) => m,
        Err(e) => return Err(e),
    };

    let u = js_sys::Uint8Array::new_with_length(qr_code.matrix.value.len() as u32);
    u.copy_from(unsafe { std::mem::transmute(qr_code.matrix.value.as_slice()) });

    let obj = js_sys::Object::new();
    // If these error, it's not recoverable
    let _ = js_sys::Reflect::set(&obj, &"matrix".into(), &u);
    let _ = js_sys::Reflect::set(&obj, &"mode".into(), &JsValue::from(qr_code.mode));
    let _ = js_sys::Reflect::set(&obj, &"version".into(), &JsValue::from(qr_code.version.0));
    let _ = js_sys::Reflect::set(&obj, &"mask".into(), &JsValue::from(qr_code.mask));
    let _ = js_sys::Reflect::set(&obj, &"ecl".into(), &JsValue::from(qr_code.ecl));
    Ok(obj.into())
}
