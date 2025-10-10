/// Medical imaging module for handling various medical image formats
/// Provides a comprehensive system for parsing, processing, and validating medical imaging files
/// including MHA, MHD, and other formats with robust error handling and cross-platform support

pub mod error;
pub mod formats;
pub mod metadata;
pub mod validation;

// Re-export main types for convenience
pub use error::*;
pub use formats::*;
pub use metadata::*;
pub use validation::*;