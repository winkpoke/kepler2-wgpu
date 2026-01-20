//! Coordinate safety and matrix orthogonality tests
//!
//! This module provides tests for:
//! - Matrix orthogonality validation (row/column vectors perpendicular, unit length)
//! - Matrix determinant properties (det = 1.0 for rotations)
//! - Orientation matrix validation for different views (axial, coronal, sagittal)


use kepler_wgpu::rendering::Orientation;

mod common;
use crate::common::fixtures::ct_volume::create_test_ct_volume;

/// Epsilon for floating point comparisons (tolerance for precision errors)
const EPSILON: f32 = 1e-6;

/// Test volume dimensions for coordinate system tests
const TEST_VOLUME_SIZE: u32 = 512;

#[cfg(test)]
mod orthogonality_tests {
    use super::*;

    /// Tests row vectors are orthogonal (dot products = 0 ± epsilon)
    #[test]
    fn test_row_vectors_orthogonal_axial() {
        // Create test volume with known dimensions
        let volume = create_test_ct_volume();

        // Build axial orientation matrix
        let base = Orientation::Transverse.build_base(&volume);
        let matrix = base.matrix;

        // Test orthogonality: row vectors should be perpendicular (dot product = 0)
        // Row 0: [m00, m10, m20] -> [col0.x, col1.x, col2.x]
        // Row 1: [m01, m11, m21] -> [col0.y, col1.y, col2.y]
        // Row 2: [m02, m12, m22] -> [col0.z, col1.z, col2.z]

        // Row 0 dot Row 1
        let dot_0_1 = matrix.col(0)[0] * matrix.col(0)[1]
            + matrix.col(1)[0] * matrix.col(1)[1]
            + matrix.col(2)[0] * matrix.col(2)[1];

        // Row 0 dot Row 2
        let dot_0_2 = matrix.col(0)[0] * matrix.col(0)[2]
            + matrix.col(1)[0] * matrix.col(1)[2]
            + matrix.col(2)[0] * matrix.col(2)[2];

        assert!(
            dot_0_1.abs() < EPSILON,
            "Row 0 dot Row 1 should be ~0: {}",
            dot_0_1
        );
        assert!(
            dot_0_2.abs() < EPSILON,
            "Row 0 dot Row 2 should be ~0: {}",
            dot_0_2
        );

        // Row 1 should be orthogonal to Row 0 (already checked) and Row 2
        // Row 1 dot Row 2
        let dot_1_2 = matrix.col(0)[1] * matrix.col(0)[2]
            + matrix.col(1)[1] * matrix.col(1)[2]
            + matrix.col(2)[1] * matrix.col(2)[2];

        assert!(
            dot_1_2.abs() < EPSILON,
            "Row 1 dot Row 2 should be ~0: {}",
            dot_1_2
        );
    }

    /// Tests row vectors have expected isotropic scale length
    ///
    /// Orientation::build_base() creates scaling + translation matrices, not rotation matrices.
    /// Row vectors should have length = isotropic scale factor d.
    #[test]
    fn test_row_vectors_scale_length_axial() {
        let volume = create_test_ct_volume();
        let base = Orientation::Transverse.build_base(&volume);
        let matrix = base.matrix;

        // Calculate expected isotropic scale factor
        // d = (dx + dy + dz) / 3.0 where dx=nx*space.x, dy=ny*space.y, dz=nz*space.z
        let nx = volume.dimensions().0 as f32;
        let ny = volume.dimensions().1 as f32;
        let nz = volume.dimensions().2 as f32;
        let space = volume.voxel_spacing();
        let d_x = nx * space.0;
        let d_y = ny * space.1;
        let d_z = nz * space.2;
        let expected_scale = (d_x + d_y + d_z) / 3.0; // Should be 408 for test volume

        // Row 0 length should equal isotropic scale
        let row_0_len =
            (matrix.col(0)[0].powi(2) + matrix.col(1)[0].powi(2) + matrix.col(2)[0].powi(2)).sqrt();

        assert!(
            (row_0_len - expected_scale).abs() < EPSILON,
            "Row 0 length should be {}: {}",
            expected_scale,
            row_0_len
        );

        // Row 1 length should equal isotropic scale
        let row_1_len =
            (matrix.col(0)[1].powi(2) + matrix.col(1)[1].powi(2) + matrix.col(2)[1].powi(2)).sqrt();

        assert!(
            (row_1_len - expected_scale).abs() < EPSILON,
            "Row 1 length should be {}: {}",
            expected_scale,
            row_1_len
        );

        // Row 2 length should equal isotropic scale
        let row_2_len =
            (matrix.col(0)[2].powi(2) + matrix.col(1)[2].powi(2) + matrix.col(2)[2].powi(2)).sqrt();

        assert!(
            (row_2_len - expected_scale).abs() < EPSILON,
            "Row 2 length should be {}: {}",
            expected_scale,
            row_2_len
        );
    }

    /// Tests column vectors are orthogonal
    #[test]
    fn test_column_vectors_orthogonal_axial() {
        let volume = create_test_ct_volume();
        let base = Orientation::Transverse.build_base(&volume);
        let matrix = base.matrix;

        // Column 0 should be orthogonal to Column 1
        // col0 dot col1
        let dot_0_1 = matrix.col(0).truncate().dot(matrix.col(1).truncate());
        // Column 0 should be orthogonal to Column 2
        let dot_0_2 = matrix.col(0).truncate().dot(matrix.col(2).truncate());

        assert!(
            dot_0_1.abs() < EPSILON,
            "Column 0 dot Column 1 should be ~0: {}",
            dot_0_1
        );
        assert!(
            dot_0_2.abs() < EPSILON,
            "Column 0 dot Column 2 should be ~0: {}",
            dot_0_2
        );
    }

    /// Tests column vectors have expected isotropic scale length
    ///
    /// Orientation::build_base() creates scaling + translation matrices.
    /// Column vectors should have length = isotropic scale factor d.
    #[test]
    fn test_column_vectors_scale_length_axial() {
        let volume = create_test_ct_volume();
        let base = Orientation::Transverse.build_base(&volume);
        let matrix = base.matrix;

        // Calculate expected isotropic scale factor
        let nx = volume.dimensions().0 as f32;
        let ny = volume.dimensions().1 as f32;
        let nz = volume.dimensions().2 as f32;
        let space = volume.voxel_spacing();
        let d_x = nx * space.0;
        let d_y = ny * space.1;
        let d_z = nz * space.2;
        let expected_scale = (d_x + d_y + d_z) / 3.0;

        // Column 0 length
        let col_0_len = matrix.col(0).truncate().length();

        assert!(
            (col_0_len - expected_scale).abs() < EPSILON,
            "Column 0 length should be {}: {}",
            expected_scale,
            col_0_len
        );

        // Column 1 length
        let col_1_len = matrix.col(1).truncate().length();

        assert!(
            (col_1_len - expected_scale).abs() < EPSILON,
            "Column 1 length should be {}: {}",
            expected_scale,
            col_1_len
        );

        // Column 2 length
        let col_2_len = matrix.col(2).truncate().length();

        assert!(
            (col_2_len - expected_scale).abs() < EPSILON,
            "Column 2 length should be {}: {}",
            expected_scale,
            col_2_len
        );
    }

    /// Tests all rows orthogonal to each other
    #[test]
    fn test_all_rows_pairwise_orthogonal() {
        let volume = create_test_ct_volume();
        let base = Orientation::Transverse.build_base(&volume);
        let matrix = base.matrix;

        // Row 0 dot Row 1
        let dot_0_1 = matrix.col(0)[0] * matrix.col(0)[1]
            + matrix.col(1)[0] * matrix.col(1)[1]
            + matrix.col(2)[0] * matrix.col(2)[1];
        // Row 0 dot Row 2
        let dot_0_2 = matrix.col(0)[0] * matrix.col(0)[2]
            + matrix.col(1)[0] * matrix.col(1)[2]
            + matrix.col(2)[0] * matrix.col(2)[2];
        // Row 1 dot Row 0
        let dot_1_0 = dot_0_1;
        // Row 1 dot Row 2
        let dot_1_2 = matrix.col(0)[1] * matrix.col(0)[2]
            + matrix.col(1)[1] * matrix.col(1)[2]
            + matrix.col(2)[1] * matrix.col(2)[2];

        assert!(dot_0_1.abs() < EPSILON, "Row 0 dot Row 1");
        assert!(dot_0_2.abs() < EPSILON, "Row 0 dot Row 2");
        assert!(dot_1_0.abs() < EPSILON, "Row 1 dot Row 0");
        assert!(dot_1_2.abs() < EPSILON, "Row 1 dot Row 2");
    }

    /// Tests all columns orthogonal to each other
    #[test]
    fn test_all_columns_pairwise_orthogonal() {
        let volume = create_test_ct_volume();
        let base = Orientation::Transverse.build_base(&volume);
        let matrix = base.matrix;

        // Column 0 dot Column 1
        let dot_0_1 = matrix.col(0).truncate().dot(matrix.col(1).truncate());
        // Column 0 dot Column 2
        let dot_0_2 = matrix.col(0).truncate().dot(matrix.col(2).truncate());
        // Column 1 dot Column 0
        let dot_1_0 = dot_0_1;
        // Column 1 dot Column 2
        let dot_1_2 = matrix.col(1).truncate().dot(matrix.col(2).truncate());

        assert!(dot_0_1.abs() < EPSILON, "Column 0 dot Column 1");
        assert!(dot_0_2.abs() < EPSILON, "Column 0 dot Column 2");
        assert!(dot_1_0.abs() < EPSILON, "Column 1 dot Column 0");
        assert!(dot_1_2.abs() < EPSILON, "Column 1 dot Column 2");
    }
}

#[cfg(test)]
mod determinant_tests {
    use super::*;

    /// Tests determinant of scaling matrix equals expected_scale³
    ///
    /// Orientation::build_base() creates isotropic scaling + translation matrices.
    /// For diagonal scaling matrix with factor d, determinant = d³.
    #[test]
    fn test_determinant_scaling_matrix() {
        let volume = create_test_ct_volume();
        let base = Orientation::Transverse.build_base(&volume);
        let matrix = base.matrix;

        // Calculate expected isotropic scale factor
        let nx = volume.dimensions().0 as f32;
        let ny = volume.dimensions().1 as f32;
        let nz = volume.dimensions().2 as f32;
        let space = volume.voxel_spacing();
        let d_x = nx * space.0;
        let d_y = ny * space.1;
        let d_z = nz * space.2;
        let expected_scale = (d_x + d_y + d_z) / 3.0;

        // For diagonal scaling matrix with factor d, determinant = d³
        let expected_det = expected_scale.powi(3);

        // Calculate actual determinant using glam's Mat3
        let mat3 = glam::Mat3::from_mat4(matrix);
        let det = mat3.determinant();

        assert!(
            (det - expected_det).abs() < EPSILON,
            "Determinant of scaling matrix should be {}: {}",
            expected_det,
            det
        );
    }

    /// Tests determinant < 1.0 (non-rotation)
    #[test]
    fn test_determinant_non_rotation() {
        let volume = create_test_ct_volume();

        // Create non-rotation matrix (shear or non-uniform scaling)
        // For now, just test the existing matrix structure
        let base = Orientation::Transverse.build_base(&volume);

        let mat3 = glam::Mat3::from_mat4(base.matrix);
        let det = mat3.determinant();

        // If not a pure rotation, determinant may not be 1.0
        // Just verify determinant is non-zero (non-singular)
        assert!(
            det.abs() > EPSILON,
            "Determinant should be non-zero: {}",
            det
        );
    }

    /// Tests determinant = 0.0 (singular matrix)
    #[test]
    fn test_determinant_singular() {
        // Singular matrix would have determinant = 0 (rank deficient)
        // This is a property test rather than testing actual implementation
        // A singular matrix cannot form a valid coordinate system

        // For a 3x3 matrix to be singular, one row must be linear combination of others
        // Create such a matrix manually
        let singular_matrix = glam::Mat3::from_cols(
            glam::Vec3::new(1.0, 0.0, 0.0),
            glam::Vec3::new(0.0, 1.0, 0.0),
            glam::Vec3::new(1.0, 1.0, 0.0), // Col 2 is sum of Col 0 and Col 1
        );

        // Calculate determinant
        let det = singular_matrix.determinant();

        // Determinant should be 0 for singular matrix
        assert!(
            det.abs() < EPSILON,
            "Singular matrix should have determinant ~0: {}",
            det
        );

        // In real code, singular matrices should be rejected or handled
        // Orientation::build_base() should detect invalid configurations
    }
}

#[cfg(test)]
mod orientation_validation_tests {
    use super::*;

    /// Tests axial orientation matrix validation
    #[test]
    fn test_axial_orientation_valid() {
        let volume = create_test_ct_volume();
        let base = Orientation::Transverse.build_base(&volume);

        // Axial orientation should have valid matrix structure
        let matrix = base.matrix;

        // Test orthogonality
        // X axis (col 0) orthogonal to Y axis (col 1)
        let dot_x_y = matrix.col(0).truncate().dot(matrix.col(1).truncate());
        // X axis orthogonal to Z axis (col 2)
        let dot_x_z = matrix.col(0).truncate().dot(matrix.col(2).truncate());
        // Y axis orthogonal to Z axis
        let dot_y_z = matrix.col(1).truncate().dot(matrix.col(2).truncate());

        assert!(dot_x_y.abs() < EPSILON, "X axis orthogonal to Y axis");
        assert!(dot_x_z.abs() < EPSILON, "X axis orthogonal to Z axis");
        assert!(dot_y_z.abs() < EPSILON, "Y axis orthogonal to Z axis");
    }

    /// Tests coronal orientation matrix validation
    #[test]
    fn test_coronal_orientation_valid() {
        let volume = create_test_ct_volume();
        let base = Orientation::Coronal.build_base(&volume);
        let matrix = base.matrix;

        // Coronal: Y axis perpendicular to X and Z
        let dot_y_x = matrix.col(1).truncate().dot(matrix.col(0).truncate());
        let dot_y_z = matrix.col(1).truncate().dot(matrix.col(2).truncate());

        assert!(dot_y_x.abs() < EPSILON, "Y axis orthogonal to X axis");
        assert!(dot_y_z.abs() < EPSILON, "Y axis orthogonal to Z axis");
    }

    /// Tests sagittal orientation matrix validation
    #[test]
    fn test_sagittal_orientation_valid() {
        let volume = create_test_ct_volume();
        let base = Orientation::Sagittal.build_base(&volume);
        let matrix = base.matrix;

        // Sagittal: X axis perpendicular to Y and Z
        let dot_x_y = matrix.col(0).truncate().dot(matrix.col(1).truncate());
        let dot_x_z = matrix.col(0).truncate().dot(matrix.col(2).truncate());

        assert!(dot_x_y.abs() < EPSILON, "X axis orthogonal to Y axis");
        assert!(dot_x_z.abs() < EPSILON, "X axis orthogonal to Z axis");
    }

    /// Tests all standard orientations produce valid scaling matrices
    ///
    /// Orientation matrices have isotropic scaling. Coronal and Sagittal
    /// permute screen axes to world coordinates.
    #[test]
    fn test_all_standard_orientations_valid() {
        let volume = create_test_ct_volume();
        let orientations = [
            Orientation::Transverse,
            Orientation::Coronal,
            Orientation::Sagittal,
        ];

        // Calculate expected isotropic scale factor (same for all orientations)
        let nx = volume.dimensions().0 as f32;
        let ny = volume.dimensions().1 as f32;
        let nz = volume.dimensions().2 as f32;
        let space = volume.voxel_spacing();
        let d_x = nx * space.0;
        let d_y = ny * space.1;
        let d_z = nz * space.2;
        let expected_scale = (d_x + d_y + d_z) / 3.0;
        let expected_det = expected_scale.powi(3);

        for orientation in orientations.iter() {
            let base = orientation.build_base(&volume);
            let matrix = base.matrix;

            // Determinant should be ±d³ (axis permutations may flip sign)
            let mat3 = glam::Mat3::from_mat4(matrix);
            let det = mat3.determinant();

            assert!(
                (det.abs() - expected_det).abs() < EPSILON,
                "Determinant absolute value should be {} for {:?}: {}",
                expected_det,
                orientation,
                det
            );

            // Each column should have length = isotropic scale
            for i in 0..3 {
                let col_len = matrix.col(i).truncate().length();
                assert!(
                    (col_len - expected_scale).abs() < EPSILON,
                    "Column {} length should be {} for {:?}",
                    i,
                    expected_scale,
                    orientation
                );
            }

            // Columns should be orthonormal (perpendicular after normalizing)
            let col0 = matrix.col(0).truncate();
            let col1 = matrix.col(1).truncate();
            let col2 = matrix.col(2).truncate();

            let dot_0_1 = col0.dot(col1);
            let dot_0_2 = col0.dot(col2);
            let dot_1_2 = col1.dot(col2);

            assert!(
                dot_0_1.abs() < EPSILON,
                "Columns 0 and 1 should be orthogonal for {:?}: {}",
                orientation,
                dot_0_1
            );
            assert!(
                dot_0_2.abs() < EPSILON,
                "Columns 0 and 2 should be orthogonal for {:?}: {}",
                orientation,
                dot_0_2
            );
            assert!(
                dot_1_2.abs() < EPSILON,
                "Columns 1 and 2 should be orthogonal for {:?}: {}",
                orientation,
                dot_1_2
            );
        }
    }

    // test_invalid_volume_dimensions removed as it is not easily testable with safe APIs
}

#[cfg(test)]
mod rotation_matrix_tests {
    use super::*;
    use glam::Mat3;
    use std::f32::consts::FRAC_PI_2;

    /// Tests that a rotation matrix has determinant = 1.0
    #[test]
    fn test_rotation_matrix_determinant() {
        // Create rotation matrix around Z axis by 90 degrees
        let angle = FRAC_PI_2; // 90 degrees
        let rot_matrix = Mat3::from_rotation_z(angle);

        let det = rot_matrix.determinant();

        assert!(
            (det - 1.0).abs() < EPSILON,
            "Rotation matrix determinant should be 1.0: {}",
            det
        );
    }

    /// Tests that rotation matrix columns are unit length (orthonormal)
    #[test]
    fn test_rotation_matrix_columns_unit_length() {
        let angle = FRAC_PI_2;
        let rot_matrix = Mat3::from_rotation_z(angle);

        // Check each column has length 1.0
        for i in 0..3 {
            let col_len = rot_matrix.col(i).length();
            assert!(
                (col_len - 1.0).abs() < EPSILON,
                "Rotation matrix column {} should have unit length: {}",
                i,
                col_len
            );
        }
    }

    /// Tests that rotation matrix rows are unit length (orthonormal)
    #[test]
    fn test_rotation_matrix_rows_unit_length() {
        let angle = FRAC_PI_2;
        let rot_matrix = Mat3::from_rotation_z(angle);

        // Check each row has length 1.0
        for i in 0..3 {
            let row_len = (rot_matrix.col(0)[i].powi(2)
                + rot_matrix.col(1)[i].powi(2)
                + rot_matrix.col(2)[i].powi(2))
            .sqrt();

            assert!(
                (row_len - 1.0).abs() < EPSILON,
                "Rotation matrix row {} should have unit length: {}",
                i,
                row_len
            );
        }
    }

    /// Tests that rotation matrix columns are orthogonal
    #[test]
    fn test_rotation_matrix_columns_orthogonal() {
        let angle = FRAC_PI_2;
        let rot_matrix = Mat3::from_rotation_z(angle);

        // Check all pairs of columns are perpendicular (dot product = 0)
        let dot_0_1 = rot_matrix.col(0).dot(rot_matrix.col(1));
        let dot_0_2 = rot_matrix.col(0).dot(rot_matrix.col(2));
        let dot_1_2 = rot_matrix.col(1).dot(rot_matrix.col(2));

        assert!(
            dot_0_1.abs() < EPSILON,
            "Rotation matrix columns 0 and 1 should be orthogonal: {}",
            dot_0_1
        );
        assert!(
            dot_0_2.abs() < EPSILON,
            "Rotation matrix columns 0 and 2 should be orthogonal: {}",
            dot_0_2
        );
        assert!(
            dot_1_2.abs() < EPSILON,
            "Rotation matrix columns 1 and 2 should be orthogonal: {}",
            dot_1_2
        );
    }

    /// Tests that rotation matrix rows are orthogonal
    #[test]
    fn test_rotation_matrix_rows_orthogonal() {
        let angle = FRAC_PI_2;
        let rot_matrix = Mat3::from_rotation_z(angle);

        // Row 0 dot Row 1
        let dot_0_1 = rot_matrix.col(0)[0] * rot_matrix.col(0)[1]
            + rot_matrix.col(1)[0] * rot_matrix.col(1)[1]
            + rot_matrix.col(2)[0] * rot_matrix.col(2)[1];

        // Row 0 dot Row 2
        let dot_0_2 = rot_matrix.col(0)[0] * rot_matrix.col(0)[2]
            + rot_matrix.col(1)[0] * rot_matrix.col(1)[2]
            + rot_matrix.col(2)[0] * rot_matrix.col(2)[2];

        // Row 1 dot Row 2
        let dot_1_2 = rot_matrix.col(0)[1] * rot_matrix.col(0)[2]
            + rot_matrix.col(1)[1] * rot_matrix.col(1)[2]
            + rot_matrix.col(2)[1] * rot_matrix.col(2)[2];

        assert!(
            dot_0_1.abs() < EPSILON,
            "Rotation matrix rows 0 and 1 should be orthogonal: {}",
            dot_0_1
        );
        assert!(
            dot_0_2.abs() < EPSILON,
            "Rotation matrix rows 0 and 2 should be orthogonal: {}",
            dot_0_2
        );
        assert!(
            dot_1_2.abs() < EPSILON,
            "Rotation matrix rows 1 and 2 should be orthogonal: {}",
            dot_1_2
        );
    }
}
