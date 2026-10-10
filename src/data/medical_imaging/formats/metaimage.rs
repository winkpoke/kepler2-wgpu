/// MetaImage (MHA/MHD) format parser.
///
/// MHA = header + embedded voxel data (`ElementDataFile=LOCAL`).
/// MHD = header + external voxel data (separate `.raw` / a second byte buffer).
use crate::data::medical_imaging::{
    error::*,
    metadata::{Endianness, ImageMetadata, MedicalVolume, PixelData},
    validation::{
        ChecksumAlgorithm, ChecksumChecker, DataSizeChecker, MedicalHeaderChecker,
        MedicalImageValidator,
    },
    CompressionType, ImageFormat,
};
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::fs::File;
use std::io::Read;

/// Unified parser for the MetaImage format family (MHA and MHD).
pub struct MetaimageParser {
    /// Validates parsed file format and signature
    validator: MedicalImageValidator,
    /// Handles different compression schemes
    compression_handler: CompressionType,
    /// Manages byte order conversion
    endian_converter: Endianness,
}

impl MetaimageParser {
    /// Creates new parser with default configuration
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
    pub async fn parse_file_mha(path: PathBuf) -> MedicalImagingResult<MedicalVolume> {
        let path = path.join("CT.mha");
        let file = tokio::fs::read(path).await?;
        Self::parse_bytes(&file, None)
    }

    /// Parses MHD header file and loads associated data file
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn parse_file_mhd(mut path: PathBuf) -> MedicalImagingResult<MedicalVolume> {
        let data_loader = PathBuf::new();
        let mhd_path = data_loader.clone().join("CT.mhd");
        let bytes_mhd = tokio::fs::read(mhd_path.clone()).await?;
        let metadata = Self::parse_metadata_only(&bytes_mhd, false)?;
        let dims = metadata.dimensions.clone();
        let n = dims[0] * dims[1] * dims[2];
        let mut temp_buf = vec![0u8; n * 4];
        path.set_extension("raw");
        let mut f = File::open(path)?;
        f.read_exact(&mut temp_buf)?;
        let pixel_data = PixelData::UInt8(temp_buf);
        MedicalVolume::new(metadata, pixel_data, ImageFormat::MHD)
    }

    /// Parses MHX from byte buffer for WASM compatibility
    pub fn parse_bytes(header: &[u8], data: Option<&[u8]>) -> MedicalImagingResult<MedicalVolume> {
        match data {
            None => {
                let metadata = Self::parse_metadata_only(header, true)?;
                let start_offset = metadata.data_offset.unwrap_or(0);
                let raw = &header[start_offset..];
                let pixel_data = PixelData::UInt8(raw.to_vec());
                let parser = Self::new(&metadata, &pixel_data);
                let result = parser.validator.format_validators
                    .get(&ImageFormat::MHA).unwrap().clone();
                if !result.is_valid {
                    return Err(MedicalImagingError::UnsupportedFormat {
                        format: format!("Invalid MHA format: {:?}", result.errors),
                    });
                }
                let pixel_data = match parser.endian_converter {
                    Endianness::Little => {
                        if parser.compression_handler == CompressionType::Raw {
                            pixel_data
                        } else {
                            PixelData::from_le_bytes(raw, metadata.pixel_type)?
                        }
                    }
                    Endianness::Big => PixelData::from_be_bytes(raw, metadata.pixel_type)?,
                };
                MedicalVolume::new(metadata, pixel_data, ImageFormat::MHA)
            }
            Some(data) => {
                let metadata = Self::parse_metadata_only(header, false)?;
                let pixel_data = PixelData::UInt8(data.to_vec());
                MedicalVolume::new(metadata, pixel_data, ImageFormat::MHD)
            }
        }
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

    /// Extracts only metadata without loading pixel data
    pub fn parse_metadata_only(data: &[u8], is_mha: bool) -> MedicalImagingResult<ImageMetadata> {
        let (kv, data_offset) = Self::parse_metaimage_header(data)?;
        if is_mha{
            ImageMetadata::get_header(kv, data_offset)
        } else {
            // Check if the MHD file references an external data file
            if let Some(data_file) = kv.get("ElementDataFile") {
                if data_file != "LOCAL" && !data_file.is_empty() {
                    log::warn!("MHD file references external data file '{}' which is not available in WASM context", data_file);
                    log::info!("Proceeding with metadata-only parsing for visualization purposes");
                }
            }
            let mut metadata = ImageMetadata::get_header(kv, None)?;
            metadata.element_data_file = "WASM_HEADER_ONLY".to_string();
            Ok(metadata)
        }
    }
}
