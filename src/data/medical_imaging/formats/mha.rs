use crate::data::medical_imaging::{
    error::*, 
    get_header, 
    metadata::{Endianness, MedicalVolume,  ImageMetadata, PixelData}, 
    CompressionType,
    ImageFormat,
};
use std::collections::HashMap;
use std::path::PathBuf;

/// 功能级注释：解析包含嵌入式数据的 MHA（MetaImage）文件
/// 处理带有内联图像数据的 ASCII 和二进制头文件
pub struct MhaParser {
    /// 验证 MHA 文件签名和格式
    validaton: Option<String>,
    /// 处理不同的压缩方案
    compression_handler: CompressionType,
    /// 管理字节序转换
    endian_converter: Endianness,
}

impl MhaParser {
    /// 解析完整的 MHA 文件，包括头文件和嵌入式数据
    pub async fn parse_file(path: PathBuf) -> MedicalImagingResult<MedicalVolume>{
        let file = tokio::fs::read(path).await?;
        Self::parse_bytes(&file)
    }
    
    /// 从字节缓冲区解析 MHA，用于 WASM 兼容性
    pub fn parse_bytes(data: &[u8]) -> MedicalImagingResult<MedicalVolume>{
        let metadata = Self::parse_metadata_only(data)?;
        let start_offset = metadata.data_offset.unwrap_or(0);
        let raw = &data[start_offset..];
        let pixel_data = PixelData::UInt8(raw.to_vec());
        MedicalVolume::new(metadata, pixel_data, ImageFormat::MHA)
    }
    
    /// 仅提取元数据而不加载像素数据
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
        get_header(kv, data_offset)
    }
}