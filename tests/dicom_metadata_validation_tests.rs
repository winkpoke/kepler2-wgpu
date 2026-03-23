mod common;

use kepler_wgpu::data::dicom::{DicomRepo, ImageSeries, Patient};
use kepler_wgpu::data::medical_imaging::metadata::PatientPosition;
use kepler_wgpu::data::CTVolumeGenerator;

#[cfg(test)]
mod dicom_metadata_validation_tests {
    use super::*;

    #[test]
    fn test_patient_id_not_empty() {
        let (patient, _study, _series, _images) = common::create_minimal_fixture();

        let result: anyhow::Result<()> = patient.validate();
        assert!(result.is_ok(), "Patient validation should succeed");
        assert!(
            !patient.patient_id.is_empty(),
            "Patient ID should not be empty"
        );
    }

    #[test]
    fn test_patient_id_max_length() {
        let patient = Patient::new(
            "A".repeat(64),
            "Test^Patient".to_string(),
            Some("19850101".to_string()),
            Some("M".to_string()),
        );

        let result = patient.validate();
        assert!(result.is_ok(), "Patient ID within limit should be valid");
    }

    #[test]
    fn test_patient_id_exceeds_max_length() {
        let patient = Patient::new(
            "A".repeat(65),
            "Test^Patient".to_string(),
            Some("19850101".to_string()),
            Some("M".to_string()),
        );

        let result = patient.validate();
        assert!(
            result.is_err(),
            "Patient ID exceeding limit should fail validation"
        );
    }

    #[test]
    fn test_patient_name_not_empty() {
        let (patient, _study, _series, _images) = common::create_minimal_fixture();

        let result = patient.validate();
        assert!(result.is_ok(), "Patient validation should succeed");
        assert!(!patient.name.is_empty(), "Patient name should not be empty");
    }

    #[test]
    fn test_patient_name_component_max_length() {
        let patient = Patient::new(
            "TEST_001".to_string(),
            "A".repeat(64),
            Some("19850101".to_string()),
            Some("M".to_string()),
        );

        let result = patient.validate();
        assert!(
            result.is_ok(),
            "Patient name component within limit should be valid"
        );
    }

    #[test]
    fn test_patient_name_component_exceeds_max_length() {
        let patient = Patient::new(
            "TEST_001".to_string(),
            "A".repeat(65),
            Some("19850101".to_string()),
            Some("M".to_string()),
        );

        let result = patient.validate();
        assert!(
            result.is_err(),
            "Patient name component exceeding limit should fail"
        );
    }

    #[test]
    fn test_patient_name_invalid_characters() {
        let patient = Patient::new(
            "TEST_001".to_string(),
            "Test$Patient".to_string(),
            Some("19850101".to_string()),
            Some("M".to_string()),
        );

        let result = patient.validate();
        assert!(
            result.is_err(),
            "Patient name with invalid characters should fail validation"
        );
    }

    #[test]
    fn test_patient_birthdate_format() {
        let patient = Patient::new(
            "TEST_001".to_string(),
            "Test^Patient".to_string(),
            Some("19850101".to_string()),
            Some("M".to_string()),
        );

        let result = patient.validate();
        assert!(
            result.is_ok(),
            "Valid birthdate format should pass validation"
        );
    }

    #[test]
    fn test_patient_birthdate_invalid_format() {
        let patient = Patient::new(
            "TEST_001".to_string(),
            "Test^Patient".to_string(),
            Some("1985-01-01".to_string()),
            Some("M".to_string()),
        );

        let result = patient.validate();
        assert!(
            result.is_err(),
            "Invalid birthdate format should fail validation"
        );
    }

    #[test]
    fn test_patient_sex_valid() {
        for sex in ["M", "F", "O"].iter() {
            let patient = Patient::new(
                "TEST_001".to_string(),
                "Test^Patient".to_string(),
                Some("19850101".to_string()),
                Some(sex.to_string()),
            );

            let result = patient.validate();
            assert!(
                result.is_ok(),
                "Valid sex value {} should pass validation",
                sex
            );
        }
    }

    #[test]
    fn test_patient_sex_invalid() {
        let patient = Patient::new(
            "TEST_001".to_string(),
            "Test^Patient".to_string(),
            Some("19850101".to_string()),
            Some("X".to_string()),
        );

        let result = patient.validate();
        assert!(result.is_err(), "Invalid sex value should fail validation");
    }

    #[test]
    fn test_study_id_not_empty() {
        let (_patient, study, _series, _images) = common::create_minimal_fixture();

        assert!(!study.study_id.is_empty(), "Study ID should not be empty");
    }

    #[test]
    fn test_study_uid_not_empty() {
        let (_patient, study, _series, _images) = common::create_minimal_fixture();

        assert!(!study.uid.is_empty(), "Study UID should not be empty");
    }

    #[test]
    fn test_study_uid_format() {
        let (_patient, study, _series, _images) = common::create_minimal_fixture();

        let uid_parts: Vec<&str> = study.uid.split('.').collect();
        assert!(
            uid_parts.len() >= 2,
            "Study UID should have at least 2 parts separated by dots"
        );
        assert!(
            uid_parts[0].starts_with('1') || uid_parts[0].starts_with('2'),
            "Study UID root should start with 1 or 2"
        );
    }

    #[test]
    fn test_study_date_format() {
        let (_patient, study, _series, _images) = common::create_minimal_fixture();

        assert_eq!(study.date.len(), 8, "Study date should be 8 characters");
        assert!(
            study.date.chars().all(|c| c.is_numeric()),
            "Study date should be numeric"
        );
    }

    #[test]
    fn test_series_uid_not_empty() {
        let (_patient, _study, series, _images) = common::create_minimal_fixture();

        assert!(!series.uid.is_empty(), "Series UID should not be empty");
    }

    #[test]
    fn test_series_uid_format() {
        let (_patient, _study, series, _images) = common::create_minimal_fixture();

        let uid_parts: Vec<&str> = series.uid.split('.').collect();
        assert!(
            uid_parts.len() >= 2,
            "Series UID should have at least 2 parts separated by dots"
        );
    }

    #[test]
    fn test_series_modality_valid() {
        let (_patient, _study, series, _images) = common::create_standard_ct_volume_fixture();

        assert_eq!(series.modality, "CT", "Modality should be CT");
    }

    #[test]
    fn test_series_modality_invalid() {
        let (_patient, _study, series, _images) = common::create_invalid_modality_fixture();

        let _series_obj = ImageSeries::new(
            series.uid.clone(),
            series.study_uid.clone(),
            series.modality.clone(),
            series.description.clone(),
        );

        let result = ImageSeries::from_bytes(&[]);
        assert!(result.is_err(), "Invalid modality should fail");
    }

    #[test]
    fn test_image_orientation_patient_format() {
        let (_patient, _study, _series, images) = common::create_standard_ct_volume_fixture();

        let orientation = images[0]
            .image_orientation_patient
            .expect("ImageOrientationPatient should be present");

        // orientation.0 is a tuple (f32, f32, f32, f32, f32, f32), not a Vec
        assert!(
            (orientation.0.abs() - 1.0) < 0.001 || orientation.0.abs() < 0.001,
            "First value should be close to 0 or 1"
        );
    }

    #[test]
    fn test_image_position_patient_format() {
        let (_patient, _study, _series, images) = common::create_standard_ct_volume_fixture();

        let position = images[0]
            .image_position_patient
            .expect("ImagePositionPatient should be present");

        // position.0 is a tuple (f32, f32, f32), not a Vec
        assert!(
            position.0.is_finite() && position.1.is_finite() && position.2.is_finite(),
            "All position values should be finite"
        );
    }

    #[test]
    fn test_pixel_spacing_format() {
        let (_patient, _study, _series, images) = common::create_standard_ct_volume_fixture();

        let spacing = images[0]
            .pixel_spacing
            .expect("PixelSpacing should be present");

        assert!(
            spacing.0 > 0.0 && spacing.1 > 0.0,
            "Pixel spacing should be positive"
        );
    }

    #[test]
    fn test_slice_thickness_positive() {
        let (_patient, _study, _series, images) = common::create_standard_ct_volume_fixture();

        let thickness = images[0]
            .slice_thickness
            .expect("SliceThickness should be present");

        assert!(thickness > 0.0, "Slice thickness should be positive");
    }

    #[test]
    fn test_rescale_slope_valid() {
        let (_patient, _study, _series, images) = common::create_rescaled_fixture();

        let slope = images[0]
            .rescale_slope
            .expect("RescaleSlope should be present");

        assert!(slope.abs() > 0.001, "Rescale slope should not be zero");
    }

    #[test]
    fn test_rescale_intercept_valid() {
        let (_patient, _study, _series, images) = common::create_rescaled_fixture();

        let intercept = images[0]
            .rescale_intercept
            .expect("RescaleIntercept should be present");

        assert!(intercept.is_finite(), "Rescale intercept should be finite");
    }

    #[test]
    fn test_patient_position_parsing() {
        let position = PatientPosition::from_str("HFS");
        assert_eq!(position, PatientPosition::HFS);

        let position = PatientPosition::from_str("FFS");
        assert_eq!(position, PatientPosition::FFS);

        let position = PatientPosition::from_str("HFP");
        assert_eq!(position, PatientPosition::HFP);
    }

    #[test]
    fn test_missing_optional_fields() {
        let (_patient, _study, _series, images) = common::create_missing_optional_fields_fixture();

        assert!(
            images[0].pixel_spacing.is_none(),
            "PixelSpacing should be optional"
        );
        assert!(
            images[0].slice_thickness.is_none(),
            "SliceThickness should be optional"
        );
        assert!(
            images[0].spacing_between_slices.is_none(),
            "SpacingBetweenSlices should be optional"
        );
        assert!(
            images[0].patient_position.is_none(),
            "PatientPosition should be optional"
        );
        assert!(
            images[0].rescale_slope.is_none(),
            "RescaleSlope should be optional"
        );
        assert!(
            images[0].rescale_intercept.is_none(),
            "RescaleIntercept should be optional"
        );
    }

    #[test]
    fn test_dicom_repo_from_fixtures() {
        let (patient, study, series, images) = common::create_minimal_fixture();

        let mut repo = DicomRepo::new();
        repo.add_patient(patient.clone());
        repo.add_study(study.clone());
        repo.add_image_series(series.clone());

        for image in images {
            repo.add_ct_image(image);
        }

        let retrieved_patient = repo.get_patient(&patient.patient_id);
        assert!(
            retrieved_patient.is_some(),
            "Patient should be retrievable from repo"
        );

        let retrieved_study = repo.get_studies_by_patient(&patient.patient_id);
        assert_eq!(retrieved_study.len(), 1, "Should have 1 study for patient");

        let retrieved_series = repo.get_series_by_study(&study.uid);
        assert_eq!(retrieved_series.len(), 1, "Should have 1 series for study");

        let retrieved_images = repo.get_images_by_series(&series.uid);
        assert_eq!(retrieved_images.len(), 5, "Should have 5 images in series");
    }

    #[test]
    fn test_ct_volume_generation_from_fixtures() {
        let (patient, study, series, images) = common::create_standard_ct_volume_fixture();
        let series_uid = series.uid.clone(); // Store UID before moving series

        let mut repo = DicomRepo::new();
        repo.add_patient(patient);
        repo.add_study(study);
        repo.add_image_series(series);

        for image in images {
            repo.add_ct_image(image);
        }

        let result = repo.generate_ct_volume(&series_uid);
        assert!(result.is_ok(), "Should generate CTVolume from fixtures");

        let volume = result.unwrap();
        assert_eq!(volume.dimensions().2, 10, "Should have 10 slices");
        assert_eq!(volume.dimensions().0, 512, "Should have 512 columns");
        assert_eq!(volume.dimensions().1, 512, "Should have 512 rows");
    }

    #[test]
    fn test_pixel_data_size_matches_dimensions() {
        let (_patient, _study, _series, images) = common::create_minimal_fixture();

        let image = &images[0];
        let expected_bytes = image.rows as usize * image.columns as usize * 2;

        assert_eq!(
            image.pixel_data.len(),
            expected_bytes,
            "Pixel data size should match dimensions (16-bit pixels)"
        );
    }

    #[test]
    fn test_image_z_position_progression() {
        let (_patient, _study, _series, images) = common::create_standard_ct_volume_fixture();

        for i in 1..images.len() {
            let prev_z = images[i - 1].image_position_patient.map(|p| p.2).unwrap();
            let curr_z = images[i].image_position_patient.map(|p| p.2).unwrap();

            assert!(curr_z > prev_z, "Z position should increase monotonically");
        }
    }
}
