/// Function-level comment: Common traits and interfaces for medical image format parsers
/// Provides unified API for parsing different medical imaging formats
use crate::data::medical_imaging::{
    error::{MedicalImagingError, MedicalImagingResult},
    metadata::*,
    metaimage::MetaimageParser,
    nifti::NiftiParser,
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
    let lower = file_path.to_lowercase();
    if lower.ends_with(".nii.gz") || lower.ends_with(".nii") {
        return Ok(ImageFormat::NIfTI);
    }
    let path = Path::new(file_path);
    path.extension()
        .and_then(|ext| ext.to_str().map(|s| s.to_string()))
        .map(|ext| match ext.as_str() {
            "mha" => ImageFormat::MHA,
            "mhd" => ImageFormat::MHD,
            "dcm" => ImageFormat::DICOM,
            _ => ImageFormat::Unknown,
        })
        .ok_or_else(|| MedicalImagingError::UnsupportedFormat {
            format: format!("file extension: {}", file_path),
        })
}

/// Function-level comment: Medical image parser trait
/// Defines the interface that all medical image format parsers must implement
pub trait MedicalImageParser {
    fn parse_bytes(
        &self,
        format: ImageFormat,
        header: &[u8],
        data: Option<&[u8]>,
    ) -> MedicalImagingResult<MedicalVolume> {
        match format {
            ImageFormat::MHA => MetaimageParser::parse_bytes(header, None),
            ImageFormat::MHD => MetaimageParser::parse_bytes(header, data),
            ImageFormat::NIfTI => NiftiParser::parse_bytes(header),
            other => Err(MedicalImagingError::UnsupportedFormat {
                format: format!("file extension: {:?}", other),
            }),
        }
    }

    fn extract_metadata(
        &self,
        path: &[u8],
        format: ImageFormat,
    ) -> MedicalImagingResult<ImageMetadata> {
        match format {
            ImageFormat::MHA => MetaimageParser::parse_metadata_only(path, true),
            ImageFormat::MHD => MetaimageParser::parse_metadata_only(path, false),
            ImageFormat::NIfTI => NiftiParser::parse_metadata_only(path),
            other => Err(MedicalImagingError::UnsupportedFormat {
                format: format!("file extension: {:?}", other),
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
