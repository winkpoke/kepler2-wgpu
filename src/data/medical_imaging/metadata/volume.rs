use crate::data::medical_imaging::{
    metadata::ImageMetadata,
    pixel_data::{PixelData, PixelType},
    formats::ImageFormat,
    error:: MedicalImagingResult,
    validation::{ValidationResult, MedicalImageValidator},
};
use crate::data::CTVolume;
use crate::core::coord::Base;
use glam::{Mat4, Vec3};

/// Function-level comment: Standardized 3D medical volume representation
/// Preserves all metadata and provides efficient access to pixel data
#[derive(Debug, Clone)]
pub struct MedicalVolume {
    /// Image metadata including spatial information
    pub metadata: ImageMetadata,
    /// Pixel data with type-safe access
    pub pixel_data: PixelData,
    /// Original file format for provenance tracking
    pub source_format: ImageFormat,
    /// Validation status and integrity checks
    pub validation_status: ValidationResult,
}

impl MedicalVolume {
    /// Creates new medical volume with validation
    pub fn new(
        metadata: ImageMetadata, 
        pixel_data: PixelData, 
        source_format: ImageFormat
    ) -> MedicalImagingResult<Self>{
        let mut validator = MedicalImageValidator::new();
        let validation_status = validator.add_format_validator(source_format.clone(), &metadata, &pixel_data);
        Ok(Self {
            metadata,
            pixel_data,
            source_format,
            validation_status,
        })
    }

    /// Generates CT volume from MHA file
    pub fn generate_ct_volume_mha(
        dim: [usize; 3], 
        data: Vec<u8>,
        pixel_type: PixelType,
        spacing: Vec<f32>,
        offset: Vec<f32>,
        _transform: Vec<f32>,
        slope:f32, 
        intercept:f32
    ) -> Result<CTVolume, String> {
        let col = dim[0]; // x
        let row = dim[1]; // y
        let depth = dim[2]; // z
        let raw = data.clone();

        let voxel_count = col * row * depth;

        // analyze raw data according to ElementType
        let voxel_data = PixelData::create_pixel_data(raw,pixel_type, voxel_count, slope, intercept).map_err(|e| e.to_string())?;

        // series
        let uid = "1.2.392.200036.9116.2.5.1.144.3437232930.1426478676.365119";

        // scaling matrix
        let scaling_matrix = Mat4::from_scale(Vec3::new(
            spacing[0],
            spacing[1],
            spacing[2],
        ));

        // translation matrix
        let translation_matrix = Mat4::from_translation(Vec3::new(
            offset[0],
            offset[1],
            offset[2],
        ));

        let direction_matrix = Mat4::IDENTITY;

        // Multiply the scaling, direction, and translation matrices
        // Order: Direction * Translation * Scaling (matches original logic)
        let base_matrix = direction_matrix * translation_matrix * scaling_matrix;

        // Return the constructed CTVolume
        let ct_volume_mha = CTVolume {
            dimensions: (col, row, depth),
            voxel_spacing: (spacing[0], spacing[1], spacing[2]),
            voxel_data,
            base: Base {
                label: uid.to_string(),
                matrix: base_matrix,
            }
        };

        log::info!("{:?}", &ct_volume_mha.dimensions);
        log::info!("{:?}", &ct_volume_mha.voxel_spacing);
        log::info!("{:?}", &ct_volume_mha.base.matrix.to_cols_array_2d());
        for (index, &value) in ct_volume_mha.voxel_data.iter().enumerate() {
            if value < -1024 {
                log::info!("索引 {}: 值 {}", index, value);
            }
        }

        Ok(ct_volume_mha)
    }
    
    // pub fn convert_pixel_type<T: PixelType>(&self) -> MedicalImagingResult<MedicalVolume>;
    
    // pub fn extract_slice(&self, axis: Axis, index: usize) -> MedicalImagingResult<MedicalSlice>;
    
    // Resamples volume to new spacing
    // pub fn resample(&self, new_spacing: [f32; 3]) -> MedicalImagingResult<MedicalVolume>;
}