use crate::data::medical_imaging::{
    error::*,
    metadata::{ImageMetadata, MedicalVolume, PixelData},
    validation::MedicalImageValidator,
    ImageFormat,
};
use std::fs::File;
use std::path::PathBuf;
use std::{collections::HashMap, io::Read};

/// Function-level comment: Parses MHD (MetaIO) files with separate data files
/// Handles header files that reference external raw or compressed data
pub struct MhdParser {
    /// Validates MHD header format
    #[allow(dead_code)]
    validator: MedicalImageValidator,
    /// Resolves data file paths relative to header
    path_resolver: PathBuf,
    /// Handles various data file formats
    #[allow(dead_code)]
    data_loader: PathBuf,
}

impl MhdParser {
    /// Creates a new MHD parser instance
    pub fn new(
        validator: MedicalImageValidator,
        path_resolver: PathBuf,
        data_loader: PathBuf,
    ) -> Self {
        Self {
            validator,
            path_resolver,
            data_loader,
        }
    }

    /// Parses MHD header file and loads associated data file
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn parse_file(path: PathBuf) -> MedicalImagingResult<MedicalVolume> {
        let mhd = MhdParser::new(MedicalImageValidator::new(), PathBuf::new(), path.clone());
        let mhd_path = mhd.data_loader.clone().join("CT.mhd");
        let bytes_mhd = tokio::fs::read(mhd_path.clone()).await?;
        let metadata = Self::parse_metadata_only(&bytes_mhd)?;
        let pixel_data = mhd.load_data_file(&metadata)?;
        let mut mhd_clone = MhdParser::new(MedicalImageValidator::new(), PathBuf::new(), mhd_path);
        let result =
            mhd_clone
                .validator
                .add_format_validator(ImageFormat::MHD, &metadata, &pixel_data);
        if result.is_valid {
            MedicalVolume::new(metadata, pixel_data, ImageFormat::MHD)
        } else {
            Err(MedicalImagingError::InvalidPath {
                path: (path.to_string_lossy().to_string()),
            })
        }
    }

    /// Parses MHD header file from raw bytes and loads pixel data
    pub fn parse_by_bytes(mhd: &[u8], data: &[u8]) -> MedicalImagingResult<MedicalVolume> {
        let metadata = Self::parse_metadata_only(mhd)?;
        let pixel_data = PixelData::UInt8(data.to_vec());
        MedicalVolume::new(metadata, pixel_data, ImageFormat::MHD)
    }

    /// Loads pixel data from associated data file
    pub fn load_data_file(self, metadata: &ImageMetadata) -> MedicalImagingResult<PixelData> {
        let dims = metadata.dimensions.clone();
        let n = dims[0] * dims[1] * dims[2];
        let mut temp_buf = vec![0u8; n * 4];
        let mut path = self.path_resolver.clone();
        path.set_extension("raw");
        let mut f = File::open(path)?;
        f.read_exact(&mut temp_buf)?;
        Ok(PixelData::UInt8(temp_buf))
    }

    /// Parses MHD header file from raw bytes
    pub fn parse_metadata_only(mhd_data: &[u8]) -> MedicalImagingResult<ImageMetadata> {
        let mut kv: HashMap<String, String> = HashMap::new();
        let data_offset: Option<usize> = None;

        // Parse header lines from the MHD file content
        for line in mhd_data.split(|&b| b == b'\n') {
            let line = std::str::from_utf8(line)
                .map_err(|e| MedicalImagingError::ParseError {
                    field: format!("Line {:?}", line),
                    reason: e.to_string(),
                })?
                .trim();

            let l = line.split('#').next().unwrap_or("").trim();
            if l.is_empty() {
                continue;
            }

            if let Some((k, v)) = line.split_once('=') {
                kv.insert(k.trim().to_string(), v.trim().to_string());
            }
        }

        // Check if the MHD file references an external data file
        if let Some(data_file) = kv.get("ElementDataFile") {
            if data_file != "LOCAL" && !data_file.is_empty() {
                log::warn!("MHD file references external data file '{}' which is not available in WASM context", data_file);
                log::info!("Proceeding with metadata-only parsing for visualization purposes");
            }
        }

        // Parse the metadata from header key-value pairs
        let mut metadata = ImageMetadata::get_header(kv, data_offset)?;

        // Mark that this is a header-only parse for WASM
        metadata.element_data_file = "WASM_HEADER_ONLY".to_string();

        Ok(metadata)
    }
}
