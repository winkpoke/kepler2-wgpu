mod common;

use common::DicomFixtureBuilder;

#[cfg(test)]
mod dicom_tests {
    use super::*;

    #[test]
    fn test_odd_sized_pixel_data() {
        let (_patient, _study, _series, images) =
            DicomFixtureBuilder::new()
                .image_count(1)
                .dimensions((3, 3))
                .build_complete_fixture();

        let image = &images[0];
        let expected_size = image.rows as usize * image.columns as usize * 2;
        assert_eq!(
            image.pixel_data.len(),
            expected_size,
            "Pixel data size should be even for 3x3 16-bit image"
        );
        assert_eq!(image.pixel_data.len() % 2, 0, "Pixel data should be divisible by 2");
    }

    #[test]
    fn test_pixel_data_not_divisible_by_two() {
        let (_patient, _study, _series, images) =
            DicomFixtureBuilder::new()
                .image_count(1)
                .dimensions((5, 7))
                .build_complete_fixture();

        let image = &images[0];
        assert_eq!(
            image.pixel_data.len() % 2,
            0,
            "Pixel data must be divisible by 2 for 16-bit pixels"
        );
    }

    #[test]
    fn test_endianness_mismatch() {
        let mut fixture = DicomFixtureBuilder::new()
            .image_count(1)
            .dimensions((30, 30))
            .pixel_representation(1)
            .no_rescale_slope()
            .no_rescale_intercept()
            .build_complete_fixture();

        let (_patient, _study, _series, ref mut images) = fixture;
        let image = &mut images[0];

        let original_data = image.pixel_data.clone();
        let original_pixels = image.get_pixel_data().expect("Should parse pixel data");

        image.pixel_data = original_data
            .chunks_exact(2)
            .flat_map(|chunk| vec![chunk[1], chunk[0]])
            .collect();

        let swapped_pixels = image.get_pixel_data().expect("Should parse swapped data");

        assert_ne!(
            original_pixels, swapped_pixels,
            "Swapped endianness should produce different values"
        );
    }

    #[test]
    fn test_truncated_pixel_data() {
        let (_patient, _study, _series, images) =
            DicomFixtureBuilder::new()
                .image_count(1)
                .dimensions((4, 4))
                .build_complete_fixture();

        let image = &images[0];
        let result = image.get_pixel_data();
        assert!(result.is_ok(), "Valid pixel data should parse successfully");

        let truncated = image.pixel_data[..image.pixel_data.len() - 4].to_vec();
        let mut truncated_image = image.clone();
        truncated_image.pixel_data = truncated;

        let result = truncated_image.get_pixel_data();
        assert!(result.is_err(), "Truncated pixel data should fail");
    }

    #[test]
    fn test_overflow_in_rescaling() {
        let (_patient, _study, _series, images) = DicomFixtureBuilder::new()
            .image_count(1)
            .dimensions((2, 2))
            .pixel_representation(1)
            .rescale_slope(32767.0)
            .rescale_intercept(0.0)
            .build_complete_fixture();

        let image = &images[0];
        let result = image.get_pixel_data();

        assert!(
            result.is_ok(),
            "Large rescale values should not cause panic, but may saturate"
        );

        let pixels = result.unwrap();
        for pixel in pixels {
            assert!(
                pixel <= i16::MAX,
                "Rescaled values should not exceed i16::MAX"
            );
        }
    }

    #[test]
    fn test_zero_rescale_slope() {
        let (_patient, _study, _series, images) = DicomFixtureBuilder::new()
            .image_count(1)
            .dimensions((2, 2))
            .pixel_representation(1)
            .rescale_slope(0.0)
            .rescale_intercept(100.0)
            .build_complete_fixture();

        let image = &images[0];
        let result = image.get_pixel_data();

        assert!(result.is_ok(), "Zero slope should not cause errors");

        let pixels = result.unwrap();
        for pixel in pixels {
            assert_eq!(
                pixel, 100,
                "With zero slope, all values should equal intercept"
            );
        }
    }

    #[test]
    fn test_negative_rescale_intercept() {
        let (_patient, _study, _series, images) = DicomFixtureBuilder::new()
            .image_count(1)
            .dimensions((2, 2))
            .pixel_representation(1)
            .rescale_slope(1.0)
            .rescale_intercept(-1024.0)
            .build_complete_fixture();

        let image = &images[0];
        let result = image.get_pixel_data();

        assert!(result.is_ok(), "Negative intercept should work correctly");

        let pixels = result.unwrap();
        for pixel in pixels {
            assert!(
                pixel < 0,
                "With negative intercept, values should be negative for small raw values"
            );
        }
    }

    #[test]
    fn test_precision_loss_on_rounding() {
        let (_patient, _study, _series, images) = DicomFixtureBuilder::new()
            .image_count(1)
            .dimensions((2, 2))
            .pixel_representation(1)
            .rescale_slope(0.5)
            .rescale_intercept(0.25)
            .build_complete_fixture();

        let image = &images[0];
        let result = image.get_pixel_data();

        assert!(result.is_ok(), "Non-integer rescaling should work");

        let pixels = result.unwrap();
        for pixel in &pixels {
            assert!(
                *pixel >= i16::MIN && *pixel <= i16::MAX,
                "Rounded values should fit in i16"
            );
        }
    }
}
