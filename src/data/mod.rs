//! Data structures and domain models
//!
//! This module contains domain models and data structures.
//! Dependencies: Core utilities only, no rendering dependencies.

// Data modules
pub mod ct_volume;
pub mod dicom;
pub mod dicom_;
pub mod medical_imaging;
pub mod volume_encoding;

// Re-exports for convenience
pub use ct_volume::*;
// Transitional re-export to preserve existing imports while AppModel lives in application layer
pub use crate::application::app_model::AppModel;

// AppModel moved to src/application/app_model.rs
