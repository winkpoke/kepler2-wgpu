//! Application-level data model
//!
//! This module defines AppModel, which holds the currently loaded CT volume and
//! provides minimal APIs to manage it. It lives in the application layer to avoid
//! coupling the data module to rendering-specific types.

use std::sync::Arc;
use crate::core::error::KeplerError;
use crate::data::ct_volume::CTVolume;

/// AppModel encapsulates application data state like the loaded CT volume.
#[derive(Debug)]
pub struct AppModel {
    pub(crate) vol: Option<CTVolume>,
}

impl AppModel {
    /// Create a new AppModel with no volume loaded.
    ///
    /// Function-level comment: Initializes the application data model to an empty state.
    pub fn new() -> Self {
        Self { vol: None }
    }

    /// Load a CTVolume into the model, replacing any previously loaded volume.
    ///
    /// Function-level comment: Sets the current volume; callers should ensure the volume
    /// is valid for subsequent rendering operations.
    pub fn load_volume(&mut self, volume: CTVolume) -> Result<(), KeplerError> {
        self.vol = Some(volume);
        Ok(())
    }

    /// Get a reference to the currently loaded volume.
    ///
    /// Function-level comment: Returns an error if no volume has been loaded.
    pub fn volume(&self) -> Result<&CTVolume, KeplerError> {
        self.vol
            .as_ref()
            .ok_or(KeplerError::Graphics("No volume loaded".into()))
    }

    /// Check whether a volume is loaded.
    ///
    /// Function-level comment: Convenience method for UI enable/disable logic.
    pub fn has_volume(&self) -> bool {
        self.vol.is_some()
    }

    /// Clear the currently loaded volume.
    ///
    /// Function-level comment: Resets the model to an empty state.
    pub fn clear(&mut self) {
        self.vol = None;
    }
}

/// Shared handle to AppModel for cross-component access.
pub type SharedAppModel = Arc<AppModel>;