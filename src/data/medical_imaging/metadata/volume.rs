use crate::data::medical_imaging::{
    metadata::ImageMetadata,
    pixel_data::{PixelData, PixelType},
    formats::ImageFormat,
    error:: MedicalImagingResult,
    validation::ValidationResult,
};
use crate::data::CTVolume;
use crate::core::coord::{Base, Matrix4x4};

/// 标准化的 3D 医学体积表示
/// 保留所有元数据并提供对像素数据的高效访问
#[derive(Debug, Clone)]
pub struct MedicalVolume {
    /// 包括空间信息的图像元数据
    pub metadata: ImageMetadata,
    /// 具有类型安全访问的像素数据
    pub pixel_data: PixelData,
    /// 原始文件格式，用于来源追踪
    pub source_format: ImageFormat,
    /// 验证状态和完整性检查
    pub validation_status: ValidationResult,
}

impl MedicalVolume {
    /// 创建经过验证的新医学体积
    pub fn new(
        metadata: ImageMetadata, 
        pixel_data: PixelData, 
        source_format: ImageFormat
    ) -> MedicalImagingResult<Self>{
        let validation_status = metadata.validate();
        Ok(Self {
            metadata,
            pixel_data,
            source_format,
            validation_status,
        })
    }

    pub fn generate_ct_volume_mha(
        dim: [usize; 3], 
        data: Vec<u8>,
        pixel_type: PixelType,
        spacing: Vec<f32>,
        offset: Vec<f32>,
        transform: Vec<f32>,
        slope:f32, 
        intercept:f32
    ) -> Result<CTVolume, String> {
        let col = dim[0]; // x
        let row = dim[1]; // y
        let depth = dim[2]; // z
        let raw = data.clone();

        let voxel_count = col * row * depth;
        let mut voxel_data = Vec::with_capacity(voxel_count);

        // analyze raw data according to ElementType
        match pixel_type {
            PixelType::Int16 => {
                for chunk in raw.chunks_exact(2).take(voxel_count) {
                    let val = i16::from_le_bytes([chunk[0], chunk[1]]);
                    voxel_data.push(val);
                }
            }
            PixelType::Float32 => {
                for chunk in raw.chunks_exact(4).take(voxel_count) {
                    let val = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    let val = (val * slope + intercept).round() as i16;
                    voxel_data.push(val);
                }
            }
            other => return Err(format!("Unsupported PixelType: {:?}", other)),
        }

        for value in &mut voxel_data {
            if *value < -1024 {
                *value = -1024;
            }
        }

        // series
        let uid = "1.2.392.200036.9116.2.5.1.144.3437232930.1426478676.365119";

        // scaling matrix
        let scaling_matrix = Matrix4x4::from_array([
            spacing[0], 0.0, 0.0, 0.0,
            0.0, spacing[1], 0.0, 0.0,
            0.0, 0.0, spacing[2], 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]);

        // translation matrix
        let translation_matrix = Matrix4x4::from_array([
            1.0, 0.0, 0.0, offset[0],
            0.0, 1.0, 0.0, offset[1],
            0.0, 0.0, 1.0, offset[2],
            0.0, 0.0, 0.0, 1.0,
        ]);

        let direction_matrix = Matrix4x4::from_array([
            transform[0], transform[1], transform[2], 0.0,
            transform[3], transform[4], transform[5], 0.0,
            transform[6], transform[7], transform[8], 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]);

        // Multiply the scaling, direction, and translation matrices
        let base_matrix = direction_matrix
            .multiply(&translation_matrix)
            .multiply(&scaling_matrix);

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
        log::info!("{:?}", &ct_volume_mha.base.matrix.data);
        for (index, &value) in ct_volume_mha.voxel_data.iter().enumerate() {
            if value < -1024 {
                log::info!("索引 {}: 值 {}", index, value);
            }
        }

        Ok(ct_volume_mha)
    }
    
    // /// 转换为不同的像素数据类型
    // pub fn convert_pixel_type<T: PixelType>(&self) -> MedicalImagingResult<MedicalVolume>;
    
    // /// 在指定索引处提取 2D 切片
    // pub fn extract_slice(&self, axis: Axis, index: usize) -> MedicalImagingResult<MedicalSlice>;
    
    // /// 将体积重新采样到新间距
    // pub fn resample(&self, new_spacing: [f64; 3]) -> MedicalImagingResult<MedicalVolume>;
    
    // /// 验证数据完整性
    // pub fn validate(&self) -> ValidationResult;
}