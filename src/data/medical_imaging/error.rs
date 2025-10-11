/// Comprehensive error types for medical imaging operations
/// Provides detailed error information for debugging and user feedback with proper error context

use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

/// Main error type for all medical imaging operations
/// Covers all possible error conditions with detailed context information
#[derive(Debug, Error)]
pub enum MedicalImagingError {
    /// Invalid path format or encoding
    #[error("Invalid path format: {path}")]
    InvalidPath { path: String },

    /// File format not supported
    #[error("Unsupported file format: {format}")]
    UnsupportedFormat { format: String },
    
    /// Invalid file header
    #[error("Invalid file header: {reason}")]
    InvalidHeader { reason: String },
    
    /// Metadata validation failed
    #[error("Metadata validation failed: {field} - {reason}")]
    MetadataValidation { field: String, reason: String },
    
    /// Pixel data corruption detected
    #[error("Pixel data corruption: expected {expected} bytes, found {actual}")]
    DataCorruption { expected: usize, actual: usize },
    
    /// Compression/decompression error
    #[error("Compression error: {algorithm} - {reason}")]
    CompressionError { algorithm: String, reason: String },
    
    /// Endianness conversion error
    #[error("Endianness conversion failed: {reason}")]
    EndiannessError { reason: String },
    
    /// File I/O error
    #[error("File I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Memory allocation error
    #[error("Memory allocation failed: requested {size} bytes")]
    MemoryAllocation { size: usize },
    
    /// Data type conversion error
    #[error("Data type conversion failed: from {from} to {to}")]
    TypeConversion { from: String, to: String },
    
    /// Validation error
    #[error("Validation failed: {0}")]
    Validation(String),
    
    /// Parse error for numeric values
    #[error("Parse error: {field} - {reason}")]
    ParseError { field: String, reason: String },
    
    /// Missing required field
    #[error("Missing required field: {field}")]
    MissingField { field: String },
    
    /// Invalid dimensions
    #[error("Invalid dimensions: {reason}")]
    InvalidDimensions { reason: String },
    
    /// Unsupported pixel type
    #[error("Unsupported pixel type: {pixel_type}")]
    UnsupportedPixelType { pixel_type: String },
}

/// Result type alias for medical imaging operations
pub type MedicalImagingResult<T> = Result<T, MedicalImagingError>;

/// Error context for detailed diagnostics
/// Provides additional context for error analysis and debugging
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub file_path: Option<PathBuf>,
    pub operation: String,
    pub line_number: Option<usize>,
    pub additional_info: HashMap<String, String>,
}

impl ErrorContext {
    /// Creates new error context
    pub fn new(operation: &str) -> Self {
        Self {
            file_path: None,
            operation: operation.to_string(),
            line_number: None,
            additional_info: HashMap::new(),
        }
    }
    
    /// Adds file path context
    pub fn with_file<P: AsRef<std::path::Path>>(mut self, path: P) -> Self {
        self.file_path = Some(path.as_ref().to_path_buf());
        self
    }
    
    /// Adds line number context
    pub fn with_line(mut self, line: usize) -> Self {
        self.line_number = Some(line);
        self
    }
    
    /// Adds additional context information
    pub fn with_info(mut self, key: &str, value: &str) -> Self {
        self.additional_info.insert(key.to_string(), value.to_string());
        self
    }
}