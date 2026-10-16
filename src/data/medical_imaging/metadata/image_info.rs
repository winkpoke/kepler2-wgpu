// Function-level comment: Patient position enumeration
// Represents different positions of the patient in the imaging setup

use crate::data::medical_imaging::{
    error::{MedicalImagingError, MedicalImagingResult},
    formats::CompressionType,
    pixel_data::{Endianness, PixelType},
};
use anyhow::{anyhow, Result};
use log::{debug, warn};
use std::collections::HashMap;
use std::fmt;

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

    // Pads a 2-element vector (from a 2D header) to 3D with the given z value.
    fn pad_2d_to_3d(mut v: Vec<f32>, z: f32) -> Vec<f32> {
        if v.len() == 2 {
            v.push(z);
        }
        v
    }

    // Computes the orientation directions from a transform matrix
    fn orientation_dirs(transform: Vec<f32>) -> [[f32; 3]; 3] {
        match transform.len() {
            9 => {
                let col = [transform[0], transform[1], transform[2]]; // x
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
    pub fn get_header(
        kv: HashMap<String, String>,
        data_offset: Option<usize>,
    ) -> MedicalImagingResult<ImageMetadata> {
        let dim = Self::parse_ints(kv.get("DimSize").ok_or_else(|| {
            MedicalImagingError::UnsupportedFormat {
                format: format!("DimSize: {}", kv.get("DimSize").unwrap_or(&"".to_string())),
            }
        })?)
        .map_err(|e| MedicalImagingError::UnsupportedFormat {
            format: format!("DimSize: {}", e),
        })?;

        let dim = if dim.len() == 2 {
            vec![dim[0], dim[1], 1]
        } else {
            dim
        };

        let element_type = kv
            .get("ElementType")
            .ok_or_else(|| MedicalImagingError::UnsupportedFormat {
                format: format!(
                    "ElementType: {}",
                    kv.get("ElementType").unwrap_or(&"".to_string())
                ),
            })?
            .to_string();
        let pixel_type = match element_type.as_str() {
            "MET_UCHAR" | "MET_CHAR" => PixelType::UInt8,
            "MET_USHORT" => PixelType::UInt16,
            "MET_SHORT" => PixelType::Int16,
            "MET_INT" | "MET_UINT" | "MET_LONG" | "MET_ULONG" => PixelType::Int32,
            "MET_FLOAT" => PixelType::Float32,
            "MET_DOUBLE" => PixelType::Float64,
            _ => {
                return Err(MedicalImagingError::UnsupportedFormat {
                    format: format!("pixel type: {}", element_type),
                })
            }
        };

        let spacing = match kv.get("ElementSpacing") {
            Some(v) => Self::parse_floats(v).map_err(|e| {
                MedicalImagingError::UnsupportedFormat {
                    format: format!("ElementSpacing: {}", e),
                }
            })?,
            None => {
                warn!("MHA header missing ElementSpacing, defaulting spacing to (1, 1, 1)");
                vec![1.0, 1.0, 1.0]
            }
        };

        let spacing = Self::pad_2d_to_3d(spacing, 1.0);
        let offset = match kv.get("Offset") {
            Some(v) => Self::parse_floats(v).map_err(|e| {
                MedicalImagingError::UnsupportedFormat {
                    format: format!("Offset: {}", e),
                }
            })?,
            None => {
                warn!("MHA header missing Offset, defaulting offset to (0.417, 0.417, 0.0)");
                vec![0.417, 0.417, 0.0]
            }
        };

        let offset = Self::pad_2d_to_3d(offset, 0.0);
        let transform = match kv.get("TransformMatrix") {
            Some(v) => Self::parse_floats(v).map_err(|e| {
                MedicalImagingError::UnsupportedFormat {
                    format: format!("TransformMatrix: {}", e),
                }
            })?,
            None => {
                warn!("MHA header missing TransformMatrix, defaulting to identity");
                vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
            }
        };
        let orientation = Self::orientation_dirs(transform);

        let patient_position = match kv.get("AnatomicalOrientation") {
            Some(v) => PatientPosition::from_str(v),
            None => {
                warn!("MHA header missing AnatomicalOrientation, defaulting to HFS");
                PatientPosition::HFS
            }
        };

        let element_data_file = kv
            .get("ElementDataFile")
            .ok_or_else(|| MedicalImagingError::UnsupportedFormat {
                format: format!(
                    "ElementDataFile: {}",
                    kv.get("ElementDataFile").unwrap_or(&"".to_string())
                ),
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
/// Represents different positions of patient in imaging setup
#[derive(Debug, Clone, PartialEq)]
pub enum PatientPosition {
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

impl PatientPosition {
    pub fn to_string(&self) -> String {
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

    pub fn from_str(anatomical_orientation: &str) -> Self {
        match anatomical_orientation {
            "HFS" => PatientPosition::HFS,   // Head First-Supine (头先进仰卧)
            "HFP" => PatientPosition::HFP,   // Head First-Prone (头先进俯卧)
            "FFS" => PatientPosition::FFS,   // Feet First-Supine (脚先进仰卧)
            "FFP" => PatientPosition::FFP,   // Feet First-Prone (脚先进俯卧)
            "HFDR" => PatientPosition::HFDR, // Head First-Decubitus Right (头先进右侧卧)
            "HFDL" => PatientPosition::HFDL, // Head First-Decubitus Left (头先进左侧卧)
            "FFDR" => PatientPosition::FFDR, // Feet First-Decubitus Right (脚先进右侧卧)
            "FFDL" => PatientPosition::FFDL, // Feet First-Decubitus Left (脚先进左侧卧)
            // ========================
            // 解剖方向到标准体位的映射
            // ========================
            // 仰卧位 (Supine) - 头先进
            "RAI" => PatientPosition::HFS, // 右前上 -> 头先进仰卧
            "LPS" => PatientPosition::HFS, // 左后上 -> 头先进仰卧
            "LAI" => PatientPosition::HFS, // 左前上 -> 头先进仰卧
            "RPS" => PatientPosition::HFS, // 右后上 -> 头先进仰卧

            // 俯卧位 (Prone) - 头先进
            "RPI" => PatientPosition::HFP, // 右后上 -> 头先进俯卧
            "LAS" => PatientPosition::HFP, // 左前下 -> 头先进俯卧
            "LPI" => PatientPosition::HFP, // 左后上 -> 头先进俯卧
            "RAS" => PatientPosition::HFP, // 右前下 -> 头先进俯卧

            // 仰卧位 (Supine) - 脚先进
            "RSA" => PatientPosition::FFS, // 右上前 -> 脚先进仰卧
            "LSP" => PatientPosition::FFS, // 左上后 -> 脚先进仰卧
            "LSA" => PatientPosition::FFS, // 左上前 -> 脚先进仰卧
            "RSP" => PatientPosition::FFS, // 右上后 -> 脚先进仰卧

            // 俯卧位 (Prone) - 脚先进
            "RPA" => PatientPosition::FFP, // 右后前 -> 脚先进俯卧
            "LIA" => PatientPosition::FFP, // 左下前 -> 脚先进俯卧
            "LPA" => PatientPosition::FFP, // 左后前 -> 脚先进俯卧
            "RIA" => PatientPosition::FFP, // 右下前 -> 脚先进俯卧

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
            "AIL" => PatientPosition::HFS, // 前上左 -> 头先进仰卧
            "PIL" => PatientPosition::HFS, // 后上左 -> 头先进仰卧
            "AIR" => PatientPosition::HFS, // 前上右 -> 头先进仰卧
            "PIR" => PatientPosition::HFS, // 后上右 -> 头先进仰卧

            // ========================
            // 默认情况
            // ========================
            _ => {
                log::info!(
                    "Unknown anatomical orientation: {}, defaulting to HFS",
                    anatomical_orientation
                );
                PatientPosition::HFS
            }
        }
    }

    /// Validate patient position consistency with image orientation
    pub fn validate_position_consistency(
        position: &PatientPosition,
        image_orientation: Option<(f32, f32, f32, f32, f32, f32)>,
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
                        warn!(
                            "PatientPosition HFS but slice direction doesn't align with S-I axis"
                        );
                    }
                }
                PatientPosition::HFP => {
                    // Head First Prone: expect slice direction pointing superior-inferior
                    if slice_z.abs() < 0.5 {
                        warn!(
                            "PatientPosition HFP but slice direction doesn't align with S-I axis"
                        );
                    }
                }
                PatientPosition::FFS | PatientPosition::FFP => {
                    // Feet First: expect slice direction pointing inferior-superior
                    if slice_z.abs() < 0.5 {
                        warn!("PatientPosition feet-first but slice direction doesn't align with I-S axis");
                    }
                }
                PatientPosition::HFDR
                | PatientPosition::HFDL
                | PatientPosition::FFDR
                | PatientPosition::FFDL => {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn kv(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    /// 最小合法头：仅含三个必需键（ElementSpacing/Offset/TransformMatrix/
    /// AnatomicalOrientation 全部走默认值分支）
    fn minimal() -> HashMap<String, String> {
        kv(&[
            ("DimSize", "4 4 2"),
            ("ElementType", "MET_SHORT"),
            ("ElementDataFile", "LOCAL"),
        ])
    }

    fn assert_ok(m: MedicalImagingResult<ImageMetadata>) -> ImageMetadata {
        match m {
            Ok(meta) => meta,
            Err(e) => panic!("expected Ok, got: {e:?}"),
        }
    }

    fn assert_err_contains(m: MedicalImagingResult<ImageMetadata>, needle: &str) {
        match m {
            Ok(meta) => panic!("expected Err containing {needle:?}, got Ok: {meta:?}"),
            Err(e) => {
                let msg = format!("{e:?}");
                assert!(
                    msg.contains(needle),
                    "error message {msg:?} does not contain {needle:?}"
                );
            }
        }
    }

    // ---------- 正常路径 ----------

    #[test]
    fn minimal_header_parses_with_documented_defaults() {
        let meta = assert_ok(ImageMetadata::get_header(minimal(), None));
        assert_eq!(meta.dimensions, vec![4, 4, 2]);
        assert_eq!(meta.pixel_type, PixelType::Int16);
        assert_eq!(meta.element_data_file, "LOCAL");
        // 默认值契约（有意锁定，勿改动断言前先确认契约）：
        assert_eq!(meta.spacing, vec![1.0, 1.0, 1.0]);
        assert_eq!(meta.offset, vec![0.417, 0.417, 0.0]); // BUG-01 语义：保留 0.417（已确认）
        assert_eq!(
            meta.orientation,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        );
        assert_eq!(meta.patient_position, PatientPosition::HFS);
        assert_eq!(meta.endianness, Endianness::Little);
        assert_eq!(meta.compression, None);
        assert_eq!(meta.data_offset, None);
    }

    #[test]
    fn full_header_all_fields_round_trip() {
        let header = kv(&[
            ("DimSize", "256 256 64"),
            ("ElementType", "MET_FLOAT"),
            ("ElementSpacing", "0.5 0.5 1.0"),
            ("Offset", "-128.0 -128.0 0.0"),
            ("TransformMatrix", "1 0 0 0 1 0 0 0 1"),
            ("AnatomicalOrientation", "RAI"),
            ("ElementDataFile", "volume.raw"),
        ]);
        let meta = assert_ok(ImageMetadata::get_header(header, Some(512)));
        assert_eq!(meta.dimensions, vec![256, 256, 64]);
        assert_eq!(meta.pixel_type, PixelType::Float32);
        assert_eq!(meta.spacing, vec![0.5, 0.5, 1.0]);
        assert_eq!(meta.offset, vec![-128.0, -128.0, 0.0]);
        assert_eq!(meta.patient_position, PatientPosition::HFS); // RAI -> HFS
        assert_eq!(meta.element_data_file, "volume.raw");
        assert_eq!(meta.data_offset, Some(512));
        assert_eq!(meta.total_pixels(), 256 * 256 * 64);
    }

    #[test]
    fn element_type_mapping_covers_metaio_set() {
        let cases: &[(&str, PixelType)] = &[
            ("MET_UCHAR", PixelType::UInt8),
            ("MET_CHAR", PixelType::UInt8), // BUG-02 修复：MET_CHAR 补映射
            ("MET_USHORT", PixelType::UInt16),
            ("MET_SHORT", PixelType::Int16),
            ("MET_INT", PixelType::Int32),
            ("MET_FLOAT", PixelType::Float32),
            ("MET_DOUBLE", PixelType::Float64),
        ];
        for (code, expected) in cases {
            let mut header = minimal();
            header.insert("ElementType".to_string(), code.to_string());
            let meta = assert_ok(ImageMetadata::get_header(header, None));
            assert_eq!(meta.pixel_type, *expected, "ElementType = {code}");
        }
    }

    #[test]
    fn transform_matrix_nine_elements_split_into_direction_rows() {
        let mut header = minimal();
        header.insert(
            "TransformMatrix".to_string(),
            "1 2 3 4 5 6 7 8 9".to_string(),
        );
        let meta = assert_ok(ImageMetadata::get_header(header, None));
        assert_eq!(meta.orientation[0], [1.0, 2.0, 3.0]);
        assert_eq!(meta.orientation[1], [4.0, 5.0, 6.0]);
        assert_eq!(meta.orientation[2], [7.0, 8.0, 9.0]);
    }

    #[test]
    fn transform_matrix_six_elements_pad_slice_axis() {
        let mut header = minimal();
        header.insert("TransformMatrix".to_string(), "0 1 0 1 0 0".to_string());
        let meta = assert_ok(ImageMetadata::get_header(header, None));
        assert_eq!(meta.orientation[0], [0.0, 1.0, 0.0]);
        assert_eq!(meta.orientation[1], [1.0, 0.0, 0.0]);
        assert_eq!(meta.orientation[2], [0.0, 0.0, 1.0]);
    }

    #[test]
    fn transform_matrix_invalid_length_falls_back_to_identity() {
        let mut header = minimal();
        header.insert(
            "TransformMatrix".to_string(),
            "1 0 0 0 1 0 0 0".to_string(), // 8 个元素 → 非 9/6
        );
        let meta = assert_ok(ImageMetadata::get_header(header, None));
        assert_eq!(
            meta.orientation,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        );
    }

    #[test]
    fn data_offset_is_passed_through() {
        let meta = assert_ok(ImageMetadata::get_header(minimal(), Some(1024)));
        assert_eq!(meta.data_offset, Some(1024));
    }

    #[test]
    fn all_standard_position_codes_map_correctly() {
        let codes = [
            ("HFS", PatientPosition::HFS),
            ("HFP", PatientPosition::HFP),
            ("FFS", PatientPosition::FFS),
            ("FFP", PatientPosition::FFP),
            ("HFDR", PatientPosition::HFDR),
            ("HFDL", PatientPosition::HFDL),
            ("FFDR", PatientPosition::FFDR),
            ("FFDL", PatientPosition::FFDL),
        ];
        for (code, expected) in codes {
            let mut header = minimal();
            header.insert("AnatomicalOrientation".to_string(), code.to_string());
            let meta = assert_ok(ImageMetadata::get_header(header, None));
            assert_eq!(meta.patient_position, expected, "code = {code}");
        }
    }

    #[test]
    fn anatomical_orientation_codes_map_to_positions() {
        let cases = [
            ("RAI", PatientPosition::HFS),
            ("LPS", PatientPosition::HFS),
            ("RPI", PatientPosition::HFP),
            ("RSA", PatientPosition::FFS),
            ("RPA", PatientPosition::FFP),
            ("ARI", PatientPosition::HFDR),
            ("ALI", PatientPosition::HFDL),
            ("ALS", PatientPosition::FFDL),
        ];
        for (code, expected) in cases {
            let mut header = minimal();
            header.insert("AnatomicalOrientation".to_string(), code.to_string());
            let meta = assert_ok(ImageMetadata::get_header(header, None));
            assert_eq!(meta.patient_position, expected, "code = {code}");
        }
    }

    #[test]
    fn world_voxel_roundtrip_with_identity_transform() {
        let meta = assert_ok(ImageMetadata::get_header(minimal(), None));
        let world = [1.5, 2.5, 3.5];
        let voxel = meta.world_to_voxel(world);
        let back = meta.voxel_to_world(voxel);
        for i in 0..3 {
            assert!((back[i] - world[i]).abs() < 1e-4, "axis {i}: {back:?}");
        }
    }

    // ---------- 拒绝路径 ----------

    #[test]
    fn missing_dimsize_is_rejected() {
        let mut header = minimal();
        header.remove("DimSize");
        assert_err_contains(ImageMetadata::get_header(header, None), "DimSize");
    }

    #[test]
    fn non_numeric_dimsize_is_rejected() {
        let mut header = minimal();
        header.insert("DimSize".to_string(), "a b c".to_string());
        assert_err_contains(ImageMetadata::get_header(header, None), "DimSize");
    }

    #[test]
    fn negative_dimsize_is_rejected() {
        let mut header = minimal();
        header.insert("DimSize".to_string(), "-4 -4 -2".to_string());
        assert_err_contains(ImageMetadata::get_header(header, None), "DimSize");
    }

    #[test]
    fn missing_element_type_is_rejected() {
        let mut header = minimal();
        header.remove("ElementType");
        assert_err_contains(ImageMetadata::get_header(header, None), "ElementType");
    }

    #[test]
    fn unknown_element_type_is_rejected() {
        for code in ["MET_DOUBLE_ARRAY", "MET_FLOAT_VECTOR", "garbage", ""] {
            let mut header = minimal();
            header.insert("ElementType".to_string(), code.to_string());
            assert_err_contains(ImageMetadata::get_header(header, None), "pixel type");
        }
    }

    #[test]
    fn missing_element_data_file_is_rejected() {
        let mut header = minimal();
        header.remove("ElementDataFile");
        assert_err_contains(ImageMetadata::get_header(header, None), "ElementDataFile");
    }

    #[test]
    fn non_numeric_spacing_is_rejected() {
        let mut header = minimal();
        header.insert("ElementSpacing".to_string(), "x y z".to_string());
        assert_err_contains(ImageMetadata::get_header(header, None), "ElementSpacing");
    }

    #[test]
    fn non_numeric_offset_is_rejected() {
        let mut header = minimal();
        header.insert("Offset".to_string(), "1.0 oops 3.0".to_string());
        assert_err_contains(ImageMetadata::get_header(header, None), "Offset");
    }

    #[test]
    fn non_numeric_transform_matrix_is_rejected() {
        let mut header = minimal();
        header.insert("TransformMatrix".to_string(), "1 0 zero 0 1 0 0 0 1".to_string());
        assert_err_contains(ImageMetadata::get_header(header, None), "TransformMatrix");
    }

    // ---------- 默认值与已确认行为 ----------

    #[test]
    fn missing_spacing_defaults_to_unit_spacing() {
        let meta = assert_ok(ImageMetadata::get_header(minimal(), None));
        assert_eq!(meta.spacing, vec![1.0, 1.0, 1.0]);
    }

    #[test]
    fn missing_offset_defaults_to_documented_fixed_value() {
        // 语义已确认（BUG-01）：Offset 缺失保留 (0.417, 0.417, 0.0)，
        // warn 文案已同步为该值。此测试锁定该契约。
        let meta = assert_ok(ImageMetadata::get_header(minimal(), None));
        assert_eq!(meta.offset, vec![0.417, 0.417, 0.0]);
    }

    #[test]
    fn missing_transform_defaults_to_identity() {
        let meta = assert_ok(ImageMetadata::get_header(minimal(), None));
        assert_eq!(
            meta.orientation,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        );
    }

    #[test]
    fn missing_orientation_defaults_to_hfs() {
        let meta = assert_ok(ImageMetadata::get_header(minimal(), None));
        assert_eq!(meta.patient_position, PatientPosition::HFS);
    }

    #[test]
    fn unknown_orientation_code_defaults_to_hfs() {
        let mut header = minimal();
        header.insert("AnatomicalOrientation".to_string(), "XYZ".to_string());
        let meta = assert_ok(ImageMetadata::get_header(header, None));
        assert_eq!(meta.patient_position, PatientPosition::HFS);
    }

    // ---------- 边界行为（按决策：不收紧校验，仅锁定现状并留注释） ----------

    #[test]
    fn dimsize_with_one_element_is_accepted_unpadded() {
        let mut header = minimal();
        header.insert("DimSize".to_string(), "4".to_string());
        let meta = assert_ok(ImageMetadata::get_header(header, None));
        assert_eq!(meta.dimensions, vec![4]);
        assert_eq!(meta.total_pixels(), 4);
    }

    #[test]
    fn spacing_with_two_elements_pads_z_for_2d_headers() {
        let mut header = minimal();
        header.insert("ElementSpacing".to_string(), "0.5 0.5".to_string());
        let meta = assert_ok(ImageMetadata::get_header(header, None));
        assert_eq!(meta.spacing, vec![0.5, 0.5, 1.0]);
    }

    #[test]
    fn offset_with_two_elements_pads_z_for_2d_headers() {
        let mut header = minimal();
        header.insert("Offset".to_string(), "1.0 2.0".to_string());
        let meta = assert_ok(ImageMetadata::get_header(header, None));
        assert_eq!(meta.offset, vec![1.0, 2.0, 0.0]);
    }
}
