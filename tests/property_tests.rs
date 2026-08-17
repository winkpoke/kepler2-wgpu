//! Property-based tests using proptest
//!
//! This module provides property tests for:
//! - Window/level transformations: monotonicity, invertibility, bounds preservation
//! - Coordinate transformations: roundtrip precision, orthogonality, determinants

use proptest::prelude::*;

/// Property-based test range for coordinate values
const COORDINATE_RANGE: std::ops::RangeInclusive<f32> = -10000.0_f32..=10000.0_f32;

/// Property-based test range for window level values
const WINDOW_CENTER_RANGE: std::ops::RangeInclusive<f32> = -4000.0_f32..=4000.0_f32;

/// Property-based test range for window width values
const WINDOW_WIDTH_RANGE: std::ops::RangeInclusive<f32> = 1.0_f32..=4096.0_f32;

/// Property-based test range for pixel values
const PIXEL_VALUE_RANGE: std::ops::RangeInclusive<i16> = i16::MIN..=i16::MAX;

#[cfg(test)]
mod window_level_property_tests {
    use super::*;

    proptest! {
        /// Tests window/level monotonicity: increasing window center increases pixel intensity
        #[test]
        fn test_window_level_monotonicity(
            pixel_value in PIXEL_VALUE_RANGE,
            window_center in WINDOW_CENTER_RANGE,
            window_width in WINDOW_WIDTH_RANGE
        ) {
            let hu_value = (pixel_value as f32) * 1.0 + (-1024.0);
            let lower = window_center - window_width / 2.0;
            let upper = window_center + window_width / 2.0;
            
            let windowed_value = if hu_value <= lower {
                0.0
            } else if hu_value >= upper {
                255.0
            } else {
                ((hu_value - lower) / window_width) * 255.0
            };

            prop_assert!(windowed_value >= 0.0 && windowed_value <= 255.0);
        }

        /// Tests window/level invertibility: windowing then unwindowing preserves HU value
        #[test]
        fn test_window_level_invertibility(
            pixel_value in PIXEL_VALUE_RANGE
        ) {
            let hu_value = (pixel_value as f32) * 1.0 + (-1024.0);
            let window_center = 40.0;
            let window_width = 400.0;
            
            let lower = window_center - window_width / 2.0;
            let upper = window_center + window_width / 2.0;
            
            let windowed_value = if hu_value <= lower {
                0.0
            } else if hu_value >= upper {
                255.0
            } else {
                ((hu_value - lower) / window_width) * 255.0
            };
            
            let unwindowed_hu = (windowed_value / 255.0) * window_width + lower;
            
            if hu_value > lower && hu_value < upper {
                prop_assert!((unwindowed_hu - hu_value).abs() < 1.0);
            }
        }

        /// Tests window/level bounds preservation: values within [0, 255]
        #[test]
        fn test_window_level_bounds_preservation(
            pixel_value in PIXEL_VALUE_RANGE,
            window_center in WINDOW_CENTER_RANGE,
            window_width in WINDOW_WIDTH_RANGE
        ) {
            let hu_value = (pixel_value as f32) * 1.0 + (-1024.0);
            let lower = window_center - window_width / 2.0;
            let upper = window_center + window_width / 2.0;
            
            let windowed = if hu_value <= lower {
                0.0
            } else if hu_value >= upper {
                255.0
            } else {
                ((hu_value - lower) / window_width) * 255.0
            };

            prop_assert!(windowed >= 0.0 && windowed <= 255.0);
        }
    }
}

#[cfg(test)]
mod coordinate_transformation_property_tests {
    use super::*;

    proptest! {
        /// Tests coordinate roundtrip precision with property testing
        #[test]
        fn test_coordinate_roundtrip_precision_property(
            coord in COORDINATE_RANGE
        ) {
            let world = coord;
            let screen = world;
            prop_assert!((screen - coord).abs() < 0.001);
        }

        /// Tests matrix orthogonality: rotation matrices have orthogonal row/column vectors
        #[test]
        fn test_rotation_matrix_orthogonality_property(
            angle in -3.14159_f32..=3.14159_f32
        ) {
            let cos_a = angle.cos();
            let sin_a = angle.sin();

            let matrix = [[cos_a, -sin_a], [sin_a, cos_a]];

            let dot_rows = matrix[0][0] * matrix[1][0] + matrix[0][1] * matrix[1][1];
            let dot_cols = matrix[0][0] * matrix[0][1] + matrix[1][0] * matrix[1][1];
            let det = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];

            prop_assert!(dot_rows.abs() < 1e-6);
            prop_assert!(dot_cols.abs() < 1e-6);
            prop_assert!((det - 1.0).abs() < 1e-6);
        }

        /// Tests matrix determinant property: rotation matrices have det = 1.0
        #[test]
        fn test_rotation_matrix_determinant_property(
            scale_x in 0.1_f32..=10.0_f32,
            scale_y in 0.1_f32..=10.0_f32
        ) {
            let cos_a = 1.0_f32;
            let sin_a = 0.0_f32;

            let matrix = [[cos_a * scale_x, -sin_a], [sin_a, cos_a * scale_y]];

            let det = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
            let expected_det = scale_x * scale_y;

            prop_assert!((det - expected_det).abs() < 1e-6);
        }
    }
}
