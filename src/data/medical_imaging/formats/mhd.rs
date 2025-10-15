use crate::data::medical_imaging::{
    error::*, 
    metadata::{MedicalVolume,  ImageMetadata, PixelData},
    ImageFormat,
    validation::MedicalImageValidator,
};
use std::{collections::HashMap, io::Read};
use std::fs::File;
use std::io::{BufRead,BufReader};
use std::path::PathBuf;

/// Function-level comment: Parses MHD (MetaIO) files with separate data files
/// Handles header files that reference external raw or compressed data
pub struct MhdParser {
    /// Validates MHD header format
    validator: MedicalImageValidator,
    /// Resolves data file paths relative to header
    path_resolver: PathBuf,
    /// Handles various data file formats
    data_loader: PathBuf,
}

impl MhdParser {
    /// Creates a new MHD parser instance
    pub fn new(
        validator: MedicalImageValidator,
        path_resolver: PathBuf,
        data_loader: PathBuf
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
        let mut mhd = MhdParser::new(MedicalImageValidator::new(), PathBuf::new(), path.clone());
        let mhd_path = mhd.data_loader.clone().join("CT.mhd");
        let bytes_mhd = tokio::fs::read(mhd_path).await?;
        let metadata = Self::parse_single_file(&bytes_mhd)?;
        let pixel_data = mhd.load_data_file(&metadata)?;
        MedicalVolume::new(metadata, pixel_data, ImageFormat::MHD)
    }
        
    /// Parses MHD header file from raw bytes and loads pixel data
    pub fn parse_by_bytes(mhd:&[u8],data: &[u8]) -> MedicalImagingResult<MedicalVolume>{
        let metadata = Self::parse_single_file(mhd)?;
        let pixel_data = PixelData::UInt8(data.to_vec());
        MedicalVolume::new(metadata, pixel_data, ImageFormat::MHD)
    }
    
    /// Loads pixel data from associated data file
    pub fn load_data_file(self, metadata: &ImageMetadata) -> MedicalImagingResult<PixelData>{
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
    pub fn parse_single_file(mhd_data: &[u8]) -> MedicalImagingResult<ImageMetadata> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_parse_by_bytes(){
        let path = "C:/share/input/CT.mhd";
        let mhd = fs::read(path);
        let bytes_mhd = mhd.as_ref().map(|v| v.as_slice()).unwrap();

        let data = fs::read(path.replace("mhd", "raw"));
        let bytes_data = data.as_ref().map(|v| v.as_slice()).unwrap();
        let volume = MhdParser::parse_by_bytes(bytes_mhd, bytes_data).unwrap();
        let header = volume.metadata;
        let pixel_data = volume.pixel_data;
        println!("=== MHDHeader 解析结果 ===");
        println!("维度 (DimSize): {:?}", header.dimensions);
        println!("体素间距 (ElementSpacing): {:?}", header.spacing);
        println!("数据类型 (ElementType): {:?}", header.pixel_type);
        println!("数据文件 (ElementDataFile): {}", header.element_data_file);
        println!("原点偏移 (Offset): {:?}", header.offset);
        println!("方向矩阵 (TransformMatrix): {:?}", header.orientation);
        println!("患者体位：{:?}",header.patient_position);
        println!("数据偏移 (data_offset，仅 .mha 有): {:?}", header.data_offset);
        println!("像素前20个数据: {:?}", &pixel_data.as_bytes()[..20]);
    }
}