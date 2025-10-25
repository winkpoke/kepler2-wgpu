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
    /// MPR view related errors
    Mpr(MprError),
}

/// MPR (Multi-Planar Reconstruction) view specific errors
#[derive(Debug)]
pub enum MprError {
    /// Invalid scale value (must be positive and finite)
    InvalidScale(f32),
    /// Invalid slice position (out of volume bounds)
    InvalidSlicePosition(f32),
    /// Invalid window level value
    InvalidWindowLevel(f32),
    /// Invalid window width value (must be positive)
    InvalidWindowWidth(f32),
    /// Invalid pan coordinates
    InvalidPanCoordinates([f32; 3]),
    /// Matrix transformation failed (singular matrix)
    InvalidTransformation,
    /// Coordinate out of bounds
    CoordinateOutOfBounds([f32; 3]),
    /// GPU resource error
    GpuResourceError(String),
    /// Invalid view dimensions
    InvalidDimensions(u32, u32),
    /// Invalid position coordinates
    InvalidPosition(i32, i32),
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
            KeplerError::Mpr(err) => write!(f, "MPR error: {err}"),
        }
    }
}

impl fmt::Display for MprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MprError::InvalidScale(scale) => write!(f, "Invalid scale value: {scale} (must be positive and finite)"),
            MprError::InvalidSlicePosition(pos) => write!(f, "Invalid slice position: {pos} (out of volume bounds)"),
            MprError::InvalidWindowLevel(level) => write!(f, "Invalid window level: {level}"),
            MprError::InvalidWindowWidth(width) => write!(f, "Invalid window width: {width} (must be positive)"),
            MprError::InvalidPanCoordinates(coords) => write!(f, "Invalid pan coordinates: [{}, {}, {}]", coords[0], coords[1], coords[2]),
            MprError::InvalidTransformation => write!(f, "Matrix transformation failed (singular matrix)"),
            MprError::CoordinateOutOfBounds(coords) => write!(f, "Coordinate out of bounds: [{}, {}, {}]", coords[0], coords[1], coords[2]),
            MprError::GpuResourceError(msg) => write!(f, "GPU resource error: {msg}"),
            MprError::InvalidDimensions(w, h) => write!(f, "Invalid view dimensions: {}x{}", w, h),
            MprError::InvalidPosition(x, y) => write!(f, "Invalid position: ({}, {})", x, y),
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

impl From<MprError> for KeplerError {
    fn from(err: MprError) -> Self {
        KeplerError::Mpr(err)
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