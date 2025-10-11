use crate::data::medical_imaging::error::MedicalImagingResult;

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

    /// Returns pixel data as byte slice
    pub fn as_bytes(&self) -> &[u8]{
        match self {
            Self::UInt8(data) => data,
            Self::UInt16(data) => bytemuck::cast_slice(data),
            Self::Int16(data) => bytemuck::cast_slice(data),
            Self::Int32(data) => bytemuck::cast_slice(data),
            Self::Float32(data) => bytemuck::cast_slice(data),
            Self::Float64(data) => bytemuck::cast_slice(data),
        }
    }
}