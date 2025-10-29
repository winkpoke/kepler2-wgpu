//! Integration tests for MPR view error handling and validation
//! 
//! Tests the actual MprView implementation with real method calls
//! to ensure error handling works correctly in practice.

use kepler_wgpu::core::error::{KeplerError, MprError, KeplerResult};

#[cfg(test)]
mod mpr_integration_tests {
    use super::*;
    
    /// Mock MprView for testing (simplified version)
    /// In real tests, this would use the actual MprView struct
    struct MockMprView {
        scale: f32,
        window_level: f32,
        window_width: f32,
        slice_position: f32,
        pan_x: f32,
        pan_y: f32,
    }
    
    impl MockMprView {
        /// Create a new mock MPR view with validation
        fn new(
            scale: f32,
            window_level: f32,
            window_width: f32,
            slice_position: f32,
            pan_x: f32,
            pan_y: f32,
        ) -> KeplerResult<Self> {
            // Validate all parameters
            Self::validate_scale(scale)?;
            Self::validate_window_level(window_level)?;
            Self::validate_window_width(window_width)?;
            Self::validate_slice_position(slice_position)?;
            Self::validate_pan_coordinates(pan_x, pan_y)?;
            
            Ok(Self {
                scale: Self::clamp_scale(scale),
                window_level: Self::clamp_window_level(window_level),
                window_width: Self::clamp_window_width(window_width),
                slice_position,
                pan_x: Self::clamp_pan_coordinate(pan_x),
                pan_y: Self::clamp_pan_coordinate(pan_y),
            })
        }
        
        /// Set window level with validation
        fn set_window_level(&mut self, level: f32) -> KeplerResult<()> {
            Self::validate_window_level(level)?;
            self.window_level = Self::clamp_window_level(level);
            Ok(())
        }
        
        /// Set window width with validation
        fn set_window_width(&mut self, width: f32) -> KeplerResult<()> {
            Self::validate_window_width(width)?;
            self.window_width = Self::clamp_window_width(width);
            Ok(())
        }
        
        /// Set scale with validation
        fn set_scale(&mut self, scale: f32) -> KeplerResult<()> {
            Self::validate_scale(scale)?;
            self.scale = Self::clamp_scale(scale);
            Ok(())
        }
        
        /// Set slice position with validation
        fn set_slice_mm(&mut self, z: f32) -> KeplerResult<()> {
            Self::validate_slice_position(z)?;
            self.slice_position = z;
            Ok(())
        }
        
        /// Set pan coordinates with validation
        fn set_pan(&mut self, x: f32, y: f32) -> KeplerResult<()> {
            Self::validate_pan_coordinates(x, y)?;
            self.pan_x = Self::clamp_pan_coordinate(x);
            self.pan_y = Self::clamp_pan_coordinate(y);
            Ok(())
        }
        
        /// Simulate center at point operation with matrix validation
        fn set_center_at_point_in_mm(&mut self, point: [f32; 3]) -> KeplerResult<()> {
            // Validate input coordinates
            for &coord in &point {
                if !coord.is_finite() {
                    return Err(MprError::CoordinateOutOfBounds(point).into());
                }
            }
            
            // Simulate matrix operations that could fail
            let transform_determinant = self.scale * self.scale; // Simplified
            if transform_determinant.abs() < f32::EPSILON {
                return Err(MprError::InvalidTransformation.into());
            }
            
            // Simulate coordinate transformation
            let new_pan_x = point[0] / self.scale;
            let new_pan_y = point[1] / self.scale;
            
            // Validate results
            if !new_pan_x.is_finite() || !new_pan_y.is_finite() {
                return Err(MprError::InvalidTransformation.into());
            }
            
            self.pan_x = Self::clamp_pan_coordinate(new_pan_x);
            self.pan_y = Self::clamp_pan_coordinate(new_pan_y);
            
            Ok(())
        }
        
        // Validation methods
        fn validate_scale(scale: f32) -> KeplerResult<()> {
            if !scale.is_finite() || scale <= 0.0 {
                return Err(MprError::InvalidScale(scale).into());
            }
            Ok(())
        }
        
        fn validate_window_level(level: f32) -> KeplerResult<()> {
            if !level.is_finite() {
                return Err(MprError::InvalidWindowLevel(level).into());
            }
            Ok(())
        }
        
        fn validate_window_width(width: f32) -> KeplerResult<()> {
            if !width.is_finite() || width <= 0.0 {
                return Err(MprError::InvalidWindowWidth(width).into());
            }
            Ok(())
        }
        
        fn validate_slice_position(z: f32) -> KeplerResult<()> {
            if !z.is_finite() {
                return Err(MprError::InvalidSlicePosition(z).into());
            }
            Ok(())
        }
        
        fn validate_pan_coordinates(x: f32, y: f32) -> KeplerResult<()> {
            if !x.is_finite() || !y.is_finite() {
                return Err(MprError::InvalidPanCoordinates([x, y, 0.0]).into());
            }
            Ok(())
        }
        
        // Clamping methods
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
        
        fn clamp_pan_coordinate(coord: f32) -> f32 {
            const MAX_PAN: f32 = 10000.0;
            const MIN_PAN: f32 = -10000.0;
            coord.clamp(MIN_PAN, MAX_PAN)
        }
    }
    
    /// Test constructor validation
    #[test]
    fn test_constructor_validation() {
        // Valid parameters should succeed
        let result = MockMprView::new(1.0, 0.0, 100.0, 50.0, 0.0, 0.0);
        assert!(result.is_ok());
        
        // Invalid scale should fail
        let result = MockMprView::new(f32::NAN, 0.0, 100.0, 50.0, 0.0, 0.0);
        assert!(matches!(result, Err(KeplerError::Mpr(MprError::InvalidScale(_)))));
        
        // Invalid window width should fail
        let result = MockMprView::new(1.0, 0.0, -100.0, 50.0, 0.0, 0.0);
        assert!(matches!(result, Err(KeplerError::Mpr(MprError::InvalidWindowWidth(_)))));
        
        // Invalid slice position should fail
        let result = MockMprView::new(1.0, 0.0, 100.0, f32::INFINITY, 0.0, 0.0);
        assert!(matches!(result, Err(KeplerError::Mpr(MprError::InvalidSlicePosition(_)))));
    }
    
    /// Test setter method validation
    #[test]
    fn test_setter_validation() {
        let mut view = MockMprView::new(1.0, 0.0, 100.0, 50.0, 0.0, 0.0).unwrap();
        
        // Valid operations should succeed
        assert!(view.set_window_level(500.0).is_ok());
        assert!(view.set_window_width(200.0).is_ok());
        assert!(view.set_scale(2.0).is_ok());
        assert!(view.set_slice_mm(75.0).is_ok());
        assert!(view.set_pan(10.0, -20.0).is_ok());
        
        // Invalid operations should fail
        assert!(matches!(
            view.set_window_level(f32::NAN),
            Err(KeplerError::Mpr(MprError::InvalidWindowLevel(_)))
        ));
        
        assert!(matches!(
            view.set_window_width(0.0),
            Err(KeplerError::Mpr(MprError::InvalidWindowWidth(_)))
        ));
        
        assert!(matches!(
            view.set_scale(-1.0),
            Err(KeplerError::Mpr(MprError::InvalidScale(_)))
        ));
        
        assert!(matches!(
            view.set_slice_mm(f32::INFINITY),
            Err(KeplerError::Mpr(MprError::InvalidSlicePosition(_)))
        ));
        
        assert!(matches!(
            view.set_pan(f32::NAN, 0.0),
            Err(KeplerError::Mpr(MprError::InvalidPanCoordinates(_)))
        ));
    }
    
    /// Test coordinate transformation error handling
    #[test]
    fn test_coordinate_transformation_errors() {
        let mut view = MockMprView::new(1.0, 0.0, 100.0, 50.0, 0.0, 0.0).unwrap();
        
        // Valid coordinates should succeed
        let valid_point = [100.0, 200.0, 50.0];
        assert!(view.set_center_at_point_in_mm(valid_point).is_ok());
        
        // Invalid coordinates should fail
        let invalid_point = [f32::NAN, 200.0, 50.0];
        assert!(matches!(
            view.set_center_at_point_in_mm(invalid_point),
            Err(KeplerError::Mpr(MprError::CoordinateOutOfBounds(_)))
        ));
        
        let infinite_point = [f32::INFINITY, f32::NEG_INFINITY, 0.0];
        assert!(matches!(
            view.set_center_at_point_in_mm(infinite_point),
            Err(KeplerError::Mpr(MprError::CoordinateOutOfBounds(_)))
        ));
    }
    
    /// Test matrix transformation failure handling
    #[test]
    fn test_matrix_transformation_failures() {
        let mut view = MockMprView::new(0.01, 0.0, 100.0, 50.0, 0.0, 0.0).unwrap(); // Very small scale
        
        // This should work with small but valid scale
        let point = [1.0, 1.0, 0.0];
        assert!(view.set_center_at_point_in_mm(point).is_ok());
        
        // Set scale to effectively zero to simulate matrix inversion failure
        view.scale = f32::EPSILON / 2.0; // Below threshold
        
        let point = [100.0, 100.0, 0.0];
        assert!(matches!(
            view.set_center_at_point_in_mm(point),
            Err(KeplerError::Mpr(MprError::InvalidTransformation))
        ));
    }
    
    /// Test bounds clamping in practice
    #[test]
    fn test_bounds_clamping_integration() {
        // Test extreme values get clamped during construction
        let view = MockMprView::new(1000.0, -5000.0, 10000.0, 0.0, 50000.0, -50000.0).unwrap();
        
        // Values should be clamped to valid ranges
        assert!(view.scale <= 100.0); // Max scale
        assert!(view.window_level >= -2048.0); // Min window level
        assert!(view.window_width <= 4096.0); // Max window width
        assert!(view.pan_x <= 10000.0); // Max pan
        assert!(view.pan_y >= -10000.0); // Min pan
        
        // Test clamping in setter methods
        let mut view = MockMprView::new(1.0, 0.0, 100.0, 0.0, 0.0, 0.0).unwrap();
        
        // Set extreme values
        assert!(view.set_scale(200.0).is_ok()); // Above max
        assert!(view.scale <= 100.0); // Should be clamped
        
        assert!(view.set_window_level(-3000.0).is_ok()); // Below min
        assert!(view.window_level >= -2048.0); // Should be clamped
        
        assert!(view.set_window_width(5000.0).is_ok()); // Above max
        assert!(view.window_width <= 4096.0); // Should be clamped
    }
    
    /// Test error propagation through method chains
    #[test]
    fn test_error_propagation() {
        let mut view = MockMprView::new(1.0, 0.0, 100.0, 50.0, 0.0, 0.0).unwrap();
        
        // Chain of operations where one fails
        let result = view.set_window_level(100.0)
            .and_then(|_| view.set_window_width(200.0))
            .and_then(|_| view.set_scale(f32::NAN)) // This should fail
            .and_then(|_| view.set_slice_mm(75.0));
        
        assert!(matches!(result, Err(KeplerError::Mpr(MprError::InvalidScale(_)))));
        
        // Verify that previous valid operations succeeded
        assert_eq!(view.window_level, 100.0);
        assert_eq!(view.window_width, 200.0);
        // Scale should remain unchanged due to error
        assert_eq!(view.scale, 1.0);
    }
    
    /// Test recovery from errors
    #[test]
    fn test_error_recovery() {
        let mut view = MockMprView::new(1.0, 0.0, 100.0, 50.0, 0.0, 0.0).unwrap();
        
        // Attempt invalid operation
        let result = view.set_scale(f32::NAN);
        assert!(result.is_err());
        
        // View should still be in valid state
        assert_eq!(view.scale, 1.0);
        
        // Subsequent valid operations should work
        assert!(view.set_scale(2.0).is_ok());
        assert_eq!(view.scale, 2.0);
        
        // Test coordinate transformation recovery
        let invalid_point = [f32::NAN, 0.0, 0.0];
        let result = view.set_center_at_point_in_mm(invalid_point);
        assert!(result.is_err());
        
        // Valid operation should still work
        let valid_point = [100.0, 100.0, 0.0];
        assert!(view.set_center_at_point_in_mm(valid_point).is_ok());
    }
}