//! Application-level data model
//!
//! This module defines AppModel, which holds the currently loaded CT volume and
//! provides minimal APIs to manage it. It lives in the application layer to avoid
//! coupling the data module to rendering-specific types.

use crate::core::error::KeplerError;
use crate::data::ct_volume::CTVolume;
use crate::data::volume_encoding::VolumeEncoding;
use std::sync::Arc;

/// AppModel encapsulates application data state like the loaded CT volume.
#[derive(Debug)]
pub struct AppModel {
    pub(crate) vol: Option<CTVolume>,
    pub enable_float_volume_texture: bool,
}

impl AppModel {
    /// Create a new AppModel with no volume loaded.
    ///
    /// Function-level comment: Initializes the application data model to an empty state.
    pub fn new(enable_float_volume_texture: bool) -> Self {
        Self {
            vol: None,
            enable_float_volume_texture,
        }
    }

    /// Generates the byte buffer required for creating the GPU texture.
    /// Handles the logic for R16Float vs Rg8Unorm conversion internally.
    pub fn get_volume_render_data(&self) -> Result<(Vec<u8>, VolumeEncoding), KeplerError> {
        let vol = self.volume()?;

        let encoding = VolumeEncoding::for_float_support(self.enable_float_volume_texture);
        Ok((encoding.encode_volume(vol.voxel_data()), encoding))
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
