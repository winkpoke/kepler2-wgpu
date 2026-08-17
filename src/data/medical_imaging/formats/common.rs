/// Function-level comment: Common traits and interfaces for medical image format parsers
/// Provides unified API for parsing different medical imaging formats
use crate::data::medical_imaging::{
    error::{MedicalImagingError, MedicalImagingResult},
    metadata::*,
    mha::MhaParser,
    mhd::MhdParser,
};
use std::collections::HashMap;
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
    path.extension()
        .and_then(|ext| ext.to_str().map(|s| s.to_string()))
        .map(|ext| match ext.as_str() {
            "mha" => ImageFormat::MHA,
            "mhd" => ImageFormat::MHD,
            "nii" => ImageFormat::NIfTI,
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
    /// Parses a medical image from raw bytes
    fn parse_bytes(&self, path: &[u8], data: Option<&[u8]>) -> MedicalImagingResult<MedicalVolume> {
        if let Some(data) = data {
            MhdParser::parse_by_bytes(path, data)
        } else {
            MhaParser::parse_bytes(path)
        }
    }

    /// Extracts metadata from a medical image file
    fn extract_metadata(
        &self,
        path: &[u8],
        format: ImageFormat,
    ) -> MedicalImagingResult<ImageMetadata> {
        match format {
            ImageFormat::MHA => MhaParser::parse_metadata_only(path),
            ImageFormat::MHD => MhdParser::parse_metadata_only(path),
            _ => Err(MedicalImagingError::UnsupportedFormat {
                format: format!("file extension: {:?}", format),
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

/// Parses MetaImage header (MHA/MHD) key-value pairs from raw bytes
/// Returns (key-value pairs, data_offset if ElementDataFile=LOCAL was found)
pub fn parse_metaimage_header(data: &[u8]) -> MedicalImagingResult<(HashMap<String, String>, Option<usize>)> {
    let mut kv: HashMap<String, String> = HashMap::new();
    let mut data_offset: Option<usize> = None;

    // Limit header scan to 64KB to avoid scanning entire file
    let max_size = 64 * 1024;
    let header_region = &data[..std::cmp::min(data.len(), max_size)];
    let mut cursor: usize = 0;

    for (line_no, raw_line) in header_region.split(|&b| b == b'\n').enumerate() {
        let line = std::str::from_utf8(raw_line)
            .map_err(|e| MedicalImagingError::ParseError {
                field: format!("Line {}", line_no),
                reason: e.to_string(),
            })?
            .trim();

        cursor += raw_line.len() + 1; // +1 for '\n'

        // Strip comments
        let l = line.split('#').next().unwrap_or("").trim();
        if l.is_empty() {
            continue;
        }

        if let Some((k, v)) = l.split_once('=') {
            let key = k.trim();
            let val = v.trim();
            kv.insert(key.to_string(), val.to_string());

            // For MHA files, track where binary data starts
            if key.eq_ignore_ascii_case("ElementDataFile") {
                if val.eq_ignore_ascii_case("LOCAL") {
                    data_offset = Some(cursor);
                }
                break; // ElementDataFile is typically the last header line
            }
        } else {
            return Err(MedicalImagingError::UnsupportedFormat {
                format: format!("Invalid line {}: {}", line_no, l),
            });
        }
    }

    Ok((kv, data_offset))
}
