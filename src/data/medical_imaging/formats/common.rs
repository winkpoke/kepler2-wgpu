/// Common traits and interfaces for medical image format parsers
/// Provides unified API for parsing different medical imaging formats

use crate::data::medical_imaging::{
    metadata::*,
    error::{MedicalImagingError, MedicalImagingResult},
    mhd::MhdParser,
    mha::MhaParser,
};
use std::path::Path;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;

/// Image format enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    MHA,
    MHD,
    NIfTI,
    DICOM,
    Unknown,
}

pub trait MedicalImageParser {
    /// 解析完整的医学影像文件
    #[cfg(not(target_arch = "wasm32"))]
    async fn parse(&self, path: PathBuf) -> MedicalImagingResult<MedicalVolume>{
        let file_path: &str = path.to_str().ok_or_else(|| MedicalImagingError::InvalidPath {path: path.display().to_string()})?;
        let format = get_extension(file_path).unwrap_or(ImageFormat::Unknown);
        match format {
            ImageFormat::MHA => MhaParser::parse_file(path).await,
            ImageFormat::MHD => MhdParser::parse_file(path).await,
            _ => Err(MedicalImagingError::UnsupportedFormat {
                format: format!("file extension: {}", file_path)
            }),
        }
    }
    
    /// 提取元数据而不包含像素数据
    fn extract_metadata(&self, path: &[u8], format: ImageFormat) -> MedicalImagingResult<ImageMetadata>{
        match format {
            ImageFormat::MHA => {
                MhaParser::parse_metadata_only(path)
            },
            ImageFormat::MHD => {
                MhdParser::parse_single_file(path)
            },
            _ => Err(MedicalImagingError::UnsupportedFormat {
                format: format!("file extension: {:?}", format)
            }),
        }
    }
}

pub fn get_header(kv: HashMap<String, String>, data_offset: Option<usize>) -> MedicalImagingResult<ImageMetadata>{
    let dim = parse_ints(kv.get("DimSize")
        .ok_or_else(|| MedicalImagingError::UnsupportedFormat {
            format: format!("DimSize: {}", kv.get("DimSize").unwrap_or(&"".to_string()))
        })?).map_err(|e| MedicalImagingError::UnsupportedFormat {
            format: format!("DimSize: {}", e)
        })?;

    let element_type = kv
        .get("ElementType")
        .ok_or_else(|| MedicalImagingError::UnsupportedFormat {
            format: format!("ElementType: {}", kv.get("ElementType").unwrap_or(&"".to_string()))
        })?
        .to_string();
    let pixel_type = match element_type.as_str() {
        "MET_UCHAR" => PixelType::UInt8,
        "MET_USHORT" => PixelType::UInt16,
        "MET_SHORT" => PixelType::Int16,
        "MET_INT" => PixelType::Int32,
        "MET_FLOAT" => PixelType::Float32,
        "MET_DOUBLE" => PixelType::Float64,
        _ => return Err(MedicalImagingError::UnsupportedFormat {
            format: format!("pixel type: {}", element_type)
        }),
    };

    let spacing = parse_floats(kv.get("ElementSpacing")
        .ok_or_else(|| MedicalImagingError::UnsupportedFormat {
            format: format!("ElementSpacing: {}", kv.get("ElementSpacing").unwrap_or(&"".to_string()))
        })?).map_err(|e| MedicalImagingError::UnsupportedFormat {
            format: format!("ElementSpacing: {}", e)
        })?;
    let offset = parse_floats(kv.get("Offset")
        .ok_or_else(|| MedicalImagingError::UnsupportedFormat {
            format: format!("Offset: {}", kv.get("Offset").unwrap_or(&"".to_string()))
        })?).map_err(|e| MedicalImagingError::UnsupportedFormat {
            format: format!("Offset: {}", e)
        })?;
    let transform = parse_floats(kv.get("TransformMatrix")
        .ok_or_else(|| MedicalImagingError::UnsupportedFormat {
            format: format!("TransformMatrix: {}", kv.get("TransformMatrix").unwrap_or(&"".to_string()))
        })?).map_err(|e| MedicalImagingError::UnsupportedFormat {
            format: format!("TransformMatrix: {}", e)
        })?;
    let orientation = orientation_dirs(transform);

    let anatomical_orientation = kv.get("AnatomicalOrientation")
        .ok_or_else(|| MedicalImagingError::UnsupportedFormat {
            format: format!("AnatomicalOrientation: {}", kv.get("AnatomicalOrientation").unwrap_or(&"".to_string()))
        })?
        .to_string();

    let anatomical_orientation = anatomical_orientation.as_str();
    let patient_position = create_patient_position(anatomical_orientation);

    let element_data_file = kv
        .get("ElementDataFile")
        .ok_or_else(|| MedicalImagingError::UnsupportedFormat {
            format: format!("ElementDataFile: {}", kv.get("ElementDataFile").unwrap_or(&"".to_string()))
        })?
        .to_string();

    Ok(ImageMetadata {
        dimensions: dim,
        spacing,
        offset,
        orientation,
        pixel_type,
        endianness: Endianness::Little,
        patient_position,
        compression: None,
        data_offset,
        element_data_file,
    })
}

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

fn parse_ints(s: &str) -> Result<Vec<usize>> {
    s.split_whitespace()
        .map(|x| x.parse().map_err(|e| anyhow!("parse int: {}", e)))
        .collect()
}

fn parse_floats(s: &str) -> Result<Vec<f32>> {
    s.split_whitespace()
        .map(|x| x.parse().map_err(|e| anyhow!("parse float: {}", e)))
        .collect()
}

pub fn orientation_dirs(transform: Vec<f32>) -> [[f32; 3]; 3] {
    match transform.len() {
        9 => {
            let col= [transform[0], transform[1], transform[2]]; // x 轴
            let row = [transform[3], transform[4], transform[5]]; // y 轴
            let slice = [transform[6], transform[7], transform[8]]; // z 轴
            [col, row, slice]
        }
        6 => {
            let col = [transform[0], transform[1], transform[2]];
            let row = [transform[3], transform[4], transform[5]];
            let slice = [0.0, 0.0, 1.0];
            [col, row, slice]
        }
        _ => [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    }
}


#[derive(Debug, Clone, PartialEq)]
pub enum CompressionType {
    GZip,
    ZLib,
    Raw,
}
