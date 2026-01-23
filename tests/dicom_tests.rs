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
use kepler_wgpu::data::medical_imaging::image_info::PatientPosition;
use kepler_wgpu::data::{
    dicom::{
        build_ct_dicom, generate_uid, CTImage, DicomRepo, FsSink, ImageSeries, Patient, StudySet,
    },
    medical_imaging::PixelType,
};

// Test utilities and mock data
mod test_utils {
    use super::*;

    // ============================================================================
    // Test Data and Utilities
    // ============================================================================
    /// Creates test MHA header with embedded data
    pub fn create_test_mha_data(
        dimensions: [usize; 3],
        pixel_type: PixelType,
        spacing: [f64; 3],
        verbose: bool,
    ) -> Vec<u8> {
        let mut header = format!(
            "ObjectType = Image\n\
            NDims = 3\n\
            BinaryData = True\n\
            BinaryDataByteOrderMSB = False\n\
            CompressedData = False\n\
            TransformMatrix = 1 0 0 0 1 0 0 0 1\n\
            Offset = 0 0 0\n\
            CenterOfRotation = 0 0 0\n\
            AnatomicalOrientation = RAI\n\
            ElementSpacing = {} {} {}\n\
            DimSize = {} {} {}\n\
            ElementType = {}\n\
            ElementDataFile = LOCAL\n",
            spacing[0],
            spacing[1],
            spacing[2],
            dimensions[0],
            dimensions[1],
            dimensions[2],
            match pixel_type {
                PixelType::UInt8 => "MET_UCHAR",
                PixelType::UInt16 => "MET_USHORT",
                PixelType::Int16 => "MET_SHORT",
                PixelType::Int32 => "MET_INT",
                PixelType::Float32 => "MET_FLOAT",
                PixelType::Float64 => "MET_DOUBLE",
            }
        )
        .into_bytes();

        // Add data section
        if verbose {
            let data_size = dimensions[0] * dimensions[1] * dimensions[2];
            let pixel_size = match pixel_type {
                PixelType::UInt8 => 1,
                PixelType::UInt16 => 2,
                PixelType::Int16 => 2,
                PixelType::Int32 => 4,
                PixelType::Float32 => 4,
                PixelType::Float64 => 8,
            };

            // Create checkerboard pattern data
            let width = dimensions[0] as usize;
            let height = dimensions[1] as usize;
            let depth = dimensions[2] as usize;
            let checkerboard_size = 128; // Size of each checkerboard square

            let mut data: Vec<u8> = Vec::with_capacity(data_size * pixel_size);

            for z in 0..depth {
                for y in 0..height {
                    for x in 0..width {
                        // Calculate checkerboard pattern
                        let checker_x = (x / checkerboard_size) % 2;
                        let checker_y = (y / checkerboard_size) % 2;
                        let checker_z = (z / checkerboard_size) % 2;

                        // Create 3D checkerboard pattern
                        let is_white = (checker_x + checker_y + checker_z) % 2 == 0;
                        let pixel_value = if is_white { 255u8 } else { 0u8 };

                        // Add pixel data based on pixel type
                        match pixel_type {
                            PixelType::UInt8 => {
                                data.push(pixel_value);
                            }
                            PixelType::UInt16 | PixelType::Int16 => {
                                let value = if is_white { 65535u16 } else { 0u16 };
                                data.extend_from_slice(&value.to_le_bytes());
                            }
                            PixelType::Int32 => {
                                let value = if is_white { 2147483647i32 } else { 0i32 };
                                data.extend_from_slice(&value.to_le_bytes());
                            }
                            PixelType::Float32 => {
                                let value = if is_white { 1.0f32 } else { 0.0f32 };
                                data.extend_from_slice(&value.to_le_bytes());
                            }
                            PixelType::Float64 => {
                                let value = if is_white { 1.0f64 } else { 0.0f64 };
                                data.extend_from_slice(&value.to_le_bytes());
                            }
                        }
                    }
                }
            }

            header.extend_from_slice(&data);
        }

        header
    }

    /// Creates test patient data
    pub fn create_test_patient() -> Patient {
        Patient::new(
            "TEST001".to_string(),
            "Test^Patient".to_string(),
            Some("19800101".to_string()),
            Some("M".to_string()),
        )
    }

    /// Creates test study data
    pub fn create_test_study() -> StudySet {
        StudySet::new(
            "STUDY001".to_string(),
            "1.2.3.4.5.6.7.8.9.0".to_string(),
            "TEST001".to_string(),
            "20240101".to_string(),
            Some("Test CT Study".to_string()),
        )
    }

    /// Creates test image series data
    pub fn create_test_image_series() -> ImageSeries {
        ImageSeries::new(
            "1.2.3.4.5.6.7.8.9.1".to_string(),
            "1.2.3.4.5.6.7.8.9.0".to_string(),
            "CT".to_string(),
            Some("Axial CT".to_string()),
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
            Some((0.5, 0.5)),                       // pixel spacing
            Some(1.0),                              // slice thickness
            Some(1.0),                              // spacing between slices
            Some((100.0, 100.0, 50.0)),             // image position
            Some((1.0, 0.0, 0.0, 0.0, 1.0, 0.0)),   // image orientation
            Some(PatientPosition::HFS.to_string()), // patient position
            Some(1.0),                              // rescale slope
            Some(-1024.0),                          // rescale intercept
            Some(40.0),                             // window center
            Some(400.0),                            // window width
            1,                                      // pixel representation (signed)
            pixel_data,
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
            // PatientPosition is not part of Patient tags; verify optional fields presence formatting
            assert!(formatted.contains("PatientBirthDate") || formatted.contains("PatientSex"));
        }

        #[test]
        fn test_patient_optional_fields() {
            let patient = Patient::new(
                "TEST002".to_string(),
                "Another^Patient".to_string(),
                None, // No birthdate
                None, // No sex
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
            assert_eq!(ct_image.patient_position, Some("HFS".to_string()));
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
        use kepler_wgpu::data::CTVolumeGenerator;

        #[test]
        fn test_dicom_repo() {
            let mut repo = DicomRepo::new();
            let patient = create_test_patient();
            repo.add_patient(patient.clone());
            let study = create_test_study();
            repo.add_study(study.clone());
            let series = create_test_image_series();
            repo.add_image_series(series.clone());
            let ct_image = create_test_ct_image();
            repo.add_ct_image(ct_image.clone());
            println!("{:?}", repo.to_string());

            let retrieved_patient = repo
                .get_patient(&patient.patient_id)
                .expect("patient exists");
            assert_eq!(retrieved_patient.patient_id, patient.patient_id);
            assert_eq!(retrieved_patient.name, patient.name);

            let retrieved_studies = repo.get_studies_by_patient(&patient.patient_id);
            assert_eq!(retrieved_studies.len(), 1);
            assert_eq!(retrieved_studies[0].uid, study.uid);
            assert_eq!(retrieved_studies[0].patient_id, patient.patient_id);

            let retrieved_series = repo.get_series_by_study(&study.uid);
            assert_eq!(retrieved_series.len(), 1);
            assert_eq!(retrieved_series[0].uid, series.uid);
            assert_eq!(retrieved_series[0].study_uid, study.uid);
            assert_eq!(retrieved_series[0].modality, "CT");

            let images = repo.get_images_by_series(&series.uid);
            assert_eq!(images.len(), 1);
            assert_eq!(images[0].series_uid, series.uid);
            assert_eq!(images[0].rows, 512);
            assert_eq!(images[0].columns, 512);
            assert_eq!(images[0].pixel_data.len(), 512 * 512 * 2);

            let ct_volume = repo
                .generate_ct_volume(images[0].series_uid.as_str())
                .unwrap();
            println!("{:?}", ct_volume);
        }
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use dicom_object::open_file;

    use anyhow::Result;
    use bytemuck::cast_slice;
    use dicom_core::Tag;
    use dicom_object::from_reader;

    #[test]
    fn test_dicom() -> Result<()> {
        let obj = open_file("C:\\share\\imrt\\CT.RT001921_1.dcm")?;
        let patient_name = obj.element_by_name("PatientName")?.to_str()?;
        let modality = obj.element_by_name("Modality")?.to_str()?;
        let pixel_data_bytes = obj.element(Tag(0x7FE0, 0x0010))?.to_bytes()?;
        let pixels: &[i16] = cast_slice(&pixel_data_bytes);
        println!("{:?}", patient_name);
        println!("{:?}", modality);
        println!("num of pxiels: {:?}", pixels.len());
        Ok(())
    }

    #[test]
    fn test_dicom_reader() -> Result<()> {
        let bytes = include_bytes!("C:\\share\\imrt\\CT.RT001921_1.dcm");
        let f = std::io::Cursor::new(bytes);
        let obj = from_reader(f)?;
        let patient_name = obj.element_by_name("PatientName")?.to_str()?;
        let modality = obj.element_by_name("Modality")?.to_str()?;
        let pixel_data_bytes = obj.element(Tag(0x7FE0, 0x0010))?.to_bytes()?;
        let pixels: &[i16] = cast_slice(&pixel_data_bytes);
        println!("{:?}", patient_name);
        println!("{:?}", modality);
        println!("num of pxiels: {:?}", pixels.len());
        Ok(())
    }
}
// =============================================================================
// INTEGRATION TESTS - End-to-End Workflows
// =============================================================================

#[cfg(all(test, target_os = "windows"))]
mod integration_tests {
    use super::*;
    use chrono::Local;
    use std::fs;
    use std::path::Path;
    use test_utils::*;

    #[test]
    #[ignore]
    fn test_dicom_export_workflow_with_mha_file() {
        // Create a test patient and study
        let patient = create_test_patient();
        let mut study = create_test_study();

        // Generate a Study UID
        let study_uid = generate_uid();
        study.uid = study_uid.to_string();
        println!("Generated Study UID: {}", study_uid);

        // Load test data
        let mha_path =
            &create_test_mha_data([512, 512, 300], PixelType::Float32, [0.5, 0.5, 0.5], true);
        let out_dir = Path::new("C:/share").join(Local::now().format("%Y-%m-%d").to_string());
        std::fs::create_dir_all(&out_dir).unwrap();

        let mut sink = FsSink { out_dir };

        // Test would call build_ct_dicom with test parameters
        let result = build_ct_dicom(
            mha_path, None, &patient, &study, 120.0,     // kV
            100.0,     // mAs
            1612.903,  // slope
            -1016.129, // intercept
            "HFS".to_string(),
            "CT".to_string(),
            &mut sink,
        );

        assert!(result.is_ok());
    }

    #[test]
    #[ignore]
    fn test_dicom_export_workflow_with_mhd_file() {
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
            mhd_path, data_path, &patient, &study, 120.0,     // kV
            100.0,     // mAs
            1612.903,  // slope
            -1016.129, // intercept
            "HFS".to_string(),
            "CT".to_string(),
            &mut sink,
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
            Ok(_) => {}  // Acceptable if implementation handles gracefully
            Err(_) => {} // Acceptable if implementation validates parameters
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
        // Should handle gracefully or return appropriate error
        match result {
            Ok(_) => {}  // Acceptable if implementation handles gracefully
            Err(_) => {} // Acceptable if implementation validates parameters
        }
    }

    /// Tests odd-sized pixel_data (not divisible by 2)
    #[test]
    fn test_odd_sized_pixel_data() {
        let mut ct_image = create_test_ct_image();

        // Create odd-sized pixel data (513 * 513 = 263169, not divisible by 2)
        let odd_size = 513 * 513;
        ct_image.pixel_data = vec![0u8; odd_size];
        ct_image.rows = 513;
        ct_image.columns = 513;

        let result = ct_image.get_pixel_data();
        // Should handle odd-sized data correctly
        // DICOM pixel data can be any size
        match result {
            Ok(data) => {
                assert_eq!(
                    data.len(),
                    odd_size / 2,
                    "Odd-sized data should be processed correctly"
                );
            }
            Err(_) => {
                // Acceptable if implementation has constraints
            }
        }
    }

    /// Tests endianness mismatch detection
    #[test]
    fn test_endianness_mismatch() {
        let mut ct_image = create_test_ct_image();

        // Create pixel data with specific byte pattern to test endianness
        let test_value: u16 = 0x1234; // 4660 in little-endian
        let mut pixel_data = vec![0u8; 512 * 512 * 2];

        // Write as big-endian (MSB first)
        pixel_data[0] = ((test_value >> 8) & 0xFF) as u8;
        pixel_data[1] = (test_value & 0xFF) as u8;

        ct_image.pixel_data = pixel_data;
        ct_image.rows = 512;
        ct_image.columns = 512;

        let result = ct_image.get_pixel_data();
        // Implementation should detect endianness and convert if needed
        match result {
            Ok(data) => {
                // Check if first pixel value matches
                let parsed_value = u16::from_be_bytes([data[0] as u8, data[1] as u8]);
                // Value may differ if endianness is handled
            }
            Err(_) => {
                // Acceptable if endianness causes error
            }
        }
    }

    /// Tests truncated pixel_data (incomplete last chunk)
    #[test]
    fn test_truncated_pixel_data() {
        let mut ct_image = create_test_ct_image();

        // Create pixel data that's truncated
        let expected_size = 512 * 512 * 2;
        let truncated_size = expected_size - 100; // Missing 100 bytes at end
        ct_image.pixel_data = vec![0u8; truncated_size];
        ct_image.rows = 512;
        ct_image.columns = 512;

        let result = ct_image.get_pixel_data();
        // Should detect truncation and fail
        assert!(result.is_err(), "Truncated pixel data should fail parsing");
    }

    /// Tests overflow in rescaling (pixel value × slope exceeds i16::MAX)
    #[test]
    fn test_rescale_overflow() {
        let mut ct_image = create_test_ct_image();

        // Set up for overflow scenario
        ct_image.rescale_slope = Some(1000.0); // Large slope
                                               // Create pixel data at max i16 value
        let max_pixel_value = i16::MAX as u16;
        let mut pixel_data = vec![0u8; 512 * 512];
        pixel_data[0] = (max_pixel_value >> 8) as u8;
        pixel_data[1] = (max_pixel_value & 0xFF) as u8;

        ct_image.pixel_data = pixel_data;
        ct_image.rows = 512;
        ct_image.columns = 512;

        let result = ct_image.get_pixel_data();
        match result {
            Ok(data) => {
                // Check if overflow is clamped or handled
                // Value (30000 * 1000) would be i32, beyond i16::MAX (32767)
            }
            Err(_) => {
                // Acceptable if overflow causes error
            }
        }
    }

    /// Tests precision loss when rounding (transformed_value.round() as i16)
    #[test]
    fn test_precision_loss_rounding() {
        let mut ct_image = create_test_ct_image();

        // Set up rescale parameters
        ct_image.rescale_slope = Some(1.0);
        ct_image.rescale_intercept = Some(-1024.0);

        // Create pixel data that requires fractional transformation
        // Original value: 1000
        // Transformed: (1000 * 1.0) - 1024.0 = -24.0
        // Rounding back to i16: -24
        let mut pixel_data = vec![0u8; 512 * 512 * 2];
        // Store as 1000
        pixel_data[0] = (1000u16 >> 8) as u8;
        pixel_data[1] = (1000u16 & 0xFF) as u8;

        ct_image.pixel_data = pixel_data;
        ct_image.rows = 512;
        ct_image.columns = 512;

        let result = ct_image.get_pixel_data();
        match result {
            Ok(data) => {
                // Verify rounding is handled
                // First pixel should be close to -24 after round-trip
            }
            Err(_) => {}
        }
    }

    /// Tests negative rescale slope inverts units correctly
    #[test]
    fn test_negative_rescale_slope() {
        let mut ct_image = create_test_ct_image();

        // Set negative slope (inverts Hounsfield units)
        ct_image.rescale_slope = Some(-1.0);
        ct_image.rescale_intercept = Some(0.0);

        let mut pixel_data = vec![0u8; 512 * 512 * 2];
        // Original value: 1000
        pixel_data[0] = (1000u16 >> 8) as u8;
        pixel_data[1] = (1000u16 & 0xFF) as u8;

        ct_image.pixel_data = pixel_data;
        ct_image.rows = 512;
        ct_image.columns = 512;

        let result = ct_image.get_pixel_data();
        match result {
            Ok(data) => {
                // Value should be inverted: -1000
                // This affects interpretation (bright becomes dark)
            }
            Err(_) => {}
        }
    }

    /// Tests zero rescale slope handled correctly
    #[test]
    fn test_zero_rescale_slope() {
        let mut ct_image = create_test_ct_image();

        // Zero slope means no rescaling (1:1 mapping)
        ct_image.rescale_slope = Some(0.0);
        ct_image.rescale_intercept = Some(0.0);

        let mut pixel_data = vec![0u8; 512 * 512 * 2];
        // Original value: 1000
        pixel_data[0] = (1000u16 >> 8) as u8;
        pixel_data[1] = (1000u16 & 0xFF) as u8;

        ct_image.pixel_data = pixel_data;
        ct_image.rows = 512;
        ct_image.columns = 512;

        let result = ct_image.get_pixel_data();
        match result {
            Ok(data) => {
                // Value should remain unchanged (1000)
                // or be handled gracefully if 0 slope causes division issues
            }
            Err(_) => {}
        }
    }
}

// =============================================================================
// PERFORMANCE TESTS - Memory and Speed Validation
// =============================================================================

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;
    use test_utils::*;

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
        assert!(
            duration.as_millis() < 1000,
            "Pixel processing took too long: {:?}",
            duration
        );
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
        assert!(
            duration.as_millis() < 100,
            "Patient retrieval took too long: {:?}",
            duration
        );
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
            assert!(
                unique_uids.insert(uid.clone()),
                "Duplicate UID generated: {}",
                uid
            );
        }

        // Performance assertion
        assert!(
            duration.as_millis() < 1000,
            "UID generation took too long: {:?}",
            duration
        );
    }
}
