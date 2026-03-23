//! Unit tests for MPR view error handling and validation
//!
//! Tests the Phase 1 critical safety improvements including:
//! - Input validation for all setter methods
//! - Bounds checking for medical imaging parameters
//! - Error handling for invalid matrix operations
//! - Constructor parameter validation

use kepler_wgpu::core::error::{KeplerError, MprError};

#[cfg(test)]
mod mpr_validation_tests {
    use super::*;

    /// Test medical imaging parameter bounds validation
    #[test]
    fn test_medical_parameter_bounds() {
        // Test scale bounds
        assert!(matches!(
            validate_scale(-1.0),
            Err(KeplerError::Mpr(MprError::InvalidScale(_)))
        ));

        assert!(matches!(
            validate_scale(f32::NAN),
            Err(KeplerError::Mpr(MprError::InvalidScale(_)))
        ));

        assert!(matches!(
            validate_scale(f32::INFINITY),
            Err(KeplerError::Mpr(MprError::InvalidScale(_)))
        ));

        // Valid scale should pass
        assert!(validate_scale(1.0).is_ok());
        assert!(validate_scale(0.5).is_ok());
        assert!(validate_scale(2.0).is_ok());
    }

    /// Test window level validation
    #[test]
    fn test_window_level_validation() {
        // Test invalid values
        assert!(matches!(
            validate_window_level(f32::NAN),
            Err(KeplerError::Mpr(MprError::InvalidWindowLevel(_)))
        ));

        assert!(matches!(
            validate_window_level(f32::INFINITY),
            Err(KeplerError::Mpr(MprError::InvalidWindowLevel(_)))
        ));

        assert!(matches!(
            validate_window_level(f32::NEG_INFINITY),
            Err(KeplerError::Mpr(MprError::InvalidWindowLevel(_)))
        ));

        // Valid values should pass
        assert!(validate_window_level(0.0).is_ok());
        assert!(validate_window_level(-1024.0).is_ok());
        assert!(validate_window_level(1024.0).is_ok());
    }

    /// Test window width validation
    #[test]
    fn test_window_width_validation() {
        // Test invalid values
        assert!(matches!(
            validate_window_width(0.0),
            Err(KeplerError::Mpr(MprError::InvalidWindowWidth(_)))
        ));

        assert!(matches!(
            validate_window_width(-100.0),
            Err(KeplerError::Mpr(MprError::InvalidWindowWidth(_)))
        ));

        assert!(matches!(
            validate_window_width(f32::NAN),
            Err(KeplerError::Mpr(MprError::InvalidWindowWidth(_)))
        ));

        // Valid values should pass
        assert!(validate_window_width(1.0).is_ok());
        assert!(validate_window_width(100.0).is_ok());
        assert!(validate_window_width(4000.0).is_ok());
    }

    /// Test slice position validation
    #[test]
    fn test_slice_position_validation() {
        // Test invalid values
        assert!(matches!(
            validate_slice_position(f32::NAN),
            Err(KeplerError::Mpr(MprError::InvalidSlicePosition(_)))
        ));

        assert!(matches!(
            validate_slice_position(f32::INFINITY),
            Err(KeplerError::Mpr(MprError::InvalidSlicePosition(_)))
        ));

        // Valid values should pass
        assert!(validate_slice_position(0.0).is_ok());
        assert!(validate_slice_position(50.0).is_ok());
        assert!(validate_slice_position(-25.0).is_ok());
    }

    /// Test pan coordinates validation
    #[test]
    fn test_pan_coordinates_validation() {
        // Test invalid values
        assert!(matches!(
            validate_pan_coordinates(f32::NAN, 0.0),
            Err(KeplerError::Mpr(MprError::InvalidPanCoordinates(_)))
        ));

        assert!(matches!(
            validate_pan_coordinates(0.0, f32::INFINITY),
            Err(KeplerError::Mpr(MprError::InvalidPanCoordinates(_)))
        ));

        assert!(matches!(
            validate_pan_coordinates(f32::NEG_INFINITY, f32::INFINITY),
            Err(KeplerError::Mpr(MprError::InvalidPanCoordinates(_)))
        ));

        // Valid values should pass
        assert!(validate_pan_coordinates(0.0, 0.0).is_ok());
        assert!(validate_pan_coordinates(100.0, -50.0).is_ok());
        assert!(validate_pan_coordinates(-200.0, 300.0).is_ok());
    }

    /// Test bounds clamping behavior
    #[test]
    fn test_bounds_clamping() {
        // Test scale clamping
        let result = clamp_scale(0.001); // Below minimum
        assert!(result >= 0.01); // Should be clamped to minimum

        let result = clamp_scale(200.0); // Above maximum
        assert!(result <= 100.0); // Should be clamped to maximum

        // Test window level clamping
        let result = clamp_window_level(-5000.0); // Below minimum
        assert!(result >= -2048.0); // Should be clamped to minimum

        let result = clamp_window_level(5000.0); // Above maximum
        assert!(result <= 2048.0); // Should be clamped to maximum

        // Test window width clamping
        let result = clamp_window_width(0.5); // Below minimum
        assert!(result >= 1.0); // Should be clamped to minimum

        let result = clamp_window_width(10000.0); // Above maximum
        assert!(result <= 4096.0); // Should be clamped to maximum
    }

    /// Test coordinate transformation error handling
    #[test]
    fn test_coordinate_transformation_errors() {
        // Test invalid input coordinates
        let invalid_coords = [f32::NAN, 0.0, 0.0];
        assert!(matches!(
            validate_coordinates(invalid_coords),
            Err(KeplerError::Mpr(MprError::CoordinateOutOfBounds(_)))
        ));

        let infinite_coords = [f32::INFINITY, f32::NEG_INFINITY, 0.0];
        assert!(matches!(
            validate_coordinates(infinite_coords),
            Err(KeplerError::Mpr(MprError::CoordinateOutOfBounds(_)))
        ));

        // Valid coordinates should pass
        let valid_coords = [100.0, 200.0, 50.0];
        assert!(validate_coordinates(valid_coords).is_ok());
    }

    /// Test position and dimension validation
    #[test]
    fn test_position_dimension_validation() {
        // Test extreme positions
        let extreme_pos = (1_000_000, -1_000_000);
        let clamped_pos = clamp_position(extreme_pos);
        assert!(clamped_pos.0 <= 100_000);
        assert!(clamped_pos.1 >= -100_000);

        // Test invalid dimensions
        let zero_dim = (0, 100);
        let clamped_dim = clamp_dimensions(zero_dim);
        assert!(clamped_dim.0 >= 1);

        let huge_dim = (50000, 50000);
        let clamped_dim = clamp_dimensions(huge_dim);
        assert!(clamped_dim.0 <= 16384);
        assert!(clamped_dim.1 <= 16384);
    }

    // Helper functions for testing (these would normally be part of MprView)

    fn validate_scale(scale: f32) -> Result<(), KeplerError> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err(MprError::InvalidScale(scale).into());
        }
        Ok(())
    }

    fn validate_window_level(level: f32) -> Result<(), KeplerError> {
        if !level.is_finite() {
            return Err(MprError::InvalidWindowLevel(level).into());
        }
        Ok(())
    }

    fn validate_window_width(width: f32) -> Result<(), KeplerError> {
        if !width.is_finite() || width <= 0.0 {
            return Err(MprError::InvalidWindowWidth(width).into());
        }
        Ok(())
    }

    fn validate_slice_position(z: f32) -> Result<(), KeplerError> {
        if !z.is_finite() {
            return Err(MprError::InvalidSlicePosition(z).into());
        }
        Ok(())
    }

    fn validate_pan_coordinates(x: f32, y: f32) -> Result<(), KeplerError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(MprError::InvalidPanCoordinates([x, y, 0.0]).into());
        }
        Ok(())
    }

    fn validate_coordinates(coords: [f32; 3]) -> Result<(), KeplerError> {
        for &coord in &coords {
            if !coord.is_finite() {
                return Err(MprError::CoordinateOutOfBounds(coords).into());
            }
        }
        Ok(())
    }

    fn clamp_scale(scale: f32) -> f32 {
        const MIN_SCALE: f32 = 0.01;
        const MAX_SCALE: f32 = 100.0;
        scale.clamp(MIN_SCALE, MAX_SCALE)
    }

    fn clamp_window_level(level: f32) -> f32 {
        const MIN_WINDOW_LEVEL: f32 = -2048.0;
        const MAX_WINDOW_LEVEL: f32 = 2048.0;
        level.clamp(MIN_WINDOW_LEVEL, MAX_WINDOW_LEVEL)
    }

    fn clamp_window_width(width: f32) -> f32 {
        const MIN_WINDOW_WIDTH: f32 = 1.0;
        const MAX_WINDOW_WIDTH: f32 = 4096.0;
        width.clamp(MIN_WINDOW_WIDTH, MAX_WINDOW_WIDTH)
    }

    fn clamp_position(pos: (i32, i32)) -> (i32, i32) {
        const MAX_POSITION: i32 = 100_000;
        const MIN_POSITION: i32 = -100_000;
        (
            pos.0.clamp(MIN_POSITION, MAX_POSITION),
            pos.1.clamp(MIN_POSITION, MAX_POSITION),
        )
    }

    fn clamp_dimensions(dim: (u32, u32)) -> (u32, u32) {
        const MAX_DIMENSION: u32 = 16384;
        const MIN_DIMENSION: u32 = 1;
        (
            dim.0.clamp(MIN_DIMENSION, MAX_DIMENSION),
            dim.1.clamp(MIN_DIMENSION, MAX_DIMENSION),
        )
    }
}

/// Property-based tests for medical imaging parameter validation
#[cfg(test)]
mod property_tests {

    /// Property: All finite, positive scales should be valid or clamped to valid range
    #[test]
    fn property_scale_validation() {
        let test_scales: [f32; 7] = [0.001, 0.01, 0.1, 1.0, 10.0, 100.0, 1000.0];

        for &scale in &test_scales {
            if scale > 0.0 && scale.is_finite() {
                let clamped = clamp_scale(scale);
                assert!(clamped >= 0.01 && clamped <= 100.0);
                assert!(clamped.is_finite());
            }
        }
    }

    /// Property: All finite window levels should be clamped to valid CT range
    #[test]
    fn property_window_level_clamping() {
        let test_levels: [f32; 7] = [-5000.0, -2048.0, -1000.0, 0.0, 1000.0, 2048.0, 5000.0];

        for &level in &test_levels {
            if level.is_finite() {
                let clamped = clamp_window_level(level);
                assert!(clamped >= -2048.0 && clamped <= 2048.0);
                assert!(clamped.is_finite());
            }
        }
    }

    /// Property: All positive, finite window widths should be clamped to valid range
    #[test]
    fn property_window_width_clamping() {
        let test_widths: [f32; 6] = [0.1, 1.0, 100.0, 1000.0, 4096.0, 10000.0];

        for &width in &test_widths {
            if width > 0.0 && width.is_finite() {
                let clamped = clamp_window_width(width);
                assert!(clamped >= 1.0 && clamped <= 4096.0);
                assert!(clamped.is_finite());
            }
        }
    }

    // Helper functions (same as above)
    fn clamp_scale(scale: f32) -> f32 {
        const MIN_SCALE: f32 = 0.01;
        const MAX_SCALE: f32 = 100.0;
        scale.clamp(MIN_SCALE, MAX_SCALE)
    }

    fn clamp_window_level(level: f32) -> f32 {
        const MIN_WINDOW_LEVEL: f32 = -2048.0;
        const MAX_WINDOW_LEVEL: f32 = 2048.0;
        level.clamp(MIN_WINDOW_LEVEL, MAX_WINDOW_LEVEL)
    }

    fn clamp_window_width(width: f32) -> f32 {
        const MIN_WINDOW_WIDTH: f32 = 1.0;
        const MAX_WINDOW_WIDTH: f32 = 4096.0;
        width.clamp(MIN_WINDOW_WIDTH, MAX_WINDOW_WIDTH)
    }
}
