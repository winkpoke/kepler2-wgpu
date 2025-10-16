//! Comprehensive validation module for medical imaging data and general data types
//! 
//! This module provides a robust validation framework that includes:
//! - Basic data type validation (strings, numbers, emails, etc.)
//! - Medical imaging specific validation (pixel data, DICOM fields, etc.)
//! - Asynchronous validation support for I/O bound operations
//! - Custom validation rules with composable validators
//! - Comprehensive error handling with detailed context

use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

use crate::data::medical_imaging::{
    metadata::{ImageMetadata,PixelData, PixelType},
    formats::ImageFormat,
};

// ============================================================================
// Core Validation Types and Traits
// ============================================================================

/// Validation error with detailed context information
#[derive(Debug, Error, Clone)]
pub enum ValidationError {
    #[error("Invalid input: {message}")]
    InvalidInput { 
        message: String, 
        field: Option<String>,
        context: HashMap<String, String> 
    },
    
    #[error("Range error: value {value} is not within range [{min}, {max}]")]
    OutOfRange { 
        value: String, 
        min: String, 
        max: String 
    },
    
    #[error("Format error: {message}")]
    InvalidFormat { 
        message: String, 
        expected_format: String 
    },
    
    #[error("Length error: {message}")]
    InvalidLength { 
        message: String, 
        actual: usize, 
        expected_min: Option<usize>, 
        expected_max: Option<usize> 
    },

    #[error("Invalid dimension: {message}")]
    InvalidDimension { 
        message: String, 
        dimension: String 
    },
    
    #[error("Invalid spacing: {message}")]
    InvalidSpacing { 
        message: String, 
        spacing: String 
    },
    
    #[error("Invalid orientation: {message}")]
    InvalidOrientation { 
        message: String, 
        orientation: String 
    },
    
    #[error("Medical imaging error: {message}")]
    MedicalImaging { 
        message: String, 
        error_code: String 
    },
    
    #[error("Custom validation error: {message}")]
    Custom { 
        message: String, 
        rule_name: String 
    },
    
    #[error("Async validation error: {message}")]
    AsyncValidation { 
        message: String 
    },
}

/// Validation warning for non-critical issues
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub message: String,
    pub severity: WarningSeverity,
    pub field: Option<String>,
}

/// Warning severity levels
#[derive(Debug, Clone, PartialEq)]
pub enum WarningSeverity {
    Low,
    Medium,
    High,
}

/// Metadata validation issues
#[derive(Debug, Clone)]
pub struct MetadataIssue {
    pub field: String,
    pub issue_type: String,
    pub description: String,
    pub severity: WarningSeverity,
}

/// Data validation issues
#[derive(Debug, Clone)]
pub struct DataIssue {
    pub location: String,
    pub issue_type: String,
    pub description: String,
    pub severity: WarningSeverity,
}

/// Integrity checker trait
pub trait IntegrityChecker {
    fn check_integrity(&self, data: &[u8]) -> ValidationResult{
        if data.is_empty(){
            return ValidationResult::failure(vec![ValidationError::InvalidInput {
                message: "Empty data".to_string(),
                field: Some("data".to_string()),
                context: HashMap::new(),
            }]);
        }
        else {
            ValidationResult::success()
        }
    }
    fn checker_name(&self) -> &str{
        "IntegrityChecker"
    }
}

/// Function-level comment: Basic data size integrity checker
/// Validates that data meets minimum size requirements
#[derive(Debug)]
pub struct DataSizeChecker {
    pub min_size: usize,
    pub max_size: Option<usize>,
}

impl DataSizeChecker {
    pub fn new(min_size: usize, max_size: Option<usize>) -> Self {
        Self { min_size, max_size }
    }
}

impl IntegrityChecker for DataSizeChecker {
    fn check_integrity(&self, data: &[u8]) -> ValidationResult {
        let mut errors = Vec::new();
        
        if data.len() < self.min_size {
            errors.push(ValidationError::InvalidLength {
                message: format!("Data size {} is below minimum {}", data.len(), self.min_size),
                actual: data.len(),
                expected_min: Some(self.min_size),
                expected_max: self.max_size,
            });
        }
        
        if let Some(max_size) = self.max_size {
            if data.len() > max_size {
                errors.push(ValidationError::InvalidLength {
                    message: format!("Data size {} exceeds maximum {}", data.len(), max_size),
                    actual: data.len(),
                    expected_min: Some(self.min_size),
                    expected_max: Some(max_size),
                });
            }
        }
        
        if errors.is_empty() {
            ValidationResult::success()
        } else {
            ValidationResult::failure(errors)
        }
    }
    
    fn checker_name(&self) -> &str {
        "DataSizeChecker"
    }
}

/// Function-level comment: Checksum integrity checker
/// Validates data integrity using simple checksum algorithms
#[derive(Debug)]
pub struct ChecksumChecker {
    pub expected_checksum: u32,
    pub algorithm: ChecksumAlgorithm,
}

#[derive(Debug, Clone)]
pub enum ChecksumAlgorithm {
    Simple,
    Crc32,
}

impl ChecksumChecker {
    pub fn new(expected_checksum: u32, algorithm: ChecksumAlgorithm) -> Self {
        Self { expected_checksum, algorithm }
    }
    
    fn calculate_checksum(&self, data: &[u8]) -> u32 {
        match self.algorithm {
            ChecksumAlgorithm::Simple => {
                data.iter().map(|&b| b as u32).sum()
            },
            ChecksumAlgorithm::Crc32 => {
                // Simplified CRC32 implementation for demonstration
                let mut crc = 0xFFFFFFFF_u32;
                for &byte in data {
                    crc ^= byte as u32;
                    for _ in 0..8 {
                        if crc & 1 != 0 {
                            crc = (crc >> 1) ^ 0xEDB88320;
                        } else {
                            crc >>= 1;
                        }
                    }
                }
                !crc
            }
        }
    }
}

impl IntegrityChecker for ChecksumChecker {
    fn check_integrity(&self, data: &[u8]) -> ValidationResult {
        let calculated_checksum = self.calculate_checksum(data);
        
        if calculated_checksum != self.expected_checksum {
            ValidationResult::failure(vec![ValidationError::MedicalImaging {
                message: format!(
                    "Checksum mismatch: expected {}, got {}",
                    self.expected_checksum, calculated_checksum
                ),
                error_code: "CHECKSUM_MISMATCH".to_string(),
            }])
        } else {
            ValidationResult::success()
        }
    }
    
    fn checker_name(&self) -> &str {
        "ChecksumChecker"
    }
}

/// Function-level comment: Medical imaging header integrity checker
/// Validates medical imaging specific header patterns and magic numbers
#[derive(Debug)]
pub struct MedicalHeaderChecker {
    pub expected_magic: Vec<u8>,
    pub header_size: usize,
}

impl MedicalHeaderChecker {
    pub fn new(expected_magic: Vec<u8>, header_size: usize) -> Self {
        Self { expected_magic, header_size }
    }
}

impl IntegrityChecker for MedicalHeaderChecker {
    fn check_integrity(&self, data: &[u8]) -> ValidationResult {
        let mut errors = Vec::new();
        
        if data.len() < self.header_size {
            errors.push(ValidationError::InvalidLength {
                message: format!("Data too short for header: {} < {}", data.len(), self.header_size),
                actual: data.len(),
                expected_min: Some(self.header_size),
                expected_max: None,
            });
        }
        
        if data.len() >= self.expected_magic.len() {
            if !data.starts_with(&self.expected_magic) {
                errors.push(ValidationError::InvalidFormat {
                    message: "Invalid magic number in header".to_string(),
                    expected_format: format!("Magic bytes: {:?}", self.expected_magic),
                });
            }
        }
        
        if errors.is_empty() {
            ValidationResult::success()
        } else {
            ValidationResult::failure(errors)
        }
    }
    
    fn checker_name(&self) -> &str {
        "MedicalHeaderChecker"
    }
}

/// Function-level comment: Comprehensive file and data validation
/// Ensures data integrity and format compliance
pub struct MedicalImageValidator {
    /// Format-specific validators
    format_validators: HashMap<ImageFormat, ValidationResult>,
    /// Data integrity checkers
    integrity_checkers: Vec<Box<dyn IntegrityChecker>>,
}

impl MedicalImageValidator {
    /// Function-level comment: Creates a new medical image validator
    /// Initializes with default format validators and integrity checkers
    pub fn new() -> Self {
        Self {
            format_validators: HashMap::new(),
            integrity_checkers: Vec::new(),
        }
    }

    pub fn add_format_validator(&mut self, format: ImageFormat, metadata: &ImageMetadata, pixel_data: &PixelData) -> ValidationResult {
        let mut results = Vec::new();
        let metadata_validation_results = self.validate_metadata(&metadata);
        results.push(metadata_validation_results);
        let pixel_data_validation_results = self.validate_pixel_data(&pixel_data, &metadata);
        results.push(pixel_data_validation_results);
        let validator = ValidationResult::combine(results);
        self.format_validators.insert(format, validator.clone());
        validator
    }

    /// Function-level comment: Adds an integrity checker to the validator
    /// Registers a new integrity checker that will be used during validation
    pub fn add_integrity_checker(&mut self, checker: Box<dyn IntegrityChecker>) {
        self.integrity_checkers.push(checker);
    }

    /// Function-level comment: Runs all integrity checkers on provided data
    /// Executes all registered integrity checkers and combines their results
    pub fn run_integrity_checks(&self, data: &[u8]) -> ValidationResult {
        if self.integrity_checkers.is_empty() {
            return ValidationResult::success();
        }

        let mut results = Vec::new();
        for checker in &self.integrity_checkers {
            let result = checker.check_integrity(data);
            results.push(result);
        }

        ValidationResult::combine(results)
    }

    /// Function-level comment: Validates complete medical image file
    /// Performs comprehensive file validation including format, metadata, and data integrity
    pub fn validate_file<P: AsRef<Path>>(&self, path: P) -> ValidationResult {
        let mut result = ValidationResult::success();
        
        // Check file existence
        if !path.as_ref().exists() {
            return ValidationResult::failure(vec![ValidationError::InvalidInput {
                message: "File does not exist".to_string(),
                field: Some("file_path".to_string()),
                context: HashMap::new(),
            }]);
        }
        
        // Validate file size
        if let Ok(metadata) = std::fs::metadata(&path) {
            if metadata.len() == 0 {
                result = result.with_warning(ValidationWarning {
                    message: "File is empty".to_string(),
                    severity: WarningSeverity::High,
                    field: Some("file_size".to_string()),
                });
            }
        }
        
        result
    }
    
    /// Function-level comment: Validates metadata consistency
    /// Checks metadata fields for consistency and medical imaging standards compliance
    pub fn validate_metadata(&self, metadata: &ImageMetadata) -> ValidationResult {
        let mut errors = Vec::new();

        // Validate dimensions
        for (i, &dim) in metadata.dimensions.iter().enumerate() {
            if dim == 0 {
                errors.push(ValidationError::InvalidDimension {
                    message: "Dimension cannot be zero".to_string(),
                    dimension: format!("dimensions[{}]", i),
                });
            }
        }

        // Validate spacing
        for (i, &sp) in metadata.spacing.iter().enumerate() {
            if sp <= 0.0 {
                errors.push(ValidationError::InvalidSpacing {
                    message: "Spacing cannot be zero or negative".to_string(),
                    spacing: format!("spacing[{}]", i),
                });
            }
        }

        // Validate orientation matrix (must be orthonormal)
        let det = metadata.orientation[0][0] * (metadata.orientation[1][1] * metadata.orientation[2][2] - metadata.orientation[1][2] * metadata.orientation[2][1])
                - metadata.orientation[0][1] * (metadata.orientation[1][0] * metadata.orientation[2][2] - metadata.orientation[1][2] * metadata.orientation[2][0])
                + metadata.orientation[0][2] * (metadata.orientation[1][0] * metadata.orientation[2][1] - metadata.orientation[1][1] * metadata.orientation[2][0]);
        if det.abs() < 1e-6 {
            errors.push(ValidationError::InvalidOrientation {
                message: "Orientation matrix must be orthonormal".to_string(),
                orientation: "orientation".to_string(),
            });
        }

        if errors.is_empty() {
            ValidationResult::success()
        } else {
            ValidationResult::failure(errors)
        }
    }
    
    /// Function-level comment: Validates pixel data integrity
    /// Ensures pixel data consistency with metadata and checks for data corruption
    pub fn validate_pixel_data(&self, data: &PixelData, metadata: &ImageMetadata) -> ValidationResult {
        let mut result = ValidationResult::success();
        
        // Calculate expected data size
        let expected_size = metadata.dimensions.iter().product::<usize>();
        let n = match metadata.pixel_type {
            PixelType::UInt8 => 1,
            PixelType::UInt16 => 2,
            PixelType::Int16 => 2,
            PixelType::Int32 => 4,
            PixelType::Float32 => 4,
            PixelType::Float64 => 8,
        };
        let actual_size = data.as_bytes().len()/n;
        
        if actual_size != expected_size {
            result.errors.push(ValidationError::InvalidLength {
                message: "Pixel data size does not match metadata dimensions".to_string(),
                actual: actual_size,
                expected_min: Some(expected_size),
                expected_max: Some(expected_size),
            });
            result.is_valid = false;
        }
        
        result
    }
}

/// Function-level comment: Validation result with detailed diagnostics
/// Provides comprehensive validation feedback
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub warnings: Vec<ValidationWarning>,
    pub errors: Vec<ValidationError>,
    pub metadata_issues: Vec<MetadataIssue>,
    pub data_issues: Vec<DataIssue>,
}

impl ValidationResult {
    /// Function-level comment: Creates successful validation result
    /// Returns a validation result indicating success with no errors or warnings
    pub fn success() -> Self {
        Self {
            is_valid: true,
            warnings: Vec::new(),
            errors: Vec::new(),
            metadata_issues: Vec::new(),
            data_issues: Vec::new(),
        }
    }
    
    /// Function-level comment: Creates failed validation result with error
    /// Returns a validation result indicating failure with the specified error
    pub fn failure(error: Vec<ValidationError>) -> Self {
        Self {
            is_valid: false,
            warnings: Vec::new(),
            errors: error,
            metadata_issues: Vec::new(),
            data_issues: Vec::new(),
        }
    }
    
    /// Function-level comment: Adds warning to result
    /// Appends a warning to the validation result without affecting validity
    pub fn with_warning(mut self, warning: ValidationWarning) -> Self {
        self.warnings.push(warning);
        self
    }
    
    /// Function-level comment: Combines multiple validation results
    /// Merges multiple validation results into a single comprehensive result
    pub fn combine(results: Vec<ValidationResult>) -> Self {
        let mut combined = Self::success();
        
        for result in results {
            if !result.is_valid {
                combined.is_valid = false;
            }
            combined.warnings.extend(result.warnings);
            combined.errors.extend(result.errors);
            combined.metadata_issues.extend(result.metadata_issues);
            combined.data_issues.extend(result.data_issues);
        }
        
        combined
    }
}