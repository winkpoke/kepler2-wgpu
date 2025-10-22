// Function-level comment: Patient position enumeration
// Represents different positions of the patient in the imaging setup

use crate::data::medical_imaging::{
    pixel_data::{PixelType, Endianness},
    formats::CompressionType,
    error::{MedicalImagingError, MedicalImagingResult},
};
use std::fmt;
use anyhow::{anyhow, Result};
use log::{warn, debug};
use std::collections::HashMap;

/// Comprehensive medical image metadata
/// Preserves all spatial and acquisition information critical for medical application
#[derive(Debug, Clone)]
pub struct ImageMetadata {
    /// Image dimensions [width, height, depth]
    pub dimensions: Vec<usize>,
    /// Pixel spacing in mm [x, y, z]
    pub spacing: Vec<f32>,
    /// Image origin in world coordinates [x, y, z]
    pub offset: Vec<f32>,
    /// Orientation matrix (3x3)
    pub orientation: [[f32; 3]; 3],
    /// Pixel data type
    pub pixel_type: PixelType,
    /// Endianness of pixel data
    pub endianness: Endianness,
    /// Compression type if any
    pub compression: Option<CompressionType>,
    /// Patient position
    pub patient_position: PatientPosition,
    /// Offset to pixel data in bytes from start of file
    pub data_offset: Option<usize>, 
    /// Path to element data file if any
    pub element_data_file: String,
}

impl ImageMetadata {
    /// Calculates total number of pixels
    pub fn total_pixels(&self) -> usize {
        self.dimensions.iter().product()
    }
    
    /// Converts world coordinates to voxel indices
    pub fn world_to_voxel(&self, world_pos: [f32; 3]) -> [f32; 3] {
        let mut voxel_pos = [0.0; 3];
        
        // Translate to origin
        let translated = [
            world_pos[0] - self.offset[0],
            world_pos[1] - self.offset[1],
            world_pos[2] - self.offset[2],
        ];
        
        // Apply inverse orientation and spacing
        for i in 0..3 {
            for j in 0..3 {
                voxel_pos[i] += self.orientation[j][i] * translated[j];
            }
            voxel_pos[i] /= self.spacing[i];
        }
        
        voxel_pos
    }
    
    /// Converts voxel indices to world coordinates
    pub fn voxel_to_world(&self, voxel_pos: [f32; 3]) -> [f32; 3] {
        let mut world_pos = [0.0; 3];
        
        // Apply spacing and orientation
        for i in 0..3 {
            for j in 0..3 {
                world_pos[i] += self.orientation[i][j] * (voxel_pos[j] * self.spacing[j]);
            }
            world_pos[i] += self.offset[i];
        }
        
        world_pos
    }

    // Parses a string of integers into a vector of usize
    fn parse_ints(s: &str) -> Result<Vec<usize>> {
        s.split_whitespace()
            .map(|x| x.parse().map_err(|e| anyhow!("parse int: {}", e)))
            .collect()
    }

    // Parses a string of floating-point numbers into a vector of f32
    fn parse_floats(s: &str) -> Result<Vec<f32>> {
        s.split_whitespace()
            .map(|x| x.parse().map_err(|e| anyhow!("parse float: {}", e)))
            .collect()
    }

    // Computes the orientation directions from a transform matrix
    fn orientation_dirs(transform: Vec<f32>) -> [[f32; 3]; 3] {
        match transform.len() {
            9 => {
                let col= [transform[0], transform[1], transform[2]]; // x 
                let row = [transform[3], transform[4], transform[5]]; // y 
                let slice = [transform[6], transform[7], transform[8]]; // z 
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

    // Extracts metadata from a MHX header
    pub fn get_header(kv: HashMap<String, String>, data_offset: Option<usize>) -> MedicalImagingResult<ImageMetadata>{
        let dim = Self::parse_ints(kv.get("DimSize")
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

        let spacing = Self::parse_floats(kv.get("ElementSpacing")
            .ok_or_else(|| MedicalImagingError::UnsupportedFormat {
                format: format!("ElementSpacing: {}", kv.get("ElementSpacing").unwrap_or(&"".to_string()))
            })?).map_err(|e| MedicalImagingError::UnsupportedFormat {
                format: format!("ElementSpacing: {}", e)
            })?;
        let offset = Self::parse_floats(kv.get("Offset")
            .ok_or_else(|| MedicalImagingError::UnsupportedFormat {
                format: format!("Offset: {}", kv.get("Offset").unwrap_or(&"".to_string()))
            })?).map_err(|e| MedicalImagingError::UnsupportedFormat {
                format: format!("Offset: {}", e)
            })?;
        let transform = Self::parse_floats(kv.get("TransformMatrix")
            .ok_or_else(|| MedicalImagingError::UnsupportedFormat {
                format: format!("TransformMatrix: {}", kv.get("TransformMatrix").unwrap_or(&"".to_string()))
            })?).map_err(|e| MedicalImagingError::UnsupportedFormat {
                format: format!("TransformMatrix: {}", e)
            })?;
        let orientation = Self::orientation_dirs(transform);

        let anatomical_orientation = kv.get("AnatomicalOrientation")
            .ok_or_else(|| MedicalImagingError::UnsupportedFormat {
                format: format!("AnatomicalOrientation: {}", kv.get("AnatomicalOrientation").unwrap_or(&"".to_string()))
            })?
            .to_string();

        let anatomical_orientation = anatomical_orientation.as_str();
        let patient_position = PatientPosition::from_str(anatomical_orientation);

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
}

/// Patient position enumeration
/// Represents different positions of the patient in the imaging setup
#[derive(Debug, Clone)]
pub enum  PatientPosition{
    HFS,
    HFP,
    FFS,
    FFP,
    HFDR,
    HFDL,
    FFDR,
    FFDL,
    Unknown,
}

impl PatientPosition{
    pub fn to_string(&self) -> String{
        match self {
            PatientPosition::HFS => "HFS".to_string(),
            PatientPosition::HFP => "HFP".to_string(),
            PatientPosition::FFS => "FFS".to_string(),
            PatientPosition::FFP => "FFP".to_string(),
            PatientPosition::HFDR => "HFDR".to_string(),
            PatientPosition::HFDL => "HFDL".to_string(),
            PatientPosition::FFDR => "FFDR".to_string(),
            PatientPosition::FFDL => "FFDL".to_string(),
            PatientPosition::Unknown => "Unknown".to_string(),
        }
    }

    pub fn from_str(anatomical_orientation: &str)-> Self{
        match anatomical_orientation {
            "HFS" => PatientPosition::HFS,  // Head First-Supine (头先进仰卧)
            "HFP" => PatientPosition::HFP,  // Head First-Prone (头先进俯卧) 
            "FFS" => PatientPosition::FFS,  // Feet First-Supine (脚先进仰卧)
            "FFP" => PatientPosition::FFP,  // Feet First-Prone (脚先进俯卧)
            "HFDR" => PatientPosition::HFDR, // Head First-Decubitus Right (头先进右侧卧)
            "HFDL" => PatientPosition::HFDL, // Head First-Decubitus Left (头先进左侧卧)
            "FFDR" => PatientPosition::FFDR, // Feet First-Decubitus Right (脚先进右侧卧)
            "FFDL" => PatientPosition::FFDL, // Feet First-Decubitus Left (脚先进左侧卧)
            // ========================
            // 解剖方向到标准体位的映射
            // ========================
            // 仰卧位 (Supine) - 头先进
            "RAI" => PatientPosition::HFS,  // 右前上 -> 头先进仰卧
            "LPS" => PatientPosition::HFS,  // 左后上 -> 头先进仰卧
            "LAI" => PatientPosition::HFS,  // 左前上 -> 头先进仰卧
            "RPS" => PatientPosition::HFS,  // 右后上 -> 头先进仰卧

            // 俯卧位 (Prone) - 头先进  
            "RPI" => PatientPosition::HFP,  // 右后上 -> 头先进俯卧
            "LAS" => PatientPosition::HFP,  // 左前下 -> 头先进俯卧
            "LPI" => PatientPosition::HFP,  // 左后上 -> 头先进俯卧
            "RAS" => PatientPosition::HFP,  // 右前下 -> 头先进俯卧

            // 仰卧位 (Supine) - 脚先进
            "RSA" => PatientPosition::FFS,  // 右上前 -> 脚先进仰卧
            "LSP" => PatientPosition::FFS,  // 左上后 -> 脚先进仰卧
            "LSA" => PatientPosition::FFS,  // 左上前 -> 脚先进仰卧
            "RSP" => PatientPosition::FFS,  // 右上后 -> 脚先进仰卧

            // 俯卧位 (Prone) - 脚先进
            "RPA" => PatientPosition::FFP,  // 右后前 -> 脚先进俯卧
            "LIA" => PatientPosition::FFP,  // 左下前 -> 脚先进俯卧
            "LPA" => PatientPosition::FFP,  // 左后前 -> 脚先进俯卧
            "RIA" => PatientPosition::FFP,  // 右下前 -> 脚先进俯卧

            // ========================
            // 侧卧位 (Decubitus)
            // ========================
            // 右侧卧位
            "ARI" => PatientPosition::HFDR, // 前右上 -> 头先进右侧卧
            "PRI" => PatientPosition::HFDR, // 后右上 -> 头先进右侧卧
            "ARS" => PatientPosition::FFDR, // 前右下 -> 脚先进右侧卧
            "PRS" => PatientPosition::FFDR, // 后右下 -> 脚先进右侧卧

            // 左侧卧位
            "ALI" => PatientPosition::HFDL, // 前左上 -> 头先进左侧卧
            "PLI" => PatientPosition::HFDL, // 后左上 -> 头先进左侧卧
            "ALS" => PatientPosition::FFDL, // 前左下 -> 脚先进左侧卧
            "PLS" => PatientPosition::FFDL, // 后左下 -> 脚先进左侧卧

            // ========================
            // 特殊情况
            // ========================
            "AIL" => PatientPosition::HFS,  // 前上左 -> 头先进仰卧
            "PIL" => PatientPosition::HFS,  // 后上左 -> 头先进仰卧
            "AIR" => PatientPosition::HFS,  // 前上右 -> 头先进仰卧
            "PIR" => PatientPosition::HFS,  // 后上右 -> 头先进仰卧

            // ========================
            // 默认情况
            // ========================
            _ => {
                log::info!("Unknown anatomical orientation: {}, defaulting to HFS", anatomical_orientation);
                PatientPosition::HFS
            }
        }
    }

    /// Validate patient position consistency with image orientation
    pub fn validate_position_consistency(
        position: &PatientPosition,
        image_orientation: Option<(f32, f32, f32, f32, f32, f32)>
    ) -> Result<()> {
        if let Some(orientation) = image_orientation {
            let (row_x, row_y, row_z, col_x, col_y, col_z) = orientation;
            
                        // Calculate slice direction (cross product of row and column directions)
            let slice_x = row_y * col_z - row_z * col_y;
            let slice_y = row_z * col_x - row_x * col_z;
            let slice_z = row_x * col_y - row_y * col_x;
            
            debug!("Image orientation - Row: ({:.3}, {:.3}, {:.3}), Col: ({:.3}, {:.3}, {:.3}), Slice: ({:.3}, {:.3}, {:.3})",
                row_x, row_y, row_z, col_x, col_y, col_z, slice_x, slice_y, slice_z);

            // Validate consistency based on expected orientations for each position
            match position {
                PatientPosition::HFS => {
                    // Head First Supine: expect slice direction pointing superior-inferior
                    if slice_z.abs() < 0.5 {
                        warn!("PatientPosition HFS but slice direction doesn't align with S-I axis");
                    }
                }
                PatientPosition::HFP => {
                    // Head First Prone: expect slice direction pointing superior-inferior
                    if slice_z.abs() < 0.5 {
                        warn!("PatientPosition HFP but slice direction doesn't align with S-I axis");
                    }
                }
                PatientPosition::FFS | PatientPosition::FFP => {
                    // Feet First: expect slice direction pointing inferior-superior
                    if slice_z.abs() < 0.5 {
                        warn!("PatientPosition feet-first but slice direction doesn't align with I-S axis");
                    }
                }
                PatientPosition::HFDR | PatientPosition::HFDL | 
                PatientPosition::FFDR | PatientPosition::FFDL => {
                    // Decubitus positions: expect slice direction in lateral plane
                    if slice_x.abs() < 0.5 && slice_y.abs() < 0.5 {
                        warn!("PatientPosition decubitus but slice direction doesn't align with lateral axis");
                    }
                }
                PatientPosition::Unknown => {
                    warn!("Unknown patient position, cannot validate orientation consistency");
                }
            }
        }
        
        Ok(())
    }

    /// Get expected coordinate system transformation for a given patient position
    pub fn get_coordinate_transform(position: &PatientPosition) -> (bool, bool, bool) {
        match position {
            PatientPosition::HFS => (false, false, false), // Standard orientation
            PatientPosition::HFP => (true, false, false),  // Flip X for prone
            PatientPosition::FFS => (false, true, true),   // Flip Y and Z for feet first
            PatientPosition::FFP => (true, true, true),    // Flip all for feet first prone
            PatientPosition::HFDR => (false, true, false), // Flip Y for right decubitus
            PatientPosition::HFDL => (false, false, false), // No flip for left decubitus
            PatientPosition::FFDR => (false, true, true),  // Flip Y and Z for feet first right
            PatientPosition::FFDL => (false, false, true), // Flip Z for feet first left
            PatientPosition::Unknown => {
                warn!("Unknown patient position, using default coordinate transform");
                (false, false, false)
            }
        }
    }
}

impl fmt::Display for PatientPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            PatientPosition::HFS => "HFS",
            PatientPosition::HFP => "HFP",
            PatientPosition::FFS => "FFS",
            PatientPosition::FFP => "FFP",
            PatientPosition::HFDR => "HFDR",
            PatientPosition::HFDL => "HFDL",
            PatientPosition::FFDR => "FFDR",
            PatientPosition::FFDL => "FFDL",
            PatientPosition::Unknown => "Unknown",
        };
        write!(f, "{}", s)
    }
}