/// Function-level comment: Common traits and interfaces for medical image format parsers
/// Provides unified API for parsing different medical imaging formats

use crate::data::medical_imaging::{
    metadata::*,
    error::{MedicalImagingError, MedicalImagingResult},
    mhd::MhdParser,
    mha::MhaParser,
};
use std::path::Path;

/// Image format enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    MHA,
    MHD,
    NIfTI,
    DICOM,
    Unknown,
}

// Function-level comment: Extracts the image format from a file path
pub fn get_extension(file_path: &str) -> MedicalImagingResult<ImageFormat> {
    let path = Path::new(file_path);
    path.extension().and_then(|ext| ext.to_str().map(|s| s.to_string()))
        .map(|ext| match ext.as_str() {
            "mha" => ImageFormat::MHA,
            "mhd" => ImageFormat::MHD,
            "nii" => ImageFormat::NIfTI,
            "dcm" => ImageFormat::DICOM,
            _ => ImageFormat::Unknown,
        })
        .ok_or_else(|| MedicalImagingError::UnsupportedFormat {
            format: format!("file extension: {}", file_path)
        })
}

/// Function-level comment: Medical image parser trait
/// Defines the interface that all medical image format parsers must implement
pub trait MedicalImageParser {
    /// Parses a medical image from raw bytes
    fn parse_bytes(&self, path: &[u8], data: Option<&[u8]>) -> MedicalImagingResult<MedicalVolume>{
        if let Some(data) = data {
            MhdParser::parse_by_bytes(path, data)
        } else {
            MhaParser::parse_bytes(path)
        }
    }
    
    /// Extracts metadata from a medical image file
    fn extract_metadata(&self, path: &[u8], format: ImageFormat) -> MedicalImagingResult<ImageMetadata>{
        match format {
            ImageFormat::MHA => {
                MhaParser::parse_metadata_only(path)
            },
            ImageFormat::MHD => {
                MhdParser::parse_metadata_only(path)
            },
            _ => Err(MedicalImagingError::UnsupportedFormat {
                format: format!("file extension: {:?}", format)
            }),
        }
    }
}

// Function-level comment: Compression type enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum CompressionType {
    GZip,
    ZLib,
    Raw,
}
