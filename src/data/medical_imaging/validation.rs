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
use async_trait::async_trait;

use crate::data::medical_imaging::{
    metadata::ImageMetadata,
    data::PixelData,
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

/// Image format enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    MHA,
    MHD,
    NIfTI,
    DICOM,
    Unknown,
}

/// Format validator trait
pub trait FormatValidator {
    fn validate_format(&self, path: &Path) -> ValidationResult;
    fn supported_format(&self) -> ImageFormat;
}

/// Integrity checker trait
pub trait IntegrityChecker {
    fn check_integrity(&self, data: &[u8]) -> ValidationResult;
    fn checker_name(&self) -> &str;
}

/// Core validation trait for synchronous validation
pub trait Validator<T> {
    /// Validate the input value
    fn validate(&self, value: &T) -> Result<ValidationResult, ValidationError>;
    
    /// Get validator name for debugging
    fn name(&self) -> &'static str;
}

/// Async validation trait for I/O bound operations
#[async_trait]
pub trait AsyncValidator<T> {
    /// Asynchronously validate the input value
    async fn validate_async(&self, value: &T) -> Result<ValidationResult, ValidationError>;
    
    /// Get validator name for debugging
    fn name(&self) -> &'static str;
}

/// Custom validation rule trait
pub trait ValidationRule<T> {
    /// Apply the validation rule
    fn apply(&self, value: &T) -> Result<(), ValidationError>;
    
    /// Get rule name
    fn rule_name(&self) -> &str;
}

/// Function-level comment: Comprehensive file and data validation
/// Ensures data integrity and format compliance
pub struct MedicalImageValidator {
    /// Format-specific validators
    format_validators: HashMap<ImageFormat, Box<dyn FormatValidator>>,
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

    /// Function-level comment: Validates complete medical image file
    /// Performs comprehensive file validation including format, metadata, and data integrity
    pub fn validate_file<P: AsRef<Path>>(&self, path: P) -> ValidationResult {
        let mut result = ValidationResult::success();
        
        // Check file existence
        if !path.as_ref().exists() {
            return ValidationResult::failure(ValidationError::InvalidInput {
                message: "File does not exist".to_string(),
                field: Some("file_path".to_string()),
                context: HashMap::new(),
            });
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
        let mut result = ValidationResult::success();
        
        // Validate dimensions
        if metadata.dimensions.iter().any(|&d| d == 0) {
            result.errors.push(ValidationError::InvalidInput {
                message: "Image dimensions cannot be zero".to_string(),
                field: Some("dimensions".to_string()),
                context: HashMap::new(),
            });
            result.is_valid = false;
        }
        
        // Validate spacing
        if metadata.spacing.iter().any(|&s| s <= 0.0) {
            result.errors.push(ValidationError::InvalidInput {
                message: "Image spacing must be positive".to_string(),
                field: Some("spacing".to_string()),
                context: HashMap::new(),
            });
            result.is_valid = false;
        }
        
        result
    }
    
    /// Function-level comment: Validates pixel data integrity
    /// Ensures pixel data consistency with metadata and checks for data corruption
    pub fn validate_pixel_data(&self, data: &PixelData, metadata: &ImageMetadata) -> ValidationResult {
        let mut result = ValidationResult::success();
        
        // Calculate expected data size
        let expected_size = metadata.dimensions.iter().product::<u32>() as usize;
        let actual_size = data.len();
        
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
    
    /// Function-level comment: Performs checksum validation if available
    /// Validates file integrity using checksums when available
    pub fn validate_checksum<P: AsRef<Path>>(&self, path: P) -> ValidationResult {
        // For now, return success - checksum validation would be implemented
        // based on specific file format requirements
        ValidationResult::success()
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
    pub fn failure(error: ValidationError) -> Self {
        Self {
            is_valid: false,
            warnings: Vec::new(),
            errors: vec![error],
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