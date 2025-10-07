use std::fmt;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Custom error types for the Kepler WGPU medical imaging application
#[derive(Debug)]
pub enum KeplerError {
    /// Graphics/Rendering related errors
    Graphics(String),
    /// DICOM file processing errors
    Dicom(String),
    /// File I/O errors
    Io(std::io::Error),
    /// WGPU surface errors
    Surface(wgpu::SurfaceError),
    /// Window creation errors
    Window(String),
    /// Data validation errors
    Validation(String),
}

impl fmt::Display for KeplerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeplerError::Graphics(msg) => write!(f, "Graphics error: {msg}"),
            KeplerError::Dicom(msg) => write!(f, "DICOM processing error: {msg}"),
            KeplerError::Io(err) => write!(f, "I/O error: {err}"),
            KeplerError::Surface(err) => write!(f, "Surface error: {err}"),
            KeplerError::Window(msg) => write!(f, "Window error: {msg}"),
            KeplerError::Validation(msg) => write!(f, "Validation error: {msg}"),
        }
    }
}

impl std::error::Error for KeplerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            KeplerError::Io(err) => Some(err),
            KeplerError::Surface(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for KeplerError {
    fn from(err: std::io::Error) -> Self {
        KeplerError::Io(err)
    }
}

impl From<wgpu::SurfaceError> for KeplerError {
    fn from(err: wgpu::SurfaceError) -> Self {
        KeplerError::Surface(err)
    }
}

impl From<anyhow::Error> for KeplerError {
    fn from(err: anyhow::Error) -> Self {
        KeplerError::Validation(err.to_string())
    }
}

#[cfg(target_arch = "wasm32")]
impl From<KeplerError> for JsValue {
    fn from(err: KeplerError) -> Self {
        JsValue::from_str(&err.to_string())
    }
}

/// Result type alias for convenience
pub type KeplerResult<T> = Result<T, KeplerError>;