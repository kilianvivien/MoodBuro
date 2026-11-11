//! The worker boundary. A single PDFium instance lives for the worker's lifetime.
use js_sys::{Array, Reflect, Uint8Array};
use pdfium_render::prelude::Pdfium;
use wasm_bindgen::prelude::*;

use crate::convert::{self, Upload};
use crate::pdfium_bindings::{FastBindings, NativeBindings, ReadBindings};

#[wasm_bindgen]
pub struct BrowserEngine {
    bindings: FastBindings,
    fast: bool,
}

#[wasm_bindgen]
impl BrowserEngine {
    #[wasm_bindgen(constructor)]
    pub fn new(fast: Option<bool>) -> Result<BrowserEngine, JsValue> {
        let bindings = Pdfium::bind_to_system_library().map_err(|_| JsValue::from_str("LOCAL_BOOT_FAILED"))?;
        // SAFETY: one engine is constructed after the modules are bound; the
        // bindings live until the worker is terminated.
        unsafe { bindings.FPDF_InitLibrary() };
        // The stock path remains available for the maintainer's comparative
        // benchmark. Production workers omit this argument and use the bridge.
        Ok(Self { bindings: FastBindings(bindings), fast: fast.unwrap_or(true) })
    }

    pub fn convert(&self, fields: &str, files: Array, now: f64) -> Result<String, JsValue> {
        let mut uploads = Vec::with_capacity(files.length() as usize);
        for file in files.iter() {
            let get = |key: &str| Reflect::get(&file, &JsValue::from_str(key));
            let bytes = get("bytes")?;
            if !bytes.is_instance_of::<Uint8Array>() {
                return Err(JsValue::from_str("LOCAL_CRASH"));
            }
            uploads.push(Upload {
                name: get("name")?.as_string(),
                content_type: get("type")?.as_string(),
                bytes: Uint8Array::new(&bytes).to_vec(),
            });
        }
        let stock = NativeBindings(self.bindings.0.as_ref());
        let bindings: &dyn ReadBindings = if self.fast { &self.bindings } else { &stock };
        let mut reader = |bytes: &[u8], name: &str| crate::extract_with_reader(bindings, bytes, name);
        Ok(convert::convert(fields, &uploads, &mut reader, now as i64))
    }
}
