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
    pub enable_mesh: bool,
}

impl AppModel {
    /// Create a new AppModel with no volume loaded.
    ///
    /// Function-level comment: Initializes the application data model to an empty state.
    pub fn new(enable_float_volume_texture: bool) -> Self {
        Self {
            vol: None,
            enable_float_volume_texture,
            enable_mesh: false,
        }
    }

    /// Generates the byte buffer required for creating the GPU texture.
    /// Handles the logic for R16Float vs Rg8Unorm conversion internally.
    pub fn get_volume_render_data(&self) -> Result<(Vec<u8>, VolumeEncoding), KeplerError> {
        let vol = self.volume()?;

        if self.enable_float_volume_texture {
            // Convert voxel i16 values to half-float bytes
            let bytes: Vec<u8> = {
                let voxels_f16_bits: Vec<u16> = vol
                    .voxel_data
                    .iter()
                    .map(|&x| half::f16::from_f32(x as f32).to_bits())
                    .collect();
                bytemuck::cast_slice(&voxels_f16_bits).to_vec()
            };
            Ok((bytes, VolumeEncoding::HuFloat))
        } else {
            let offset = VolumeEncoding::DEFAULT_HU_OFFSET;
            let voxel_data: Vec<u16> = vol
                .voxel_data
                .iter()
                .map(|x| (*x + offset as i16) as u16)
                .collect();
            let bytes: Vec<u8> = bytemuck::cast_slice(&voxel_data).to_vec();
            Ok((bytes, VolumeEncoding::HuPackedRg8 { offset }))
        }
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
