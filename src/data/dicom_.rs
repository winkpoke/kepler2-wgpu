use anyhow::Result;
use dicom_object::{from_reader, FileDicomObject, InMemDicomObject};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
pub struct DicomObject {
    #[allow(dead_code)]
    pub dcm: FileDicomObject<InMemDicomObject>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct DicomObject {
    #[allow(dead_code)]
    dcm: FileDicomObject<InMemDicomObject>,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl DicomObject {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let f = std::io::Cursor::new(bytes);
        let dcm = from_reader(f)?;
        Ok(Self { dcm })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, JsValue> {
        let f = std::io::Cursor::new(bytes);
        let result = from_reader(f);
        result
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
            .map(|dcm| Self { dcm })
    }
}
