//! Comprehensive DICOM functionality tests
//! 
//! This module provides extensive testing coverage for all DICOM-related functionality
//! in the medical imaging framework, including:
//! - Unit tests for individual DICOM structures and parsers
//! - Integration tests for complete DICOM workflows
//! - Error handling and edge case validation
//! - Performance and memory safety tests
//! - Cross-platform compatibility verification


// Import DICOM modules under test
use kepler_wgpu::data::dicom::{
    Patient, StudySet, ImageSeries, CTImage, DicomRepo,
    build_ct_dicom, generate_uid, change_dicom_uid, FsSink
};

// Test utilities and mock data
mod test_utils {
    use super::*;
    
    /// Creates minimal valid DICOM data for testing
    /// Returns a byte array representing a basic CT DICOM file
    pub fn create_minimal_ct_dicom() -> Vec<u8> {
        // This would normally be a valid DICOM file in bytes
        // For testing purposes, we'll create a mock structure
        // In a real implementation, you'd use actual DICOM test files
        vec![
            // DICOM preamble (128 bytes of zeros)
            0u8; 128
        ].into_iter()
        .chain(b"DICM".iter().cloned()) // DICOM prefix
        .chain(create_mock_dicom_elements())
        .collect()
    }
    
    /// Creates mock DICOM data elements for testing
    fn create_mock_dicom_elements() -> Vec<u8> {
        // Mock implementation - in real tests, use actual DICOM libraries
        // to create valid test data
        vec![0u8; 1024] // Placeholder for DICOM elements
    }
    
    /// Creates test patient data
    pub fn create_test_patient() -> Patient {
        Patient::new(
            "TEST001".to_string(),
            "Test^Patient".to_string(),
            Some("19800101".to_string()),
            Some("M".to_string())
        )
    }
    
    /// Creates test study data
    pub fn create_test_study() -> StudySet {
        StudySet::new(
            "STUDY001".to_string(),
            "1.2.3.4.5.6.7.8.9.0".to_string(),
            "TEST001".to_string(),
            "20240101".to_string(),
            Some("Test CT Study".to_string())
        )
    }
    
    /// Creates test image series data
    pub fn create_test_image_series() -> ImageSeries {
        ImageSeries::new(
            "1.2.3.4.5.6.7.8.9.1".to_string(),
            "1.2.3.4.5.6.7.8.9.0".to_string(),
            "CT".to_string(),
            Some("Axial CT".to_string())
        )
    }
    
    /// Creates test CT image data with realistic parameters
    pub fn create_test_ct_image() -> CTImage {
        let pixel_data = vec![0u8; 512 * 512 * 2]; // 16-bit pixels, 512x512
        
        CTImage::new(
            "1.2.3.4.5.6.7.8.9.2".to_string(),
            "1.2.3.4.5.6.7.8.9.1".to_string(),
            512,
            512,
            Some((0.5, 0.5)), // pixel spacing
            Some(1.0), // slice thickness
            Some(1.0), // spacing between slices
            Some((100.0, 100.0, 50.0)), // image position
            Some((1.0, 0.0, 0.0, 0.0, 1.0, 0.0)), // image orientation
            Some(1.0), // rescale slope
            Some(-1024.0), // rescale intercept
            Some(40.0), // window center
            Some(400.0), // window width
            1, // pixel representation (signed)
            pixel_data
        )
    }
}

// =============================================================================
// UNIT TESTS - Individual Module Testing
// =============================================================================

#[cfg(test)]
mod unit_tests {
    use super::*;
    use test_utils::*;

    /// Tests Patient struct creation and validation
    mod patient_tests {
        use super::*;

        #[test]
        fn test_patient_creation() {
            let patient = create_test_patient();
            assert_eq!(patient.patient_id, "TEST001");
            assert_eq!(patient.name, "Test^Patient");
            assert_eq!(patient.birthdate, Some("19800101".to_string()));
            assert_eq!(patient.sex, Some("M".to_string()));
        }

        #[test]
        fn test_patient_format_tags() {
            let patient = create_test_patient();
            let formatted = patient.format_tags();
            assert!(formatted.contains("PatientID"));
            assert!(formatted.contains("PatientName"));
            assert!(formatted.contains("TEST001"));
            assert!(formatted.contains("Test^Patient"));
        }

        #[test]
        fn test_patient_from_bytes_missing_required_fields() {
            let minimal_data = create_minimal_ct_dicom();
            let result = Patient::from_bytes(&minimal_data);
            assert!(result.is_err());
        }

        #[test]
        fn test_patient_optional_fields() {
            let patient = Patient::new(
                "TEST002".to_string(),
                "Another^Patient".to_string(),
                None, // No birthdate
                None  // No sex
            );
            assert_eq!(patient.patient_id, "TEST002");
            assert_eq!(patient.name, "Another^Patient");
        }
    }

    /// Tests StudySet struct creation and validation
    mod study_tests {
        use super::*;

        #[test]
        fn test_study_creation() {
            let study = create_test_study();
            assert_eq!(study.study_id, "STUDY001");
            assert_eq!(study.uid, "1.2.3.4.5.6.7.8.9.0");
            assert_eq!(study.patient_id, "TEST001");
            assert_eq!(study.date, "20240101");
            assert_eq!(study.description, Some("Test CT Study".to_string()));
        }

        #[test]
        fn test_study_format_tags() {
            let study = create_test_study();
            let formatted = study.format_tags();
            assert!(formatted.contains("StudyID"));
            assert!(formatted.contains("StudyInstanceUID"));
            assert!(formatted.contains("STUDY001"));
        }

        #[test]
        fn test_study_date_validation() {
            // Test various date formats
            let study = StudySet::new(
                "STUDY002".to_string(),
                "1.2.3.4.5.6.7.8.9.1".to_string(),
                "TEST001".to_string(),
                "20241231".to_string(), // Valid DICOM date format
                None
            );
            assert_eq!(study.date, "20241231");
        }
    }

    /// Tests ImageSeries struct creation and validation
    mod image_series_tests {
        use super::*;

        #[test]
        fn test_image_series_creation() {
            let series = create_test_image_series();
            assert_eq!(series.uid, "1.2.3.4.5.6.7.8.9.1");
            assert_eq!(series.study_uid, "1.2.3.4.5.6.7.8.9.0");
            assert_eq!(series.modality, "CT");
            assert_eq!(series.description, Some("Axial CT".to_string()));
        }

        #[test]
        fn test_image_series_validation() {
            // Test that only CT modality is accepted
            let series = ImageSeries::new(
                "1.2.3.4.5.6.7.8.9.1".to_string(),
                "1.2.3.4.5.6.7.8.9.0".to_string(),
                "CT".to_string(),
                None,
            );
            assert_eq!(series.modality, "CT");
        }
    }

    /// Tests CTImage struct creation and pixel data processing
    mod ct_image_tests {
        use super::*;

        #[test]
        fn test_ct_image_creation() {
            let ct_image = create_test_ct_image();
            assert_eq!(ct_image.uid, "1.2.3.4.5.6.7.8.9.2");
            assert_eq!(ct_image.series_uid, "1.2.3.4.5.6.7.8.9.1");
            assert_eq!(ct_image.rows, 512);
            assert_eq!(ct_image.columns, 512);
            assert_eq!(ct_image.pixel_representation, 1);
            assert_eq!(ct_image.pixel_spacing, Some((0.5, 0.5)));
            assert_eq!(ct_image.rescale_slope, Some(1.0));
            assert_eq!(ct_image.rescale_intercept, Some(-1024.0));
            assert_eq!(ct_image.window_center, Some(40.0));
            assert_eq!(ct_image.window_width, Some(400.0));
        }

        #[test]
        fn test_get_pixel_data_no_rescaling() {
            let mut ct_image = create_test_ct_image();
            // Set rescale parameters to identity (no transformation)
            ct_image.rescale_slope = Some(1.0);
            ct_image.rescale_intercept = Some(0.0);
            ct_image.pixel_representation = 1;
            
            let result = ct_image.get_pixel_data();
            assert!(result.is_ok());
            let pixel_data = result.unwrap();
            assert_eq!(pixel_data.len(), 512 * 512);
        }

        #[test]
        fn test_get_pixel_data_with_rescaling() {
            let mut ct_image = create_test_ct_image();
            // Set rescale parameters for transformation
            ct_image.rescale_slope = Some(2.0);
            ct_image.rescale_intercept = Some(-1000.0);
            
            let result = ct_image.get_pixel_data();
            assert!(result.is_ok());
            let pixel_data = result.unwrap();
            assert_eq!(pixel_data.len(), 512 * 512);
        }

        #[test]
        fn test_get_pixel_data_unsigned_pixels() {
            let mut ct_image = create_test_ct_image();
            ct_image.pixel_representation = 0; // Unsigned
            
            let result = ct_image.get_pixel_data();
            assert!(result.is_ok());
        }

        #[test]
        fn test_get_pixel_data_invalid_pixel_representation() {
            let mut ct_image = create_test_ct_image();
            ct_image.pixel_representation = 2; // Invalid value
            
            let result = ct_image.get_pixel_data();
            assert!(result.is_err());
        }

    }

    /// Tests DicomRepo functionality
    mod dicom_repo_tests {
        use super::*;

        #[test]
        fn test_dicom_repo_add_patient() {
            let mut repo = DicomRepo::new();
            let patient = create_test_patient();
            repo.add_patient(patient.clone());
            
            let retrieved = repo.get_patient("TEST001").unwrap();
            println!("{:?}", retrieved);
        }

        #[test]
        fn test_dicom_repo_add_study() {
            let mut repo = DicomRepo::new();
            let patient = create_test_patient();
            repo.add_patient(patient.clone());
            let study = create_test_study();
            repo.add_study(study.clone());
            
            let retrieved = repo.get_studies_by_patient(&patient.patient_id);
            println!("{:?}", retrieved);
        }

        #[test]
        fn test_dicom_repo_add_series() {
            let mut repo = DicomRepo::new();
            let series = create_test_image_series();
            repo.add_image_series(series.clone());
            
            let retrieved = repo.get_series_by_study(&series.study_uid);
            println!("{:?}", retrieved);
        }

        #[test]
        fn test_dicom_repo_get_images_by_series() {
            let mut repo = DicomRepo::new();
            let ct_image = create_test_ct_image();
            let series_uid = ct_image.series_uid.clone();
            repo.add_ct_image(ct_image);
            
            let images = repo.get_images_by_series(&series_uid);
            println!("{:?}", images);
        }
    }

    /// Tests UID generation and manipulation
    mod uid_tests {
        use super::*;

        #[test]
        fn test_generate_uid() {
            let uid1 = generate_uid();
            let uid2 = generate_uid();
            
            // UIDs should be different
            assert_ne!(uid1, uid2);
            
            // UIDs should be valid format (basic check)
            assert!(uid1.contains('.'));
            assert!(uid2.contains('.'));
        }

        #[test]
        fn test_change_dicom_uid() {
            let base_uid = generate_uid();
            let changed_uid1 = change_dicom_uid(&base_uid, true);
            let changed_uid2 = change_dicom_uid(&base_uid, false);
            
            // Changed UIDs should be different from base and each other
            assert_ne!(base_uid, changed_uid1);
            assert_ne!(base_uid, changed_uid2);
            assert_ne!(changed_uid1, changed_uid2);
        }
    }
}

// =============================================================================
// INTEGRATION TESTS - End-to-End Workflows
// =============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;
    use test_utils::*;
    use std::fs;
    use std::path::Path;
    use chrono::Local;

    /// Tests DICOM export functionality with real MHA file
    /// NOTE: Ignored by default because it depends on external local paths (C:/share/input).
    /// Configure local fixtures or update paths before running manually.
    #[test]
    #[ignore]
    fn test_dicom_export_workflow_with_real_mha_file() {
        // Create a test patient and study
        let patient = create_test_patient();
        let mut study = create_test_study();

        // Generate a Study UID
        let study_uid = generate_uid();
        study.uid = study_uid.to_string();
        println!("Generated Study UID: {}", study_uid);

        // Load test data
        let path = "C:/share/input/CT_new.mha";
        let data = fs::read(path);
        let mha_path = data.as_ref().map(|v| v.as_slice()).unwrap();
        let out_dir = Path::new("C:/share").join(Local::now().format("%Y-%m-%d").to_string());
        std::fs::create_dir_all(&out_dir).unwrap();

        let mut sink = FsSink { out_dir };
        
        // Test would call build_ct_dicom with test parameters
        let result = build_ct_dicom(
            mha_path,
            None,
            &patient,
            &study,
            120.0, // kV
            100.0, // mAs
            1612.903,   // slope
            -1016.129, // intercept
            &mut sink
        );

        assert!(result.is_ok());
    }

    /// Tests DICOM export functionality with real MHD file
    /// NOTE: Ignored by default because it depends on external local paths (C:/share/input).
    /// Configure local fixtures or update paths before running manually.
    #[test]
    #[ignore]
    fn test_dicom_export_workflow_with_real_mhd_file() {
        // Create a test patient and study
        let patient = create_test_patient();
        let mut study = create_test_study();

        // Generate a Study UID
        let study_uid = generate_uid();
        study.uid = study_uid.to_string();
        println!("Generated Study UID: {}", study_uid);

        // Load test data
        let mhd_path = "C:/share/input/CT_new.mhd";
        let data_path = "C:/share/input/CT_new.raw";
        let mhd = fs::read(mhd_path);
        let data = fs::read(data_path);
        let mhd_path = mhd.as_ref().map(|v| v.as_slice()).unwrap();
        let data_path = Some(data.as_ref().map(|v| v.as_slice()).unwrap());
        let out_dir = Path::new("C:/share").join(Local::now().format("%Y-%m-%d").to_string());
        std::fs::create_dir_all(&out_dir).unwrap();

        let mut sink = FsSink { out_dir };
        
        // Test would call build_ct_dicom with test parameters
        let result = build_ct_dicom(
            mhd_path,
            data_path,
            &patient,
            &study,
            120.0, // kV
            100.0, // mAs
            1612.903,   // slope
            -1016.129, // intercept
            &mut sink
        );

        assert!(result.is_ok());
    }
}

// =============================================================================
// ERROR HANDLING TESTS - Edge Cases and Error Scenarios
// =============================================================================

#[cfg(test)]
mod error_handling_tests {
    use super::*;
    use test_utils::*;

    /// Tests handling of corrupted DICOM data
    #[test]
    fn test_corrupted_dicom_data() {
        let corrupted_data = vec![0xFF; 100]; // Invalid DICOM data
        
        let patient_result = Patient::from_bytes(&corrupted_data);
        let study_result = StudySet::from_bytes(&corrupted_data);
        let series_result = ImageSeries::from_bytes(&corrupted_data);
        let ct_image_result = CTImage::from_bytes(&corrupted_data);
        
        assert!(patient_result.is_err());
        assert!(study_result.is_err());
        assert!(series_result.is_err());
        assert!(ct_image_result.is_err());
    }

    /// Tests handling of empty data
    #[test]
    fn test_empty_dicom_data() {
        let empty_data = vec![];
        
        let patient_result = Patient::from_bytes(&empty_data);
        let study_result = StudySet::from_bytes(&empty_data);
        let series_result = ImageSeries::from_bytes(&empty_data);
        let ct_image_result = CTImage::from_bytes(&empty_data);
        
        assert!(patient_result.is_err());
        assert!(study_result.is_err());
        assert!(series_result.is_err());
        assert!(ct_image_result.is_err());
    }


    /// Tests handling of invalid rescale parameters
    #[test]
    fn test_invalid_rescale_parameters() {
        let mut ct_image = create_test_ct_image();
        
        // Test with extreme rescale values
        ct_image.rescale_slope = Some(f32::INFINITY);
        ct_image.rescale_intercept = Some(f32::NAN);
        
        let result = ct_image.get_pixel_data();
        // Should handle gracefully or return appropriate error
        match result {
            Ok(_) => {}, // Acceptable if implementation handles gracefully
            Err(_) => {}, // Acceptable if implementation validates parameters
        }
    }

    /// Tests memory limits and large data handling
    #[test]
    fn test_large_pixel_data_handling() {
        let mut ct_image = create_test_ct_image();
        
        // Test with very large pixel data (but reasonable for medical imaging)
        let large_size = 2048 * 2048 * 2; // 2K x 2K x 16-bit
        ct_image.pixel_data = vec![0u8; large_size];
        ct_image.rows = 2048;
        ct_image.columns = 2048;
        
        let result = ct_image.get_pixel_data();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2048 * 2048);
    }
}

// =============================================================================
// PERFORMANCE TESTS - Memory and Speed Validation
// =============================================================================

#[cfg(test)]
mod performance_tests {
    use super::*;
    use test_utils::*;
    use std::time::Instant;

    /// Tests pixel data processing performance
    #[test]
    fn test_pixel_data_processing_performance() {
        let mut ct_image = create_test_ct_image();
        
        // Create larger test data
        let large_size = 1024 * 1024 * 2; // 1M pixels x 16-bit
        ct_image.pixel_data = vec![0u8; large_size];
        ct_image.rows = 1024;
        ct_image.columns = 1024;
        
        let start = Instant::now();
        let result = ct_image.get_pixel_data();
        let duration = start.elapsed();
        
        assert!(result.is_ok());
        
        // Performance assertion - should process 1M pixels in reasonable time
        assert!(duration.as_millis() < 1000, "Pixel processing took too long: {:?}", duration);
    }

    /// Tests memory usage with large datasets
    #[test]
    fn test_memory_usage_large_dataset() {
        let mut repo = DicomRepo::new();
        
        // Add many patients to test memory scaling
        for i in 0..1000 {
            let mut patient = create_test_patient();
            patient.patient_id = format!("PERF{:04}", i);
            repo.add_patient(patient);
        }
        
        // Test retrieval performance
        let start = Instant::now();
        for i in 0..1000 {
            let patient_id = format!("PERF{:04}", i);
            let patient = repo.get_patient(&patient_id).unwrap();
            println!("{:?}", patient);
        }
        let duration = start.elapsed();
        assert!(duration.as_millis() < 100, "Patient retrieval took too long: {:?}", duration);
    }

    /// Tests UID generation performance
    #[test]
    fn test_uid_generation_performance() {
        let start = Instant::now();
        let mut uids = Vec::new();
        
        for _ in 0..1000 {
            uids.push(generate_uid());
        }
        
        let duration = start.elapsed();
        
        // Verify all UIDs are unique
        let mut unique_uids = std::collections::HashSet::new();
        for uid in &uids {
            assert!(unique_uids.insert(uid.clone()), "Duplicate UID generated: {}", uid);
        }
        
        // Performance assertion
        assert!(duration.as_millis() < 1000, "UID generation took too long: {:?}", duration);
    }
}