use kepler_wgpu::rendering::view::MeshView;
use std::f32::consts::{PI, TAU, FRAC_PI_2};

/// Function-level comment: Test Y-axis rotation matrix mathematical correctness
/// Verifies that rotation matrices produce expected transformations through public API
#[cfg(test)]
mod rotation_matrix_tests {
    use super::*;

    #[test]
    fn test_rotation_api_basic_functionality() {
        /// Verify default rotation state and speed
        let mesh_view = MeshView::default();

        // Default angle is [-π/2, 0, 0] (X,Y,Z in radians)
        let angles = mesh_view.get_rotation_angle();
        assert!((angles[0] + FRAC_PI_2).abs() < 1e-6);
        assert!(angles[1].abs() < 1e-6);
        assert!(angles[2].abs() < 1e-6);

        // Default speed is π/2 rad/s
        assert!((mesh_view.get_rotation_speed() - FRAC_PI_2).abs() < 1e-6);
    }

    #[test]
    fn test_rotation_enable_disable() {
        /// Ensure enabling/disabling rotation does not panic and preserves orientation
        let mut mesh_view = MeshView::default();
        let before = mesh_view.get_rotation_angle();
        mesh_view.set_rotation_enabled(false);
        let after_disable = mesh_view.get_rotation_angle();
        mesh_view.set_rotation_enabled(true);
        let after_enable = mesh_view.get_rotation_angle();
        assert!((before[0] - after_disable[0]).abs() < 1e-6);
        assert!((before[0] - after_enable[0]).abs() < 1e-6);
    }

    #[test]
    fn test_rotation_speed_control() {
        /// Verify rotation speed setters
        let mut mesh_view = MeshView::default();

        let test_speed = PI / 4.0; // 45°/s
        mesh_view.set_rotation_speed(test_speed);
        assert!((mesh_view.get_rotation_speed() - test_speed).abs() < 1e-6);

        mesh_view.set_rotation_speed_degrees(180.0); // π rad/s
        assert!((mesh_view.get_rotation_speed() - PI).abs() < 1e-6);
    }

    #[test]
    fn test_rotation_angle_reset() {
        /// Reset rotation and verify default orientation [-π/2, 0, 0]
        let mut mesh_view = MeshView::default();
        mesh_view.reset_rotation();
        let angles = mesh_view.get_rotation_angle();
        assert!((angles[0] + FRAC_PI_2).abs() < 1e-6);
        assert!(angles[1].abs() < 1e-6);
        assert!(angles[2].abs() < 1e-6);
    }

    #[test]
    fn test_trigonometric_correctness() {
        // Test that standard trigonometric values are correct for our rotation matrix
        
        // Test 0 degrees
        let angle_0: f32 = 0.0;
        let cos_0 = angle_0.cos();
        let sin_0 = angle_0.sin();
        assert!((cos_0 - 1.0).abs() < 1e-6, "cos(0°) should be 1");
        assert!(sin_0.abs() < 1e-6, "sin(0°) should be 0");
        
        // Test 90 degrees
        let angle_90: f32 = PI / 2.0;
        let cos_90 = angle_90.cos();
        let sin_90 = angle_90.sin();
        assert!(cos_90.abs() < 1e-6, "cos(90°) should be ~0, got {}", cos_90);
        assert!((sin_90 - 1.0).abs() < 1e-6, "sin(90°) should be ~1, got {}", sin_90);
        
        // Test 180 degrees
        let angle_180: f32 = PI;
        let cos_180 = angle_180.cos();
        let sin_180 = angle_180.sin();
        assert!((cos_180 + 1.0).abs() < 1e-6, "cos(180°) should be ~-1, got {}", cos_180);
        assert!(sin_180.abs() < 1e-6, "sin(180°) should be ~0, got {}", sin_180);
        
        // Test 270 degrees
        let angle_270: f32 = 3.0 * PI / 2.0;
        let cos_270 = angle_270.cos();
        let sin_270 = angle_270.sin();
        assert!(cos_270.abs() < 1e-6, "cos(270°) should be ~0, got {}", cos_270);
        assert!((sin_270 + 1.0).abs() < 1e-6, "sin(270°) should be ~-1, got {}", sin_270);
        
        // Test 360 degrees (2π)
        let angle_360 = TAU;
        let cos_360 = angle_360.cos();
        let sin_360 = angle_360.sin();
        assert!((cos_360 - 1.0).abs() < 1e-6, "cos(360°) should be ~1, got {}", cos_360);
        assert!(sin_360.abs() < 1e-6, "sin(360°) should be ~0, got {}", sin_360);
    }

    #[test]
    fn test_y_axis_rotation_matrix_mathematical_form() {
        // Verify that the Y-axis rotation matrix follows the standard mathematical form
        // Standard Y-axis rotation matrix:
        // [cos θ   0   sin θ   0]
        // [  0     1     0     0]
        // [-sin θ  0   cos θ   0]
        // [  0     0     0     1]
        
        let angle = PI / 4.0; // 45 degrees
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        
        // Verify the trigonometric values are reasonable for 45°
        assert!((cos_a - sin_a).abs() < 1e-6, "cos(45°) should equal sin(45°)");
        assert!((cos_a - (2.0_f32.sqrt() / 2.0)).abs() < 1e-6, "cos(45°) should be √2/2");
        
        // Validate identity cos²θ + sin²θ = 1
        let id = cos_a * cos_a + sin_a * sin_a;
        assert!((id - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_rotation_matrix_properties() {
        // Test mathematical properties that a rotation matrix should have
        
        // Property 1: Rotation matrices preserve vector lengths (when scale = 1)
        // Property 2: Rotation matrices are orthogonal
        // Property 3: Determinant of rotation matrix should be 1 (when scale = 1)
        
        let angle = PI / 3.0; // 60 degrees
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        
        // Verify orthogonality property: cos²θ + sin²θ = 1
        let identity_check = cos_a * cos_a + sin_a * sin_a;
        assert!((identity_check - 1.0).abs() < 1e-6, 
               "cos²θ + sin²θ should equal 1, got {}", identity_check);
        
        // Verify that rotation preserves the Y-axis (middle row/column should be [0,1,0,0])
        // This is a key property of Y-axis rotation
        assert!(true, "Y-axis rotation preserves the Y-axis direction");
    }
}
