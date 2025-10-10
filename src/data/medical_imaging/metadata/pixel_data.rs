use crate::data::medical_imaging::error::{MedicalImagingError, MedicalImagingResult};
use std::fs::File;

/// Function-level comment: Endianness enumeration
/// Defines byte order for multi-byte pixel types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    Little,
    Big,
}

/// Function-level comment: Pixel data type enumeration
/// Represents different pixel data types supported in medical imaging
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelType {
    UInt8,
    UInt16,
    Int16,
    Float32,
    Int32,
    Float64,
}

impl PixelType {
    /// Parses pixel type from string (case-insensitive)
    pub fn from_str(s: &str) -> MedicalImagingResult<Self> {
        match s.to_lowercase().as_str() {
            "met_uint8" | "met_uchar" => Ok(PixelType::UInt8),
            "met_int8" | "met_char" => Ok(PixelType::Int8),
            "met_uint16" | "met_ushort" => Ok(PixelType::UInt16),
            "met_int16" | "met_short" => Ok(PixelType::Int16),
            "met_uint32" | "met_uint"  => Ok(PixelType::UInt32),
            "met_int32" | "met_int" => Ok(PixelType::Int32),
            "met_float32" | "met_float" => Ok(PixelType::Float32),
            "met_float64" | "met_double" => Ok(PixelType::Float64),
            _ => Err(MedicalImagingError::UnsupportedFormat {
                format: format!("pixel type: {}", s)
            }),
        }
    }
}

/// Type-safe pixel data container
/// Stores pixel data with type information and provides safe access methods
#[derive(Debug, Clone)]
pub enum PixelData {
    UInt8(Vec<u8>),
    Int16(Vec<i16>),
    UInt16(Vec<u16>),
    Int32(Vec<i32>),
    Float32(Vec<f32>),
    Float64(Vec<f64>),
}

impl PixelData {
    /// Creates pixel data from raw bytes with specified type
    pub fn from_bytes(
        bytes: &[u8], 
        pixel_type: PixelType, 
        endianness: Endianness
    ) -> MedicalImagingResult<Self>{
        match pixel_type {
            PixelType::UInt8 => Ok(Self::UInt8(bytes.to_vec())),
            PixelType::UInt16 => Ok(Self::UInt16(bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect())),
            PixelType::Int16 => Ok(Self::Int16(bytes.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect())),
            PixelType::Int32 => Ok(Self::Int32(bytes.chunks_exact(4).map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())),
            PixelType::Float32 => Ok(Self::Float32(bytes.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())),
            PixelType::Float64 => Ok(Self::Float64(bytes.chunks_exact(8).map(|c| f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]])).collect())),
        }
    }

    pub fn get_pixel(path: &'static str, x: usize, y: usize, z: usize, dims: [usize; 3]) -> MedicalImagingResult<Self>{
        let n = x * y * z;
        let mut temp_buf = vec![0u8; n * 4];
        let mut f = File::open(path)?;
        f.read_exact(&mut temp_buf)?;
        temp_buf
    }
}