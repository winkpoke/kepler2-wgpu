use std::path::Path;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use  crate::data::medical_imaging::metadata::{
    ImageMetadata, 
    PixelData, 
    PixelType,
    Endianness,
};
use crate::data::medical_imaging::data::compression::CompressionType;
use crate::data::medical_imaging::formats::{ImageFormat, FormatValidator, IntegrityChecker};

#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub code: String,
    pub message: String,
    pub severity: WarningSeverity,
    pub context: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
    pub context: HashMap<String, String>,
}

/// 主医学影像验证器
/// 协调医学影像文件和数据的全面验证
pub struct MedicalImageValidator {
    /// 格式特定验证器
    format_validators: HashMap<ImageFormat, Box<dyn FormatValidator>>,
    /// 数据完整性检查器
    integrity_checkers: Vec<Box<dyn IntegrityChecker>>,
}

impl MedicalImageValidator {
    /// 验证完整的医学影像文件
    pub fn validate_file<P: AsRef<Path>>(path: P) -> ValidationResult{
        let path = path.as_ref();
        let mut result = Vec::new();

        // 1. 基础文件检查
        results.push(self.validate_file_existence(path));
        
        // 2. 文件大小检查
        results.push(self.validate_file_size(path));
        
        // 3. 格式检测和验证
        if let Some(format) = self.detect_format(path) {
            if let Some(validator) = self.format_validators.get(&format) {
                results.push(validator.validate_format(path));
            } else {
                results.push(ValidationResult::failure(ValidationError {
                    code: "UNSUPPORTED_FORMAT".to_string(),
                    message: format!("No validator registered for format: {:?}", format),
                    context: HashMap::new(),
                }));
            }
        } else {
            results.push(ValidationResult::failure(ValidationError {
                code: "UNKNOWN_FORMAT".to_string(),
                message: "Unable to detect file format".to_string(),
                context: HashMap::new(),
            }));
        }
        
        // 4. 校验和验证
        results.push(self.validate_checksum(path));
        
        // 合并所有验证结果
        ValidationResult::combine(results)
    }
    
    /// 验证元数据一致性
    pub fn validate_metadata(metadata: &ImageMetadata) -> ValidationResult;
    
    /// 验证像素数据完整性
    pub fn validate_pixel_data(data: &PixelData, metadata: &ImageMetadata) -> ValidationResult;
    
    /// 执行校验和验证（如果可用）
    pub fn validate_checksum<P: AsRef<Path>>(path: P) -> ValidationResult;

    fn validate_file_existence(&self, path: &Path) -> ValidationResult {
        if !path.exists() {
            return ValidationResult::failure(ValidationError {
                code: "FILE_NOT_FOUND".to_string(),
                message: "File does not exist".to_string(),
                context: HashMap::from([
                    ("path".to_string(), path.display().to_string()),
                ]),
            });
        }
        
        if !path.is_file() {
            return ValidationResult::failure(ValidationError {
                code: "NOT_A_FILE".to_string(),
                message: "Path is not a file".to_string(),
                context: HashMap::from([
                    ("path".to_string(), path.display().to_string()),
                ]),
            });
        }
        
        ValidationResult::success()
    }
    
    fn validate_file_size(&self, path: &Path) -> ValidationResult {
        match std::fs::metadata(path) {
            Ok(metadata) => {
                let file_size = metadata.len();
                
                if file_size == 0 {
                    return ValidationResult::failure(ValidationError {
                        code: "EMPTY_FILE".to_string(),
                        message: "File is empty".to_string(),
                        context: HashMap::from([
                            ("path".to_string(), path.display().to_string()),
                        ]),
                    });
                }
                
                // 检查文件大小是否合理（最大1GB）
                const MAX_FILE_SIZE: u64 = 1024 * 1024 * 1024; // 1GB
                if file_size > MAX_FILE_SIZE {
                    return ValidationResult::failure(ValidationError {
                        code: "FILE_TOO_LARGE".to_string(),
                        message: format!("File size exceeds maximum allowed size ({} bytes)", MAX_FILE_SIZE),
                        context: HashMap::from([
                            ("file_size".to_string(), file_size.to_string()),
                            ("max_size".to_string(), MAX_FILE_SIZE.to_string()),
                        ]),
                    });
                }
                
                ValidationResult::success()
            }
            Err(e) => ValidationResult::failure(ValidationError {
                code: "METADATA_ACCESS_FAILED".to_string(),
                message: format!("Failed to access file metadata: {}", e),
                context: HashMap::from([
                    ("path".to_string(), path.display().to_string()),
                ]),
            }),
        }
    }

    fn detect_format(&self, path: &Path) -> Option<ImageFormat> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext.to_lowercase().as_str() {
                "mha" => Some(ImageFormat::MHA),
                "mhd" => Some(ImageFormat::MHD),
                "nii" | "nii.gz" => Some(ImageFormat::NIfTI),
                "dcm" => Some(ImageFormat::DICOM),
                _ => None,
            })
    }


}

/// 功能级注释：具有详细诊断的验证结果
/// 提供全面的验证反馈
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub warnings: Vec<ValidationWarning>,
    pub errors: Vec<ValidationError>,
    pub metadata_issues: Vec<MetadataIssue>,
    pub data_issues: Vec<DataIssue>,
}

impl ValidationResult {
    /// 创建成功的验证结果
    pub fn success() -> Self;
    
    /// 创建带有错误的失败验证结果
    pub fn failure(error: ValidationError) -> Self;
    
    /// 向结果添加警告
    pub fn with_warning(mut self, warning: ValidationWarning) -> Self;
    
    /// 合并多个验证结果
    pub fn combine(results: Vec<ValidationResult>) -> Self;
}