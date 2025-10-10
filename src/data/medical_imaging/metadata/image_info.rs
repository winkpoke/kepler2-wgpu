use std::collections::HashMap;
use crate::data::medical_imaging::{
    validation::{ValidationResult, ValidationError},
    pixel_data::{PixelType, Endianness},
    formats::CompressionType,
};

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

/// Comprehensive medical image metadata
/// Preserves all spatial and acquisition information critical for medical application
#[derive(Debug, Clone, PartialEq)]pub struct ImageMetadata {
    /// Image dimensions [width, height, depth]
    pub dimensions: [usize; 3],
    /// Pixel spacing in mm [x, y, z]
    pub spacing: [f64; 3],
    /// Image origin in world coordinates [x, y, z]
    pub origin: [f64; 3],
    /// Orientation matrix (3x3)
    pub orientation: [[f64; 3]; 3],
    /// Pixel data type
    pub pixel_type: PixelType,
    /// Endianness of pixel data
    pub endianness: Endianness,
    /// Compression type if any
    pub compression: Option<CompressionType>,
    /// Patient position
    pub patient_position: PatientPosition,
}

impl ImageMetadata {
    /// Validates metadata consistency
    pub fn validate(&self) -> ValidationResult {
        let mut errors = Vec::new();

        // Validate dimensions
        for (i, &dim) in self.dimensions.iter().enumerate() {
            if dim == 0 {
                errors.push(ValidationError::InvalidDimension {
                    axis: match i {
                        0 => "width",
                        1 => "height",
                        2 => "depth",
                        _ => "unknown",
                    },
                    value: dim,
                });
            }
        }

        // Validate spacing
        for (i, &sp) in self.spacing.iter().enumerate() {
            if sp <= 0.0 {
                errors.push(ValidationError::InvalidSpacing {
                    axis: match i {
                        0 => "x",
                        1 => "y",
                        2 => "z",
                        _ => "unknown",
                    },
                    value: sp,
                });
            }
        }

        // Validate orientation matrix (must be orthonormal)
        let det = self.orientation[0][0] * (self.orientation[1][1] * self.orientation[2][2] - self.orientation[1][2] * self.orientation[2][1])
                - self.orientation[0][1] * (self.orientation[1][0] * self.orientation[2][2] - self.orientation[1][2] * self.orientation[2][0])
                + self.orientation[0][2] * (self.orientation[1][0] * self.orientation[2][1] - self.orientation[1][1] * self.orientation[2][0]);
        if det.abs() < 1e-6 {
            errors.push(ValidationError::InvalidOrientation);
        }

        if errors.is_empty() {
            ValidationResult::success()
        } else {
            ValidationResult::failure(errors)
        }
    }
    
    /// Calculates total number of pixels
    pub fn total_pixels(&self) -> usize {
        self.dimensions.iter().product()
    }
    
    /// Converts world coordinates to voxel indices
    pub fn world_to_voxel(&self, world_pos: [f64; 3]) -> [f64; 3] {
        let mut voxel_pos = [0.0; 3];
        
        // Translate to origin
        let translated = [
            world_pos[0] - self.origin[0],
            world_pos[1] - self.origin[1],
            world_pos[2] - self.origin[2],
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
    pub fn voxel_to_world(&self, voxel_pos: [f64; 3]) -> [f64; 3] {
        let mut world_pos = [0.0; 3];
        
        // Apply spacing and orientation
        for i in 0..3 {
            for j in 0..3 {
                world_pos[i] += self.orientation[i][j] * (voxel_pos[j] * self.spacing[j]);
            }
            world_pos[i] += self.origin[i];
        }
        
        world_pos
    }
}

fn create_patient_position(anatomical_orientation: &str)-> PatientPosition{
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