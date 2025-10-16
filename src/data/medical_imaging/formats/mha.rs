/// Function-level comment: MHA (MetaImage) format parser implementation
/// Handles parsing of MHA files with embedded binary data according to MetaImage specification

use crate::data::medical_imaging::{
    error::*, 
    metadata::{Endianness, ImageMetadata, MedicalVolume, PixelData}, 
    CompressionType,
    ImageFormat, 
    validation::{MedicalImageValidator,DataSizeChecker, ChecksumChecker, ChecksumAlgorithm, MedicalHeaderChecker},
};
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;

/// Function-level comment: MHA format parser
/// Implements parsing for MHA files (MetaImage format with embedded data)
pub struct MhaParser {
    /// Validates MHA file format and signature
    validator: MedicalImageValidator,
    /// Handles different compression schemes
    compression_handler: CompressionType,
    /// Manages byte order conversion
    endian_converter: Endianness,
}

impl MhaParser {
    /// Creates new MHA parser with default configuration
    pub fn new(metadata: &ImageMetadata, pixel_data: &PixelData) -> Self {
        let mut validator = MedicalImageValidator::new();
        validator.add_format_validator(ImageFormat::MHA, metadata, pixel_data);
        let size_checker = DataSizeChecker::new(1024, Some(1024 * 1024));
        validator.add_integrity_checker(Box::new(size_checker));
        let checksum_checker = ChecksumChecker::new(0x12345678, ChecksumAlgorithm::Crc32);
        validator.add_integrity_checker(Box::new(checksum_checker));
        let header_checker = MedicalHeaderChecker::new(vec![0x4D, 0x48, 0x41], 256); // MHA
        validator.add_integrity_checker(Box::new(header_checker));
        Self {
            validator,
            compression_handler: CompressionType::Raw,
            endian_converter: Endianness::Little,
        }
    }

    /// Parses complete MHA file including header and embedded data
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn parse_file(path: PathBuf) -> MedicalImagingResult<MedicalVolume>{
        let path = path.join("CT.mha");
        let file = tokio::fs::read(path).await?;
        Self::parse_bytes(&file)
    }

    /// Parses MHA from byte buffer for WASM compatibility
    pub fn parse_bytes(data: &[u8]) -> MedicalImagingResult<MedicalVolume>{
        let metadata = Self::parse_metadata_only(data)?;
        let start_offset = metadata.data_offset.unwrap_or(0);
        let raw = &data[start_offset..];
        let pixel_data = PixelData::UInt8(raw.to_vec());
        MedicalVolume::new(metadata, pixel_data, ImageFormat::MHA)
    }
    
    /// Extracts only metadata without loading pixel data
    pub fn parse_metadata_only(data: &[u8]) -> MedicalImagingResult<ImageMetadata>{
        let mut kv: HashMap<String, String> = HashMap::new();
        let mut data_offset: Option<usize> = None;

        // find header lines
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

            cursor += raw_line.len() + 1; // +1 表示 '\n'

            let l = line.split('#').next().unwrap_or("").trim();
            if l.is_empty() {
                continue;
            }

            if let Some((k, v)) = l.split_once('=') {
                let key = k.trim();
                let val = v.trim();
                kv.insert(key.to_string(), val.to_string());

                if key.eq_ignore_ascii_case("ElementDataFile") {
                    if val.eq_ignore_ascii_case("LOCAL") {
                        data_offset = Some(cursor);
                    }
                    break; // ElementDataFile 通常是最后一行
                }
            } else {
                return Err(MedicalImagingError::UnsupportedFormat {
                    format: format!("Invalid line {}: {}", line_no, l)
                });
            }
        }

        // analyze header key-values
        ImageMetadata::get_header(kv, data_offset)
    }

}
