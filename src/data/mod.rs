//! Data structures and domain models
//! 
//! This module contains domain models and data structures.
//! Dependencies: Core utilities only, no rendering dependencies.

// Data modules
pub mod ct_volume;
pub mod dicom;
pub mod dicom_;
pub mod medical_imaging;

// Re-exports for convenience
pub use ct_volume::*;