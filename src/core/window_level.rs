//! Window/Level management for medical imaging display
//! 
//! This module provides a dedicated struct for handling CT window width and level
//! parameters with bias adjustment, validation, and common medical imaging presets.

use crate::core::error::{KeplerError, KeplerResult, MprError};

/// Window/Level parameters for medical imaging display with bias adjustment
/// 
/// In medical imaging, window/level controls the contrast and brightness of displayed images:
/// - Window Width: Controls the contrast range (wider = less contrast)
/// - Window Level: Controls the center brightness point
/// - Bias: Additional offset applied to the effective window level
/// 
/// The effective window level is calculated as: `window_level + bias`
#[derive(Copy, Debug, Clone, PartialEq)]
pub struct WindowLevel {
    /// Window width for contrast control (must be positive)
    window_width: f32,
    /// Window level for brightness center
    window_level: f32,
    /// Bias offset applied to window level
    bias: f32,
    /// Flag indicating if parameters have changed and need GPU update
    dirty: bool,
}

impl WindowLevel {
    /// Medical imaging parameter bounds for validation
    pub const MIN_WINDOW_WIDTH: f32 = 1.0;        // Minimum contrast range
    pub const MAX_WINDOW_WIDTH: f32 = 4096.0;     // Maximum contrast range for CT
    pub const MIN_WINDOW_LEVEL: f32 = -2048.0;    // Minimum brightness for CT
    pub const MAX_WINDOW_LEVEL: f32 = 2048.0;     // Maximum brightness for CT
    pub const MIN_BIAS: f32 = -1024.0;            // Minimum bias offset
    pub const MAX_BIAS: f32 = 1024.0;             // Maximum bias offset
    
    /// Default window/level for soft tissue
    pub const DEFAULT_SOFT_TISSUE: (f32, f32) = (400.0, 40.0);
    /// Default window/level for bone
    pub const DEFAULT_BONE: (f32, f32) = (1500.0, 400.0);
    /// Default window/level for lung
    pub const DEFAULT_LUNG: (f32, f32) = (1500.0, -600.0);
    /// Default window/level for brain
    pub const DEFAULT_BRAIN: (f32, f32) = (80.0, 40.0);
    /// Default window/level for liver
    pub const DEFAULT_LIVER: (f32, f32) = (150.0, 30.0);

    /// Create a new WindowLevel with default soft tissue settings
    pub fn new() -> Self {
        Self {
            window_width: Self::DEFAULT_SOFT_TISSUE.0,
            window_level: Self::DEFAULT_SOFT_TISSUE.1,
            bias: 0.0,
            dirty: true, // Initial state requires GPU update
        }
    }

    /// Create a new WindowLevel with specified parameters
    /// 
    /// # Arguments
    /// * `window_width` - Window width for contrast control
    /// * `window_level` - Window level for brightness center
    /// * `bias` - Bias offset applied to window level
    /// 
    /// # Returns
    /// * `KeplerResult<WindowLevel>` - New WindowLevel instance or validation error
    pub fn with_params(window_width: f32, window_level: f32, bias: f32) -> KeplerResult<Self> {
        let mut wl = Self::new();
        wl.set_window_width(window_width)?;
        wl.set_window_level(window_level)?;
        wl.set_bias(bias)?;
        Ok(wl)
    }

    /// Set window width with validation
    /// 
    /// # Arguments
    /// * `width` - New window width (must be positive and within bounds)
    /// 
    /// # Returns
    /// * `KeplerResult<()>` - Success or validation error
    pub fn set_window_width(&mut self, width: f32) -> KeplerResult<()> {
        if !width.is_finite() || width <= 0.0 {
            return Err(KeplerError::Mpr(MprError::InvalidWindowWidth(width)));
        }

        let clamped_width = width.clamp(Self::MIN_WINDOW_WIDTH, Self::MAX_WINDOW_WIDTH);
        if (clamped_width - width).abs() > f32::EPSILON {
            log::warn!("Window width {} clamped to {}", width, clamped_width);
        }

        if (self.window_width - clamped_width).abs() > f32::EPSILON {
            self.window_width = clamped_width;
            self.mark_dirty();
            log::debug!("Window width updated to: {}", self.window_width);
        }

        Ok(())
    }

    /// Set window level with validation
    /// 
    /// # Arguments
    /// * `level` - New window level (within medical imaging bounds)
    /// 
    /// # Returns
    /// * `KeplerResult<()>` - Success or validation error
    pub fn set_window_level(&mut self, level: f32) -> KeplerResult<()> {
        if !level.is_finite() {
            return Err(KeplerError::Mpr(MprError::InvalidWindowLevel(level)));
        }

        let clamped_level = level.clamp(Self::MIN_WINDOW_LEVEL, Self::MAX_WINDOW_LEVEL);
        if (clamped_level - level).abs() > f32::EPSILON {
            log::warn!("Window level {} clamped to {}", level, clamped_level);
        }

        if (self.window_level - clamped_level).abs() > f32::EPSILON {
            self.window_level = clamped_level;
            self.mark_dirty();
            log::debug!("Window level updated to: {}", self.window_level);
        }

        Ok(())
    }

    /// Set bias offset with validation
    /// 
    /// # Arguments
    /// * `bias` - New bias offset (within reasonable bounds)
    /// 
    /// # Returns
    /// * `KeplerResult<()>` - Success or validation error
    pub fn set_bias(&mut self, bias: f32) -> KeplerResult<()> {
        if !bias.is_finite() {
            return Err(KeplerError::Mpr(MprError::InvalidBias(bias)));
        }

        let clamped_bias = bias.clamp(Self::MIN_BIAS, Self::MAX_BIAS);
        if (clamped_bias - bias).abs() > f32::EPSILON {
            log::warn!("Bias {} clamped to {}", bias, clamped_bias);
        }

        if (self.bias - clamped_bias).abs() > f32::EPSILON {
            self.bias = clamped_bias;
            self.mark_dirty();
            log::debug!("Bias updated to: {}", self.bias);
        }

        Ok(())
    }

    /// Apply a medical imaging preset
    /// 
    /// # Arguments
    /// * `preset` - Tuple of (window_width, window_level) for the preset
    /// 
    /// # Returns
    /// * `KeplerResult<()>` - Success or validation error
    pub fn apply_preset(&mut self, preset: (f32, f32)) -> KeplerResult<()> {
        self.set_window_width(preset.0)?;
        self.set_window_level(preset.1)?;
        log::info!("Applied preset: width={}, level={}", preset.0, preset.1);
        Ok(())
    }

    /// Apply soft tissue preset
    pub fn apply_soft_tissue_preset(&mut self) -> KeplerResult<()> {
        self.apply_preset(Self::DEFAULT_SOFT_TISSUE)
    }

    /// Apply bone preset
    pub fn apply_bone_preset(&mut self) -> KeplerResult<()> {
        self.apply_preset(Self::DEFAULT_BONE)
    }

    /// Apply lung preset
    pub fn apply_lung_preset(&mut self) -> KeplerResult<()> {
        self.apply_preset(Self::DEFAULT_LUNG)
    }

    /// Apply brain preset
    pub fn apply_brain_preset(&mut self) -> KeplerResult<()> {
        self.apply_preset(Self::DEFAULT_BRAIN)
    }

    /// Apply liver preset
    pub fn apply_liver_preset(&mut self) -> KeplerResult<()> {
        self.apply_preset(Self::DEFAULT_LIVER)
    }

    /// Get the current window width
    pub fn window_width(&self) -> f32 {
        self.window_width
    }

    /// Get the current window level
    pub fn window_level(&self) -> f32 {
        self.window_level
    }

    /// Get the current bias
    pub fn bias(&self) -> f32 {
        self.bias
    }

    /// Calculate the effective window level (level + bias)
    /// 
    /// This is the actual level value used for display calculations
    pub fn effective_level(&self) -> f32 {
        self.window_level + self.bias
    }

    /// Get normalized window/level values for shader use
    /// 
    /// Returns (normalized_width, normalized_level) where:
    /// - normalized_width = 1.0 / window_width (for shader efficiency)
    /// - normalized_level = effective_level / window_width (centered normalization)
    /// 
    /// # Returns
    /// * `(f32, f32)` - Tuple of (normalized_width, normalized_level)
    pub fn normalized_values(&self) -> (f32, f32) {
        let inv_width = 1.0 / self.window_width;
        let normalized_level = self.effective_level() * inv_width;
        (inv_width, normalized_level)
    }

    /// Get shader-ready uniform values
    /// 
    /// Returns values optimized for GPU shader use:
    /// - window_scale = 1.0 / window_width
    /// - window_offset = -effective_level / window_width + 0.5
    /// 
    /// These values allow efficient pixel intensity mapping in shaders:
    /// `display_value = clamp(pixel_value * window_scale + window_offset, 0.0, 1.0)`
    /// 
    /// # Returns
    /// * `(f32, f32)` - Tuple of (window_scale, window_offset)
    pub fn shader_uniforms(&self) -> (f32, f32) {
        let window_scale = 1.0 / self.window_width;
        let window_offset = -self.effective_level() * window_scale + 0.5;
        (window_scale, window_offset)
    }

    /// Check if parameters have changed and need GPU update
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark parameters as clean (called after GPU update)
    pub fn mark_clean(&mut self) {
        if self.dirty {
            self.dirty = false;
            log::trace!("WindowLevel marked as clean");
        }
    }

    /// Mark parameters as dirty (internal use)
    fn mark_dirty(&mut self) {
        if !self.dirty {
            self.dirty = true;
            log::trace!("WindowLevel marked as dirty");
        }
    }

    /// Validate current parameters
    /// 
    /// # Returns
    /// * `KeplerResult<()>` - Success if all parameters are valid
    pub fn validate(&self) -> KeplerResult<()> {
        if !self.window_width.is_finite() || self.window_width <= 0.0 {
            return Err(KeplerError::Mpr(MprError::InvalidWindowWidth(self.window_width)));
        }

        if !self.window_level.is_finite() {
            return Err(KeplerError::Mpr(MprError::InvalidWindowLevel(self.window_level)));
        }

        if !self.bias.is_finite() {
            return Err(KeplerError::Mpr(MprError::InvalidBias(self.bias)));
        }

        // Check bounds
        if self.window_width < Self::MIN_WINDOW_WIDTH || self.window_width > Self::MAX_WINDOW_WIDTH {
            return Err(KeplerError::Mpr(MprError::InvalidWindowWidth(self.window_width)));
        }

        if self.window_level < Self::MIN_WINDOW_LEVEL || self.window_level > Self::MAX_WINDOW_LEVEL {
            return Err(KeplerError::Mpr(MprError::InvalidWindowLevel(self.window_level)));
        }

        if self.bias < Self::MIN_BIAS || self.bias > Self::MAX_BIAS {
            return Err(KeplerError::Mpr(MprError::InvalidBias(self.bias)));
        }

        Ok(())
    }
}

impl Default for WindowLevel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_window_level() {
        let wl = WindowLevel::new();
        assert_eq!(wl.window_width(), WindowLevel::DEFAULT_SOFT_TISSUE.0);
        assert_eq!(wl.window_level(), WindowLevel::DEFAULT_SOFT_TISSUE.1);
        assert_eq!(wl.bias(), 0.0);
        assert!(wl.is_dirty());
    }

    #[test]
    fn test_with_params() {
        let wl = WindowLevel::with_params(100.0, 50.0, 10.0).unwrap();
        assert_eq!(wl.window_width(), 100.0);
        assert_eq!(wl.window_level(), 50.0);
        assert_eq!(wl.bias(), 10.0);
        assert_eq!(wl.effective_level(), 60.0);
    }

    #[test]
    fn test_validation() {
        let mut wl = WindowLevel::new();
        
        // Test invalid window width
        assert!(wl.set_window_width(-1.0).is_err());
        assert!(wl.set_window_width(0.0).is_err());
        assert!(wl.set_window_width(f32::NAN).is_err());
        
        // Test valid window width
        assert!(wl.set_window_width(100.0).is_ok());
        assert_eq!(wl.window_width(), 100.0);
    }

    #[test]
    fn test_clamping() {
        let mut wl = WindowLevel::new();
        
        // Test window width clamping
        wl.set_window_width(10000.0).unwrap();
        assert_eq!(wl.window_width(), WindowLevel::MAX_WINDOW_WIDTH);
        
        // Test window level clamping
        wl.set_window_level(-5000.0).unwrap();
        assert_eq!(wl.window_level(), WindowLevel::MIN_WINDOW_LEVEL);
    }

    #[test]
    fn test_presets() {
        let mut wl = WindowLevel::new();
        
        wl.apply_bone_preset().unwrap();
        assert_eq!(wl.window_width(), WindowLevel::DEFAULT_BONE.0);
        assert_eq!(wl.window_level(), WindowLevel::DEFAULT_BONE.1);
        
        wl.apply_lung_preset().unwrap();
        assert_eq!(wl.window_width(), WindowLevel::DEFAULT_LUNG.0);
        assert_eq!(wl.window_level(), WindowLevel::DEFAULT_LUNG.1);
    }

    #[test]
    fn test_effective_level() {
        let mut wl = WindowLevel::new();
        wl.set_window_level(100.0).unwrap();
        wl.set_bias(25.0).unwrap();
        assert_eq!(wl.effective_level(), 125.0);
    }

    #[test]
    fn test_shader_uniforms() {
        let mut wl = WindowLevel::new();
        wl.set_window_width(100.0).unwrap();
        wl.set_window_level(50.0).unwrap();
        wl.set_bias(0.0).unwrap();
        
        let (scale, offset) = wl.shader_uniforms();
        assert_eq!(scale, 0.01); // 1.0 / 100.0
        assert_eq!(offset, 0.0); // -50.0 * 0.01 + 0.5 = 0.0
    }

    #[test]
    fn test_dirty_state() {
        let mut wl = WindowLevel::new();
        assert!(wl.is_dirty());
        
        wl.mark_clean();
        assert!(!wl.is_dirty());
        
        wl.set_window_width(200.0).unwrap();
        assert!(wl.is_dirty());
    }
}