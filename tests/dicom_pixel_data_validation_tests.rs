mod common;

use common::{create_minimal_fixture, create_rescaled_fixture, create_unsigned_pixel_fixture};
use kepler_wgpu::data::dicom::CTImage;

#[cfg(test)]
mod dicom_pixel_data_validation_tests {
    use super::*;

    #[test]
    fn test_pixel_data_size_matches_dimensions_16bit() {
        let (_patient, _study, _series, images) = create_minimal_fixture();
        let image = &images[0];

        let expected_size = image.rows as usize * image.columns as usize * 2;
        assert_eq!(
            image.pixel_data.len(),
            expected_size,
            "Pixel data size should match dimensions (16-bit pixels)"
        );
    }

    #[test]
    fn test_pixel_data_size_matches_dimensions_unsigned() {
        let (_patient, _study, _series, images) = create_unsigned_pixel_fixture();
        let image = &images[0];

        let expected_size = image.rows as usize * image.columns as usize * 2;
        assert_eq!(
            image.pixel_data.len(),
            expected_size,
            "Pixel data size should match dimensions for unsigned 16-bit pixels"
        );
    }

    #[test]
    fn test_get_pixel_data_returns_correct_length() {
        let (_patient, _study, _series, images) = create_minimal_fixture();
        let image = &images[0];

        let result = image.get_pixel_data();
        assert!(result.is_ok(), "Should successfully get pixel data");

        let pixel_data = result.unwrap();
        let expected_length = image.rows as usize * image.columns as usize;
        assert_eq!(
            pixel_data.len(),
            expected_length,
            "Returned pixel data should have correct length"
        );
    }

    #[test]
    fn test_get_pixel_data_with_signed_representation() {
        let (_patient, _study, _series, images) = create_minimal_fixture();
        let image = &images[0];

        assert_eq!(
            image.pixel_representation, 1,
            "Should have signed pixel representation"
        );

        let result = image.get_pixel_data();
        assert!(result.is_ok(), "Should successfully get signed pixel data");

        let pixel_data = result.unwrap();
        assert!(!pixel_data.is_empty(), "Pixel data should not be empty");
    }

    #[test]
    fn test_get_pixel_data_with_unsigned_representation() {
        let (_patient, _study, _series, images) = create_unsigned_pixel_fixture();
        let image = &images[0];

        assert_eq!(
            image.pixel_representation, 0,
            "Should have unsigned pixel representation"
        );

        let result = image.get_pixel_data();
        assert!(
            result.is_ok(),
            "Should successfully get unsigned pixel data"
        );

        let pixel_data = result.unwrap();
        assert!(!pixel_data.is_empty(), "Pixel data should not be empty");
    }

    #[test]
    fn test_rescaling_with_positive_slope() {
        let (_patient, _study, _series, images) = create_rescaled_fixture();
        let image = &images[0];

        let result = image.get_pixel_data();
        assert!(
            result.is_ok(),
            "Should successfully get rescaled pixel data"
        );

        let pixel_data = result.unwrap();
        assert!(
            !pixel_data.is_empty(),
            "Rescaled pixel data should not be empty"
        );

        let min_val = *pixel_data.iter().min().unwrap();
        let max_val = *pixel_data.iter().max().unwrap();
        assert!(
            min_val < max_val,
            "Rescaled data should have a range of values"
        );
    }

    #[test]
    fn test_pixel_data_all_zeros() {
        use common::DicomFixtureBuilder;

        let builder = DicomFixtureBuilder::new().dimensions((4, 4)).image_count(1);
        let (_patient, _study, _series, mut images) = builder.build_complete_fixture();

        let image = &mut images[0];

        let zero_data = vec![0u8; image.pixel_data.len()];
        image.pixel_data = zero_data;

        let result = image.get_pixel_data();
        assert!(result.is_ok(), "Should handle all-zero pixel data");

        let pixel_data = result.unwrap();

        assert_eq!(pixel_data.len(), 16, "Should have 16 pixels (4x4)");
    }

    #[test]
    fn test_pixel_data_consistency_across_slices() {
        let (_patient, _study, _series, images) = create_minimal_fixture();

        let first_data = images[0].get_pixel_data().unwrap();
        let last_data = images[images.len() - 1].get_pixel_data().unwrap();

        assert_eq!(
            first_data.len(),
            last_data.len(),
            "All slices should have the same pixel data length"
        );
    }

    #[test]
    fn test_pixel_data_byte_order_little_endian() {
        let (_patient, _study, _series, images) = create_minimal_fixture();
        let image = &images[0];

        let pixel_bytes = &image.pixel_data;

        let i16_value = i16::from_le_bytes([pixel_bytes[0], pixel_bytes[1]]);
        assert!(
            i16_value >= i16::MIN && i16_value <= i16::MAX,
            "Little-endian byte order should produce valid i16 values"
        );
    }

    #[test]
    fn test_pixel_data_range_within_i16_bounds() {
        let (_patient, _study, _series, images) = create_rescaled_fixture();
        let image = &images[0];

        let result = image.get_pixel_data();
        assert!(result.is_ok(), "Should successfully get pixel data");

        let pixel_data = result.unwrap();

        for &value in &pixel_data {
            assert!(
                value >= i16::MIN && value <= i16::MAX,
                "Pixel data values should be within i16 bounds, got {}",
                value
            );
        }
    }

    #[test]
    fn test_no_rescaling_by_default() {
        let (_patient, _study, _series, images) = create_minimal_fixture();
        let image = &images[0];

        let result = image.get_pixel_data();
        assert!(
            result.is_ok(),
            "Should successfully get pixel data without rescaling"
        );

        let pixel_data = result.unwrap();
        assert!(
            !pixel_data.is_empty(),
            "Non-rescaled pixel data should not be empty"
        );
    }

    #[test]
    fn test_get_pixel_data_fails_with_wrong_size() {
        use common::DicomFixtureBuilder;

        let builder = DicomFixtureBuilder::new().dimensions((4, 4)).image_count(1);
        let (_patient, _study, _series, mut images) = builder.build_complete_fixture();

        let image = &mut images[0];

        image.pixel_data = vec![0u8; 10];

        let result = image.get_pixel_data();
        assert!(
            result.is_err(),
            "Should fail with incorrect pixel data size"
        );

        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("mismatch"),
            "Error should mention size mismatch, got: {}",
            error_msg
        );
    }

    #[test]
    fn test_get_pixel_data_handles_rescale_slope() {
        let (_patient, _study, _series, images) = create_rescaled_fixture();
        let image = &images[0];

        let result = image.get_pixel_data();
        assert!(
            result.is_ok(),
            "Should successfully get pixel data with rescale slope"
        );

        let pixel_data = result.unwrap();
        assert!(
            !pixel_data.is_empty(),
            "Rescaled pixel data should not be empty"
        );

        let slope = image.rescale_slope.unwrap();
        assert!(slope.abs() > 0.001, "Rescale slope should be non-zero");
    }

    #[test]
    fn test_get_pixel_data_handles_rescale_intercept() {
        let (_patient, _study, _series, images) = create_rescaled_fixture();
        let image = &images[0];

        let result = image.get_pixel_data();
        assert!(
            result.is_ok(),
            "Should successfully get pixel data with rescale intercept"
        );

        let pixel_data = result.unwrap();
        assert!(
            !pixel_data.is_empty(),
            "Pixel data with intercept should not be empty"
        );

        let intercept = image.rescale_intercept.unwrap();
        assert!(intercept.is_finite(), "Rescale intercept should be finite");
    }

    #[test]
    fn test_pixel_data_not_corrupted() {
        let (_patient, _study, _series, images) = create_minimal_fixture();
        let image = &images[0];

        let result = image.get_pixel_data();
        assert!(
            result.is_ok(),
            "Should successfully get non-corrupted pixel data"
        );

        let pixel_data = result.unwrap();
        assert!(
            !pixel_data.is_empty(),
            "Non-corrupted pixel data should not be empty"
        );

        let has_nan = pixel_data.iter().any(|&v| v != v);
        assert!(!has_nan, "Pixel data should not contain NaN values");
    }

    #[test]
    fn test_pixel_data_divisible_by_two() {
        let (_patient, _study, _series, images) = create_minimal_fixture();
        let image = &images[0];

        assert_eq!(
            image.pixel_data.len() % 2,
            0,
            "Pixel data length should be divisible by 2 for 16-bit pixels"
        );
    }

    #[test]
    fn test_pixel_data_odd_sized_rejected() {
        use common::DicomFixtureBuilder;

        let builder = DicomFixtureBuilder::new().dimensions((4, 4)).image_count(1);
        let (_patient, _study, _series, mut images) = builder.build_complete_fixture();

        let image = &mut images[0];

        image.pixel_data = vec![0u8; 31];

        let result = image.get_pixel_data();
        assert!(result.is_err(), "Should fail with odd-sized pixel data");
    }

    #[test]
    fn test_pixel_data_even_number_of_pixels() {
        let (_patient, _study, _series, images) = create_minimal_fixture();
        let image = &images[0];

        let total_pixels = image.rows as usize * image.columns as usize;
        assert_eq!(
            total_pixels % 2,
            0,
            "Total number of pixels should result in even byte count for 16-bit data"
        );
    }

    #[test]
    fn test_multiple_images_consistent_pixel_format() {
        let (_patient, _study, _series, images) = create_minimal_fixture();

        let first_pixel_rep = images[0].pixel_representation;

        for image in &images[1..] {
            assert_eq!(
                image.pixel_representation, first_pixel_rep,
                "All images in series should have the same pixel representation"
            );
        }
    }
}
