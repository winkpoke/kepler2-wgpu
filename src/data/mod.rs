//! Data structures and domain models
//! 
//! This module contains domain models and data structures.
//! Dependencies: Core utilities only, no rendering dependencies.

use std::sync::Arc;
use crate::core::error::KeplerError;

// Data modules
pub mod ct_volume;
pub mod dicom;
pub mod dicom_;
pub mod medical_imaging;

// Re-exports for convenience
pub use ct_volume::*;

/// Application model for data management
#[derive(Debug)]
pub struct AppModel {
    pub(crate) vol: Option<CTVolume>,
}

impl AppModel {
    pub fn new() -> Self {
        Self { vol: None }
    }
    
    pub fn load_volume(&mut self, volume: CTVolume) -> Result<(), KeplerError> {
        self.vol = Some(volume);
        Ok(())
    }
    
    pub fn volume(&self) -> Result<&CTVolume, KeplerError> {
        self.vol.as_ref().ok_or(KeplerError::Graphics("No volume loaded".into()))
    }
    
    pub fn has_volume(&self) -> bool {
        self.vol.is_some()
    }
    
    pub fn clear(&mut self) {
        self.vol = None;
    }
}

pub type SharedAppModel = Arc<AppModel>;
