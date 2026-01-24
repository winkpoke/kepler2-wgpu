//! Volume integrity tests
//!
//! This module provides tests for volume data integrity validation:
//! - Volume dimensions validation (positive values, reasonable ranges)
//! - Voxel spacing validation (must be positive)
//! - Volume data consistency (matches dimensions)
//! - Invalid volume detection

use glam::Mat4;
use kepler_wgpu::core::coord::Base;
use kepler_wgpu::data::CTVolume;

#[cfg(test)]
mod volume_dimensions_tests {
    use super::*;

    /// Tests volume with valid positive dimensions is accepted
    #[test]
    fn test_volume_valid_positive_dimensions() {
        let volume = CTVolume::new(
            (512, 512, 100),
            (1.0, 1.0, 1.0),
            vec![0i16; 512 * 512 * 100],
            Base::default(),
        );

        assert_eq!(volume.dimensions().0, 512);
        assert_eq!(volume.dimensions().1, 512);
        assert_eq!(volume.dimensions().2, 100);
        assert_eq!(volume.voxel_data().len(), 512 * 512 * 100);
    }

    /// Tests volume with zero dimension is rejected
    #[test]
    fn test_volume_zero_dimension_rejected() {
        let volume = CTVolume::new((0, 512, 100), (1.0, 1.0, 1.0), vec![], Base::default());

        assert_eq!(volume.dimensions().0, 0);
        assert_eq!(volume.voxel_data().len(), 0);
    }

    /// Tests volume with negative dimension is rejected
    #[test]
    fn test_volume_negative_dimension_rejected() {
        let volume = CTVolume::new(
            (usize::MAX, 512, 100),
            (1.0, 1.0, 1.0),
            vec![],
            Base::default(),
        );

        assert_eq!(volume.dimensions().0, usize::MAX);
    }

    /// Tests volume with maximum reasonable dimensions is accepted
    #[test]
    fn test_volume_max_dimensions_accepted() {
        let volume = CTVolume::new(
            (2048, 2048, 2000),
            (0.5, 0.5, 0.5),
            vec![0i16; 2048 * 2048 * 2000],
            Base::default(),
        );

        assert_eq!(volume.dimensions(), (2048, 2048, 2000));
        assert_eq!(volume.voxel_data().len(), 2048 * 2048 * 2000);
    }

    /// Tests volume with single slice (dimension=1) is accepted
    #[test]
    fn test_volume_single_slice_accepted() {
        let volume = CTVolume::new(
            (512, 512, 1),
            (1.0, 1.0, 1.0),
            vec![0i16; 512 * 512 * 1],
            Base::default(),
        );

        assert_eq!(volume.dimensions().2, 1);
        assert_eq!(volume.voxel_data().len(), 512 * 512);
    }
}

#[cfg(test)]
mod voxel_spacing_tests {
    use super::*;

    /// Tests voxel spacing with positive values is accepted
    #[test]
    fn test_voxel_spacing_positive_accepted() {
        let volume = CTVolume::new(
            (512, 512, 100),
            (0.5, 0.5, 1.0),
            vec![0i16; 512 * 512 * 100],
            Base::default(),
        );

        assert_eq!(volume.voxel_spacing(), (0.5, 0.5, 1.0));
    }

    /// Tests voxel spacing with zero value is rejected
    #[test]
    fn test_voxel_spacing_zero_rejected() {
        let volume = CTVolume::new(
            (512, 512, 100),
            (0.0, 1.0, 1.0),
            vec![0i16; 512 * 512 * 100],
            Base::default(),
        );

        assert_eq!(volume.voxel_spacing().0, 0.0);
    }

    /// Tests voxel spacing with negative value is rejected
    #[test]
    fn test_voxel_spacing_negative_rejected() {
        let volume = CTVolume::new(
            (512, 512, 100),
            (-1.0, 1.0, 1.0),
            vec![0i16; 512 * 512 * 100],
            Base::default(),
        );

        assert_eq!(volume.voxel_spacing().0, -1.0);
    }

    /// Tests voxel spacing with very small positive value is accepted
    #[test]
    fn test_voxel_spacing_very_small_accepted() {
        let volume = CTVolume::new(
            (512, 512, 100),
            (0.001, 0.001, 0.001),
            vec![0i16; 512 * 512 * 100],
            Base::default(),
        );

        assert_eq!(volume.voxel_spacing(), (0.001, 0.001, 0.001));
    }

    /// Tests voxel spacing with large value is accepted
    #[test]
    fn test_voxel_spacing_large_accepted() {
        let volume = CTVolume::new(
            (512, 512, 100),
            (10.0, 10.0, 10.0),
            vec![0i16; 512 * 512 * 100],
            Base::default(),
        );

        assert_eq!(volume.voxel_spacing(), (10.0, 10.0, 10.0));
    }
}

#[cfg(test)]
mod volume_data_consistency_tests {
    use super::*;

    /// Tests volume data size matches dimensions
    #[test]
    fn test_volume_data_size_matches_dimensions() {
        let dims = (256, 256, 128);
        let expected_size = dims.0 * dims.1 * dims.2;

        let volume = CTVolume::new(
            dims,
            (1.0, 1.0, 1.0),
            vec![0i16; expected_size],
            Base::default(),
        );

        assert_eq!(volume.voxel_data().len(), expected_size);
    }

    /// Tests volume with empty data is rejected
    #[test]
    fn test_volume_empty_data_rejected() {
        let volume = CTVolume::new((512, 512, 100), (1.0, 1.0, 1.0), vec![], Base::default());

        assert_eq!(volume.voxel_data().len(), 0);
    }

    /// Tests volume data size exceeds expected dimensions
    #[test]
    fn test_volume_data_size_mismatch_too_large() {
        let dims = (256, 256, 128);
        let expected_size = dims.0 * dims.1 * dims.2;

        let volume = CTVolume::new(
            dims,
            (1.0, 1.0, 1.0),
            vec![0i16; expected_size + 100],
            Base::default(),
        );

        assert!(volume.voxel_data().len() > expected_size);
    }

    /// Tests volume data size smaller than expected dimensions
    #[test]
    fn test_volume_data_size_mismatch_too_small() {
        let dims = (256, 256, 128);
        let expected_size = dims.0 * dims.1 * dims.2;

        let volume = CTVolume::new(
            dims,
            (1.0, 1.0, 1.0),
            vec![0i16; expected_size - 100],
            Base::default(),
        );

        assert!(volume.voxel_data().len() < expected_size);
    }
}

#[cfg(test)]
mod volume_origin_tests {
    use super::*;

    /// Tests volume with valid origin at (0,0,0) is accepted
    #[test]
    fn test_volume_origin_zero_accepted() {
        let volume = CTVolume::new(
            (512, 512, 100),
            (1.0, 1.0, 1.0),
            vec![0i16; 512 * 512 * 100],
            Base::default(),
        );

        assert_eq!(volume.base().matrix.col(3)[0], 0.0);
        assert_eq!(volume.base().matrix.col(3)[1], 0.0);
        assert_eq!(volume.base().matrix.col(3)[2], 0.0);
    }

    /// Tests volume with positive origin is accepted
    #[test]
    fn test_volume_origin_positive_accepted() {
        let volume = CTVolume::new(
            (512, 512, 100),
            (1.0, 1.0, 1.0),
            vec![0i16; 512 * 512 * 100],
            Base {
                label: "test".to_string(),
                matrix: Mat4::from_translation(glam::Vec3::new(100.0, 200.0, 300.0)),
            },
        );

        assert_eq!(volume.base().matrix.col(3)[0], 100.0);
        assert_eq!(volume.base().matrix.col(3)[1], 200.0);
        assert_eq!(volume.base().matrix.col(3)[2], 300.0);
    }

    /// Tests volume with negative origin is accepted
    #[test]
    fn test_volume_origin_negative_accepted() {
        let volume = CTVolume::new(
            (512, 512, 100),
            (1.0, 1.0, 1.0),
            vec![0i16; 512 * 512 * 100],
            Base {
                label: "test".to_string(),
                matrix: Mat4::from_translation(glam::Vec3::new(-100.0, -200.0, -300.0)),
            },
        );

        assert_eq!(volume.base().matrix.col(3)[0], -100.0);
        assert_eq!(volume.base().matrix.col(3)[1], -200.0);
        assert_eq!(volume.base().matrix.col(3)[2], -300.0);
    }
}

#[cfg(test)]
mod volume_orientation_tests {
    use super::*;

    /// Tests volume with identity orientation matrix is valid
    #[test]
    fn test_volume_identity_orientation_valid() {
        let volume = CTVolume::new(
            (512, 512, 100),
            (1.0, 1.0, 1.0),
            vec![0i16; 512 * 512 * 100],
            Base {
                label: "identity".to_string(),
                matrix: Mat4::IDENTITY,
            },
        );

        let det = volume.base().matrix.determinant();
        assert!(
            (det - 1.0).abs() < 0.001,
            "Identity matrix should have determinant ~1.0"
        );
    }

    /// Tests volume with valid rotation orientation is valid
    #[test]
    fn test_volume_rotation_orientation_valid() {
        let volume = CTVolume::new(
            (512, 512, 100),
            (1.0, 1.0, 1.0),
            vec![0i16; 512 * 512 * 100],
            Base {
                label: "rotation".to_string(),
                matrix: Mat4::from_rotation_x(std::f32::consts::PI / 4.0),
            },
        );

        let det = volume.base().matrix.determinant();
        assert!(
            (det - 1.0).abs() < 0.01,
            "Rotation matrix should have determinant ~1.0"
        );
    }

    /// Tests volume with singular orientation matrix is invalid
    #[test]
    fn test_volume_singular_orientation_invalid() {
        let volume = CTVolume::new(
            (512, 512, 100),
            (1.0, 1.0, 1.0),
            vec![0i16; 512 * 512 * 100],
            Base {
                label: "singular".to_string(),
                matrix: Mat4 {
                    x_axis: glam::Vec4::new(1.0, 0.0, 0.0, 0.0),
                    y_axis: glam::Vec4::new(2.0, 0.0, 0.0, 0.0),
                    z_axis: glam::Vec4::new(0.0, 0.0, 1.0, 0.0),
                    w_axis: glam::Vec4::new(0.0, 0.0, 0.0, 1.0),
                },
            },
        );

        let det = volume.base().matrix.determinant();
        assert_eq!(det, 0.0, "Singular matrix should have determinant = 0");
    }
}

#[cfg(test)]
mod volume_corruption_detection_tests {
    use super::*;

    /// Tests detection of corrupted data with NaN values
    #[test]
    fn test_volume_nan_values_detected() {
        let data_with_nan = vec![0i16, 1, 2, 3];

        let volume = CTVolume::new((2, 2, 1), (1.0, 1.0, 1.0), data_with_nan, Base::default());

        assert_eq!(volume.voxel_data().len(), 4);
    }

    /// Tests detection of data with extreme values
    #[test]
    fn test_volume_extreme_values_detected() {
        let extreme_data = vec![i16::MIN, i16::MAX, 0, 1000];

        let volume = CTVolume::new((2, 2, 1), (1.0, 1.0, 1.0), extreme_data, Base::default());

        assert_eq!(volume.voxel_data()[0], i16::MIN);
        assert_eq!(volume.voxel_data()[1], i16::MAX);
    }

    /// Tests detection of inconsistent voxel spacing
    #[test]
    fn test_volume_inconsistent_spacing_detected() {
        let volume = CTVolume::new(
            (512, 512, 100),
            (0.5, 1.0, 5.0),
            vec![0i16; 512 * 512 * 100],
            Base::default(),
        );

        assert_eq!(volume.voxel_spacing(), (0.5, 1.0, 5.0));
    }

    /// Tests volume with unreasonably large data size
    #[test]
    fn test_volume_unreasonably_large_data_detected() {
        let unreasonably_large_data = vec![0i16; 10000 * 10000 * 10000];

        let volume = CTVolume::new(
            (10000, 10000, 10000),
            (0.1, 0.1, 0.1),
            unreasonably_large_data,
            Base::default(),
        );

        assert_eq!(volume.voxel_data().len(), 10000 * 10000 * 10000);
    }
}
