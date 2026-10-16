//! Coordinate roundtrip precision tests
//!
//! This module provides tests for coordinate transformation roundtrip accuracy,
//! NaN/infinity propagation, and precision tolerance enforcement
//! for screen-to-world and world-to-screen coordinate transformations.

use glam::{Mat4, Vec3, Vec4};
use kepler_wgpu::core::coord::Base;

/// Tolerance for roundtrip precision (0.001 mm as per medical imaging standards)
const ROUNDTRIP_TOLERANCE_MM: f32 = 0.001;

/// Very large coordinate values for testing (>10000 mm)
const LARGE_COORDINATE_MM: f32 = 15000.0;

#[cfg(test)]
mod roundtrip_tests {
    use super::*;

    /// Tests coordinate roundtrip maintains precision within tolerance
    #[test]
    fn test_coordinate_roundtrip_precision() {
        let world_base = Base {
            label: "world".to_string(),
            matrix: Mat4::from_translation(Vec3::new(10.0, 20.0, 30.0)),
        };

        let screen_base = Base {
            label: "screen".to_string(),
            matrix: Mat4::from_translation(Vec3::new(5.0, 10.0, 15.0)),
        };

        let world_point = Vec3::new(100.0, 200.0, 300.0);

        let screen_matrix = world_base.to_base(&screen_base);
        let screen_point_4d =
            screen_matrix * Vec4::new(world_point.x, world_point.y, world_point.z, 1.0);
        let screen_point = Vec3::new(screen_point_4d.x, screen_point_4d.y, screen_point_4d.z);

        let world_matrix = screen_base.to_base(&world_base);
        let world_point_4d =
            world_matrix * Vec4::new(screen_point.x, screen_point.y, screen_point.z, 1.0);
        let world_point_roundtrip = Vec3::new(world_point_4d.x, world_point_4d.y, world_point_4d.z);

        let diff = (world_point - world_point_roundtrip).length();
        assert!(
            diff < ROUNDTRIP_TOLERANCE_MM,
            "Roundtrip precision failed: {} difference, expected < {}",
            diff,
            ROUNDTRIP_TOLERANCE_MM
        );
    }

    /// Tests roundtrip with large coordinate values
    #[test]
    fn test_roundtrip_with_large_coordinates() {
        let world_base = Base {
            label: "world".to_string(),
            matrix: Mat4::from_translation(Vec3::new(100.0, 200.0, 300.0)),
        };

        let screen_base = Base {
            label: "screen".to_string(),
            matrix: Mat4::from_translation(Vec3::new(50.0, 100.0, 150.0)),
        };

        let world_point = Vec3::new(
            LARGE_COORDINATE_MM,
            LARGE_COORDINATE_MM,
            LARGE_COORDINATE_MM,
        );

        let screen_matrix = world_base.to_base(&screen_base);
        let screen_point_4d =
            screen_matrix * Vec4::new(world_point.x, world_point.y, world_point.z, 1.0);
        let screen_point = Vec3::new(screen_point_4d.x, screen_point_4d.y, screen_point_4d.z);

        let world_matrix = screen_base.to_base(&world_base);
        let world_point_4d =
            world_matrix * Vec4::new(screen_point.x, screen_point.y, screen_point.z, 1.0);
        let world_point_roundtrip = Vec3::new(world_point_4d.x, world_point_4d.y, world_point_4d.z);

        let diff = (world_point - world_point_roundtrip).length();
        assert!(
            diff < ROUNDTRIP_TOLERANCE_MM,
            "Roundtrip with large coordinates failed: {} difference, expected < {}",
            diff,
            ROUNDTRIP_TOLERANCE_MM
        );
    }

    /// Tests roundtrip with zero coordinates
    #[test]
    fn test_roundtrip_with_zero_coordinates() {
        let world_base = Base {
            label: "world".to_string(),
            matrix: Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        };

        let screen_base = Base {
            label: "screen".to_string(),
            matrix: Mat4::IDENTITY,
        };

        let world_point = Vec3::ZERO;

        let screen_matrix = world_base.to_base(&screen_base);
        let screen_point_4d =
            screen_matrix * Vec4::new(world_point.x, world_point.y, world_point.z, 1.0);
        let screen_point = Vec3::new(screen_point_4d.x, screen_point_4d.y, screen_point_4d.z);

        let world_matrix = screen_base.to_base(&world_base);
        let world_point_4d =
            world_matrix * Vec4::new(screen_point.x, screen_point.y, screen_point.z, 1.0);
        let world_point_roundtrip = Vec3::new(world_point_4d.x, world_point_4d.y, world_point_4d.z);

        let diff = (world_point - world_point_roundtrip).length();
        assert!(
            diff < ROUNDTRIP_TOLERANCE_MM,
            "Roundtrip with zero coordinates failed: {} difference, expected < {}",
            diff,
            ROUNDTRIP_TOLERANCE_MM
        );
    }

    /// Tests roundtrip with fractional coordinates
    #[test]
    fn test_roundtrip_with_fractional_coordinates() {
        let world_base = Base {
            label: "world".to_string(),
            matrix: Mat4::from_translation(Vec3::new(10.5, 20.7, 30.3)),
        };

        let screen_base = Base {
            label: "screen".to_string(),
            matrix: Mat4::from_translation(Vec3::new(5.25, 10.5, 15.75)),
        };

        let world_point = Vec3::new(100.123, 200.456, 300.789);

        let screen_matrix = world_base.to_base(&screen_base);
        let screen_point_4d =
            screen_matrix * Vec4::new(world_point.x, world_point.y, world_point.z, 1.0);
        let screen_point = Vec3::new(screen_point_4d.x, screen_point_4d.y, screen_point_4d.z);

        let world_matrix = screen_base.to_base(&world_base);
        let world_point_4d =
            world_matrix * Vec4::new(screen_point.x, screen_point.y, screen_point.z, 1.0);
        let world_point_roundtrip = Vec3::new(world_point_4d.x, world_point_4d.y, world_point_4d.z);

        let diff = (world_point - world_point_roundtrip).length();
        assert!(
            diff < ROUNDTRIP_TOLERANCE_MM,
            "Roundtrip with fractional coordinates failed: {} difference, expected < {}",
            diff,
            ROUNDTRIP_TOLERANCE_MM
        );
    }
}

#[cfg(test)]
mod special_value_propagation_tests {
    use super::*;

    // IEEE 754 contract for the coordinate pipeline: Base::to_base() is a
    // 4x4 matrix multiply. A translation matrix has zeros off the diagonal,
    // and 0 * NaN = NaN, so a non-finite input coordinate contaminates EVERY
    // component of the transformed point — not just its own axis. These tests
    // lock that semantic: non-finite values propagate (no silent
    // sanitization) and no operation panics. Callers must validate inputs
    // upstream if they need finite guarantees.

    /// Tests NaN propagation through coordinate transformations
    #[test]
    fn test_nan_propagation_screen_to_world() {
        let world_base = Base {
            label: "world".to_string(),
            matrix: Mat4::IDENTITY,
        };

        let screen_base = Base {
            label: "screen".to_string(),
            matrix: Mat4::from_translation(Vec3::new(10.0, 20.0, 30.0)),
        };

        let nan_point = Vec3::new(f32::NAN, 200.0, 300.0);

        let screen_matrix = world_base.to_base(&screen_base);
        let screen_point_4d = screen_matrix * Vec4::new(nan_point.x, nan_point.y, nan_point.z, 1.0);

        assert!(
            !screen_point_4d.x.is_finite(),
            "NaN should propagate in x coordinate"
        );
        assert!(
            !screen_point_4d.y.is_finite(),
            "0 * NaN = NaN: y is contaminated by the x NaN (IEEE 754)"
        );
        assert!(
            !screen_point_4d.z.is_finite(),
            "0 * NaN = NaN: z is contaminated by the x NaN (IEEE 754)"
        );
    }

    /// Tests NaN propagation through world-to-screen transformations
    #[test]
    fn test_nan_propagation_world_to_screen() {
        let world_base = Base {
            label: "world".to_string(),
            matrix: Mat4::from_translation(Vec3::new(10.0, 20.0, 30.0)),
        };

        let screen_base = Base {
            label: "screen".to_string(),
            matrix: Mat4::IDENTITY,
        };

        let nan_point = Vec3::new(100.0, f32::NAN, 300.0);

        let screen_matrix = world_base.to_base(&screen_base);
        let screen_point_4d = screen_matrix * Vec4::new(nan_point.x, nan_point.y, nan_point.z, 1.0);

        assert!(
            !screen_point_4d.y.is_finite(),
            "NaN should propagate in y coordinate"
        );
        assert!(
            !screen_point_4d.x.is_finite(),
            "0 * NaN = NaN: x is contaminated by the y NaN (IEEE 754)"
        );
        assert!(
            !screen_point_4d.z.is_finite(),
            "0 * NaN = NaN: z is contaminated by the y NaN (IEEE 754)"
        );
    }

    /// Tests infinity propagation through coordinate transformations
    #[test]
    fn test_infinity_propagation_screen_to_world() {
        let world_base = Base {
            label: "world".to_string(),
            matrix: Mat4::IDENTITY,
        };

        let screen_base = Base {
            label: "screen".to_string(),
            matrix: Mat4::from_translation(Vec3::new(10.0, 20.0, 30.0)),
        };

        let inf_point = Vec3::new(f32::INFINITY, 200.0, 300.0);

        let screen_matrix = world_base.to_base(&screen_base);
        let screen_point_4d = screen_matrix * Vec4::new(inf_point.x, inf_point.y, inf_point.z, 1.0);

        assert!(
            !screen_point_4d.x.is_finite(),
            "Positive infinity should propagate in x coordinate"
        );
        assert!(
            !screen_point_4d.y.is_finite(),
            "0 * Inf = NaN: y is contaminated by the x Inf (IEEE 754)"
        );
        assert!(
            !screen_point_4d.z.is_finite(),
            "0 * Inf = NaN: z is contaminated by the x Inf (IEEE 754)"
        );
    }

    /// Tests negative infinity propagation
    #[test]
    fn test_negative_infinity_propagation() {
        let world_base = Base {
            label: "world".to_string(),
            matrix: Mat4::from_translation(Vec3::new(10.0, 20.0, 30.0)),
        };

        let screen_base = Base {
            label: "screen".to_string(),
            matrix: Mat4::from_translation(Vec3::new(10.0, 20.0, 30.0)),
        };

        let neg_inf_point = Vec3::new(100.0, 200.0, f32::NEG_INFINITY);

        let screen_matrix = world_base.to_base(&screen_base);
        let screen_point_4d =
            screen_matrix * Vec4::new(neg_inf_point.x, neg_inf_point.y, neg_inf_point.z, 1.0);

        assert!(
            !screen_point_4d.z.is_finite(),
            "Negative infinity should propagate in z coordinate"
        );
        assert!(
            !screen_point_4d.x.is_finite(),
            "0 * -Inf = NaN: x is contaminated by the z -Inf (IEEE 754)"
        );
        assert!(
            !screen_point_4d.y.is_finite(),
            "0 * -Inf = NaN: y is contaminated by the z -Inf (IEEE 754)"
        );
    }

    /// Tests very large coordinates exceed reasonable bounds
    #[test]
    fn test_extreme_coordinates_handling() {
        let world_base = Base {
            label: "world".to_string(),
            matrix: Mat4::IDENTITY,
        };

        let screen_base = Base {
            label: "screen".to_string(),
            matrix: Mat4::from_translation(Vec3::new(10.0, 20.0, 30.0)),
        };

        let extreme_point = Vec3::new(1e7, 1e7, 1e7);

        let screen_matrix = world_base.to_base(&screen_base);
        let screen_point_4d =
            screen_matrix * Vec4::new(extreme_point.x, extreme_point.y, extreme_point.z, 1.0);

        assert!(
            screen_point_4d.x.is_finite(),
            "Extreme x coordinate should remain finite"
        );
        assert!(
            screen_point_4d.y.is_finite(),
            "Extreme y coordinate should remain finite"
        );
        assert!(
            screen_point_4d.z.is_finite(),
            "Extreme z coordinate should remain finite"
        );
    }
}
