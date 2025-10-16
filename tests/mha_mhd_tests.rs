//! Comprehensive test suite for MHA/MHD medical imaging functionality
//! 
//! This test suite validates all critical paths, edge cases, and error handling scenarios
//! for the medical imaging module, ensuring robust functionality across different platforms
mod mha_mhd_tests{
    use std::fs;
    use kepler_wgpu::data::medical_imaging::{*};

    // ============================================================================
    // Test Data and Utilities
    // ============================================================================

    fn create_test_mhd() -> (Vec<u8>, Vec<u8>) {
        let path = "C:/share/input/CT.mhd";
        let mhd = fs::read(path);
        let bytes_mhd = mhd.as_ref().map(|v| v.as_slice()).unwrap();
        let data = fs::read(path.replace("mhd", "raw"));
        let bytes_data = data.as_ref().map(|v| v.as_slice()).unwrap();
        (bytes_mhd.to_vec(), bytes_data.to_vec())
    }

    fn create_test_mha() -> Vec<u8> {
        let path = "C:/share/input/CT.mha";
        let mha = fs::read(path);
        mha.as_ref().map(|v| v.as_slice()).unwrap().to_vec()
    }

    /// Creates test MHA header with embedded data
    fn create_test_mha_data(
        dimensions: [usize; 3],
        pixel_type: PixelType,
        spacing: [f64; 3],
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
            spacing[0], spacing[1], spacing[2],
            dimensions[0], dimensions[1], dimensions[2],
            match pixel_type {
                PixelType::UInt8 => "MET_UCHAR",
                PixelType::UInt16 => "MET_USHORT",
                PixelType::Int16 => "MET_SHORT",
                PixelType::Int32 => "MET_INT",
                PixelType::Float32 => "MET_FLOAT",
                PixelType::Float64 => "MET_DOUBLE",
            }
        ).into_bytes();

        // Add data section
        let data_size = dimensions[0] * dimensions[1] * dimensions[2];
        let pixel_size = match pixel_type {
            PixelType::UInt8 => 1,
            PixelType::UInt16 => 2,
            PixelType::Int16 => 2,
            PixelType::Int32 => 4,
            PixelType::Float32 => 4,
            PixelType::Float64 => 8,
        };
        
        let data: Vec<u8> = (0..data_size * pixel_size)
            .map(|i| (i % 256) as u8)
            .collect();
        
        header.extend_from_slice(&data);
        header
    }

    // ============================================================================
    // Unit Tests - Format Parsers
    // ============================================================================

    #[test]
    fn test_mhd_parser_header_by_bytes() {
        let (bytes_mhd, _) = create_test_mhd();
        let result = MhdParser::parse_metadata_only(&bytes_mhd);
        assert!(result.is_ok(), "Failed to parse valid MHD header: {:?}", result.err());

        let metadata = result.unwrap();
        assert_eq!(metadata.dimensions, [512, 512, 300]);
        assert_eq!(metadata.pixel_type, PixelType::Float32);
        assert_eq!(metadata.spacing, [0.2, 0.2, 0.5]);
    }

    #[test]
    fn test_mhd_parse_by_bytes(){
        let (bytes_mhd, bytes_data) = create_test_mhd();
        let volume = MhdParser::parse_by_bytes(&bytes_mhd, &bytes_data).unwrap();
        let header = volume.metadata;
        let pixel_data = volume.pixel_data;
        println!("=== MHDHeader ===");
        println!("DimSize: {:?}", header.dimensions);
        println!("ElementSpacing: {:?}", header.spacing);
        println!("ElementType: {:?}", header.pixel_type);
        println!("ElementDataFile: {}", header.element_data_file);
        println!("Offset: {:?}", header.offset);
        println!("TransformMatrix: {:?}", header.orientation);
        println!("PatientPosition: {:?}",header.patient_position);
        println!("data_offset: {:?}", header.data_offset);
        println!("First 20 bytes of pixel data: {:?}", &pixel_data.as_bytes()[..20]);
    }

    #[test]
    fn test_mha_parser_header_by_bytes() {
        let mha_data = create_test_mha();
        let result = MhaParser::parse_metadata_only(&mha_data);
        assert!(result.is_ok(), "Failed to parse MHA metadata");

        let metadata = result.unwrap();
        assert_eq!(metadata.dimensions, [512, 512, 300]);
        assert_eq!(metadata.pixel_type, PixelType::Float32);
        assert_eq!(metadata.spacing, [0.2, 0.2, 0.5]);
    }

    #[test]
    fn test_mha_parse_by_bytes(){
        let mha_data = create_test_mha();
        let volume = MhaParser::parse_bytes(&mha_data).unwrap();
        let validation_status = volume.validation_status;
        assert_eq!(validation_status.is_valid, true);
        let header = volume.metadata;
        let pixel_data = volume.pixel_data;
        assert_eq!(volume.source_format.clone(), ImageFormat::MHA);
        println!("=== MHAHeader ===");
        println!("DimSize: {:?}", header.dimensions);
        println!("ElementSpacing: {:?}", header.spacing);
        println!("ElementType: {:?}", header.pixel_type);
        println!("ElementDataFile: {}", header.element_data_file);
        println!("Offset: {:?}", header.offset);
        println!("TransformMatrix: {:?}", header.orientation);
        println!("PatientPosition: {:?}",header.patient_position);
        println!("data_offset: {:?}", header.data_offset);
        println!("First 20 bytes of pixel data: {:?}", &pixel_data.as_bytes()[..20]);
    }

}

// /// Function-level comment: Creates malformed test data for error testing
// /// Generates various types of corrupted data to test error handling
// fn create_malformed_data(error_type: &str) -> Vec<u8> {
//     match error_type {
//         "invalid_header" => b"InvalidHeader\nNotAValidFormat".to_vec(),
//         "missing_dimensions" => b"ObjectType = Image\nNDims = 3\n".to_vec(),
//         "invalid_pixel_type" => {
//             b"ObjectType = Image\nNDims = 3\nElementType = INVALID_TYPE\nDimSize = 10 10 10\n".to_vec()
//         },
//         "truncated_data" => {
//             let mut data = create_test_mha_data([10, 10, 10], PixelType::UInt8, [1.0, 1.0, 1.0]);
//             data.truncate(data.len() / 2); // Truncate to simulate corruption
//             data
//         },
//         _ => b"".to_vec(),
//     }
// }

// #[cfg(test)]
// mod parser_tests {
//     use super::*;

//     #[test]
//     fn test_mhd_parser_embedded_data() {
//         let header_data = create_test_mhd_header(
//             [10, 10, 5],
//             PixelType::UInt8,
//             [0.5, 0.5, 1.0],
//             None // LOCAL data
//         );

//         let result = MhdParser::parse_single_file(&header_data);
//         assert!(result.is_ok(), "Failed to parse MHD with embedded data");

//         let metadata = result.unwrap();
//         assert_eq!(metadata.dimensions, [10, 10, 5]);
//         assert_eq!(metadata.pixel_type, PixelType::UInt8);
//     }

//     #[test]
//     fn test_mha_parser_complete_file() {
//         let mha_data = create_test_mha_data([8, 8, 8], PixelType::Int16, [1.0, 1.0, 1.0]);

//         let result = MhaParser::parse_bytes(&mha_data);
//         assert!(result.is_ok(), "Failed to parse valid MHA file: {:?}", result.err());

//         let volume = result.unwrap();
//         assert_eq!(volume.metadata.dimensions, [8, 8, 8]);
//         assert_eq!(volume.metadata.pixel_type, PixelType::Int16);
//         assert_eq!(volume.source_format, ImageFormat::MHA);
//     }

//     #[test]
//     fn test_parser_different_pixel_types() {
//         let pixel_types = [
//             PixelType::UInt8,
//             PixelType::Int16,
//             PixelType::Float32,
//             PixelType::Float64,
//         ];

//         for pixel_type in &pixel_types {
//             let mha_data = create_test_mha_data([4, 4, 4], *pixel_type, [1.0, 1.0, 1.0]);
//             let result = MhaParser::parse_bytes(&mha_data);
//             assert!(result.is_ok(), "Failed to parse MHA with pixel type {:?}", pixel_type);

//             let volume = result.unwrap();
//             assert_eq!(volume.metadata.pixel_type, *pixel_type);
//         }
//     }

//     #[test]
//     fn test_parser_various_dimensions() {
//         let test_dimensions = [
//             [1, 1, 1],      // Minimal volume
//             [256, 256, 1],  // 2D-like volume
//             [64, 64, 64],   // Cubic volume
//             [512, 512, 100], // Typical CT dimensions
//         ];

//         for dims in &test_dimensions {
//             let mha_data = create_test_mha_data(*dims, PixelType::UInt8, [1.0, 1.0, 1.0]);
//             let result = MhaParser::parse_bytes(&mha_data);
//             assert!(result.is_ok(), "Failed to parse MHA with dimensions {:?}", dims);

//             let volume = result.unwrap();
//             assert_eq!(volume.metadata.dimensions, *dims);
//         }
//     }
// }

// // ============================================================================
// // Unit Tests - Validation System
// // ============================================================================

// #[cfg(test)]
// mod validation_tests {
//     use super::*;

//     #[test]
//     fn test_data_size_checker() {
//         let checker = DataSizeChecker::new(100, Some(1000));
        
//         // Test valid size
//         let valid_data = vec![0u8; 500];
//         let result = checker.check_integrity(&valid_data);
//         assert!(result.is_valid, "Valid data size should pass validation");

//         // Test too small
//         let small_data = vec![0u8; 50];
//         let result = checker.check_integrity(&small_data);
//         assert!(!result.is_valid, "Too small data should fail validation");

//         // Test too large
//         let large_data = vec![0u8; 2000];
//         let result = checker.check_integrity(&large_data);
//         assert!(!result.is_valid, "Too large data should fail validation");
//     }

//     #[test]
//     fn test_checksum_checker_simple() {
//         let expected_checksum = 12345u32;
//         let checker = ChecksumChecker::new(expected_checksum, ChecksumAlgorithm::Simple);
        
//         // Create data that matches expected checksum
//         let test_data = vec![1u8, 2, 3, 4, 5]; // Sum = 15, but we'll test with expected
//         let result = checker.check_integrity(&test_data);
//         // Note: This will likely fail unless data actually sums to expected_checksum
//         // This tests the checker logic, not necessarily a passing case
//         assert!(result.errors.len() > 0 || result.is_valid);
//     }

//     #[test]
//     fn test_medical_header_checker() {
//         let magic_bytes = vec![0x4D, 0x48, 0x41]; // "MHA"
//         let checker = MedicalHeaderChecker::new(magic_bytes.clone(), 256);
        
//         // Test valid header
//         let mut valid_data = magic_bytes.clone();
//         valid_data.extend(vec![0u8; 300]); // Ensure minimum size
//         let result = checker.check_integrity(&valid_data);
//         assert!(result.is_valid, "Valid header should pass validation");

//         // Test invalid magic
//         let invalid_data = vec![0x00, 0x01, 0x02, 0x03];
//         let result = checker.check_integrity(&invalid_data);
//         assert!(!result.is_valid, "Invalid magic bytes should fail validation");

//         // Test insufficient size
//         let small_data = vec![0x4D, 0x48, 0x41]; // Correct magic but too small
//         let result = checker.check_integrity(&small_data);
//         assert!(!result.is_valid, "Insufficient header size should fail validation");
//     }

//     #[test]
//     fn test_medical_image_validator() {
//         let mut validator = MedicalImageValidator::new();
        
//         // Add integrity checkers
//         let size_checker = DataSizeChecker::new(10, Some(1000));
//         validator.add_integrity_checker(Box::new(size_checker));
        
//         let magic_bytes = vec![0x4D, 0x48, 0x41];
//         let header_checker = MedicalHeaderChecker::new(magic_bytes, 100);
//         validator.add_integrity_checker(Box::new(header_checker));

//         // Test with valid data
//         let mut test_data = vec![0x4D, 0x48, 0x41]; // MHA magic
//         test_data.extend(vec![0u8; 200]); // Sufficient size
        
//         let result = validator.run_integrity_checks(&test_data);
//         assert!(result.is_valid || !result.errors.is_empty(), "Should run all registered checkers");
//     }

//     #[test]
//     fn test_validation_result_combination() {
//         let success = ValidationResult::success();
//         let failure = ValidationResult::failure(vec![ValidationError::InvalidInput {
//             message: "Test error".to_string(),
//             field: None,
//             context: HashMap::new(),
//         }]);

//         let combined = ValidationResult::combine(vec![success, failure]);
//         assert!(!combined.is_valid, "Combined result should be invalid if any input is invalid");
//         assert!(combined.errors.len() > 0, "Combined result should contain errors");
//     }
// }

// // ============================================================================
// // Unit Tests - Metadata and Data Structures
// // ============================================================================

// #[cfg(test)]
// mod metadata_tests {
//     use super::*;

//     #[test]
//     fn test_image_metadata_creation() {
//         let metadata = ImageMetadata {
//             dimensions: vec![256, 256, 128],
//             spacing: vec![1.0, 1.0, 2.0],
//             offset: vec![0.0, 0.0, 0.0],
//             orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
//             pixel_type: PixelType::UInt16,
//             endianness: Endianness::Little,
//             compression: None,
//             patient_position: PatientPosition::HFS,
//             data_offset: Some(1024),
//             element_data_file: "data.raw".to_string(),
//         };
        
//         assert_eq!(metadata.dimensions.len(), 3);
//         assert_eq!(metadata.pixel_type, PixelType::UInt16);
//         assert_eq!(metadata.endianness, Endianness::Little);
//     }

//     #[test]
//     fn test_pixel_data_from_bytes() {
//         let bytes = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
        
//         // Test UInt8
//         let pixel_data = PixelData::from_bytes(&bytes, PixelType::UInt8, Endianness::Little);
//         assert!(pixel_data.is_ok());
        
//         match pixel_data.unwrap() {
//             PixelData::UInt8(data) => assert_eq!(data, bytes),
//             _ => panic!("Expected UInt8 pixel data"),
//         }
//     }

//     #[test]
//     fn test_medical_volume_creation() {
//         let metadata = ImageMetadata {
//             dimensions: vec![2, 2, 2],
//             spacing: vec![1.0, 1.0, 1.0],
//             offset: vec![0.0, 0.0, 0.0],
//             orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
//             pixel_type: PixelType::UInt8,
//             endianness: Endianness::Little,
//             compression: None,
//             patient_position: PatientPosition::HFS,
//             data_offset: None,
//             element_data_file: "LOCAL".to_string(),
//         };
        
//         let pixel_data = PixelData::UInt8(vec![0u8; 8]);
        
//         let result = MedicalVolume::new(metadata, pixel_data, ImageFormat::MHA);
//         assert!(result.is_ok(), "Failed to create medical volume: {:?}", result.err());
        
//         let volume = result.unwrap();
//         assert_eq!(volume.source_format, ImageFormat::MHA);
//         assert_eq!(volume.metadata.dimensions, vec![2, 2, 2]);
//     }

//     #[test]
//     fn test_pixel_data_size_validation() {
//         let metadata = ImageMetadata {
//             dimensions: vec![2, 2, 2], // 8 pixels
//             spacing: vec![1.0, 1.0, 1.0],
//             offset: vec![0.0, 0.0, 0.0],
//             orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
//             pixel_type: PixelType::UInt8,
//             endianness: Endianness::Little,
//             compression: None,
//             patient_position: PatientPosition::HFS,
//             data_offset: None,
//             element_data_file: "LOCAL".to_string(),
//         };
        
//         // Correct size
//         let correct_data = PixelData::UInt8(vec![0u8; 8]);
//         let result = MedicalVolume::new(metadata.clone(), correct_data, ImageFormat::MHA);
//         assert!(result.is_ok(), "Should accept correct pixel data size");
        
//         // Incorrect size
//         let incorrect_data = PixelData::UInt8(vec![0u8; 16]);
//         let result = MedicalVolume::new(metadata, incorrect_data, ImageFormat::MHA);
//         assert!(result.is_err(), "Should reject incorrect pixel data size");
//     }
// }

// // ============================================================================
// // Edge Case and Error Handling Tests
// // ============================================================================

// #[cfg(test)]
// mod edge_case_tests {
//     use super::*;

//     #[test]
//     fn test_malformed_header_handling() {
//         let malformed_data = create_malformed_data("invalid_header");
//         let result = MhdParser::parse_single_file(&malformed_data);
//         assert!(result.is_err(), "Should reject malformed header");
        
//         if let Err(error) = result {
//             match error {
//                 MedicalImagingError::InvalidHeader { .. } => {
//                     // Expected error type
//                 },
//                 _ => panic!("Expected InvalidHeader error, got {:?}", error),
//             }
//         }
//     }

//     #[test]
//     fn test_missing_required_fields() {
//         let incomplete_data = create_malformed_data("missing_dimensions");
//         let result = MhdParser::parse_single_file(&incomplete_data);
//         assert!(result.is_err(), "Should reject incomplete header");
//     }

//     #[test]
//     fn test_invalid_pixel_type() {
//         let invalid_data = create_malformed_data("invalid_pixel_type");
//         let result = MhdParser::parse_single_file(&invalid_data);
//         assert!(result.is_err(), "Should reject invalid pixel type");
//     }

//     #[test]
//     fn test_truncated_data_handling() {
//         let truncated_data = create_malformed_data("truncated_data");
//         let result = MhaParser::parse_bytes(&truncated_data);
//         // This might succeed in parsing metadata but fail in data validation
//         // The exact behavior depends on implementation
//         if result.is_ok() {
//             let volume = result.unwrap();
//             // Validate that the volume detects the data corruption
//             let validation_result = volume.validate();
//             assert!(!validation_result.is_valid || validation_result.warnings.len() > 0,
//                    "Should detect data corruption");
//         }
//     }

//     #[test]
//     fn test_zero_dimensions() {
//         let header_data = create_test_mhd_header(
//             [0, 10, 10], // Zero dimension
//             PixelType::UInt8,
//             [1.0, 1.0, 1.0],
//             None
//         );

//         let result = MhdParser::parse_single_file(&header_data);
//         assert!(result.is_err(), "Should reject zero dimensions");
//     }

//     #[test]
//     fn test_negative_spacing() {
//         let mut header = String::from_utf8(create_test_mhd_header(
//             [10, 10, 10],
//             PixelType::UInt8,
//             [1.0, 1.0, 1.0],
//             None
//         )).unwrap();
        
//         // Replace spacing with negative values
//         header = header.replace("ElementSpacing = 1 1 1", "ElementSpacing = -1 1 1");
        
//         let result = MhdParser::parse_single_file(header.as_bytes());
//         // Implementation should handle this gracefully
//         if result.is_ok() {
//             let metadata = result.unwrap();
//             // Validation should catch negative spacing
//             assert!(metadata.spacing[0] < 0.0 || result.is_err());
//         }
//     }

//     #[test]
//     fn test_extremely_large_dimensions() {
//         let header_data = create_test_mhd_header(
//             [usize::MAX, 1, 1], // Extremely large dimension
//             PixelType::UInt8,
//             [1.0, 1.0, 1.0],
//             None
//         );

//         let result = MhdParser::parse_single_file(&header_data);
//         // Should either reject or handle gracefully
//         if result.is_ok() {
//             // If parsing succeeds, volume creation should fail due to memory constraints
//             let metadata = result.unwrap();
//             let pixel_data = PixelData::UInt8(vec![0u8; 100]); // Insufficient data
//             let volume_result = MedicalVolume::new(metadata, pixel_data, ImageFormat::MHD);
//             assert!(volume_result.is_err(), "Should reject volume with insufficient data for large dimensions");
//         }
//     }

//     #[test]
//     fn test_empty_data() {
//         let empty_data = vec![];
//         let result = MhdParser::parse_single_file(&empty_data);
//         assert!(result.is_err(), "Should reject empty data");

//         let result = MhaParser::parse_bytes(&empty_data);
//         assert!(result.is_err(), "Should reject empty data");
//     }

//     #[test]
//     fn test_unicode_in_header() {
//         let mut header = String::from("ObjectType = Image\n");
//         header.push_str("NDims = 3\n");
//         header.push_str("Comment = Test with unicode: 测试 🏥\n"); // Unicode characters
//         header.push_str("DimSize = 10 10 10\n");
//         header.push_str("ElementType = MET_UCHAR\n");
//         header.push_str("ElementDataFile = LOCAL\n");

//         let result = MhdParser::parse_single_file(header.as_bytes());
//         // Should handle unicode gracefully (either accept or reject consistently)
//         assert!(result.is_ok() || result.is_err()); // Just ensure it doesn't panic
//     }
// }

// // ============================================================================
// // Integration and System Tests
// // ============================================================================

// #[cfg(test)]
// mod integration_tests {
//     use super::*;

//     #[test]
//     fn test_end_to_end_mha_processing() {
//         // Create complete MHA file
//         let mha_data = create_test_mha_data([32, 32, 16], PixelType::Float32, [1.0, 1.0, 2.0]);
        
//         // Parse the file
//         let volume_result = MhaParser::parse_bytes(&mha_data);
//         assert!(volume_result.is_ok(), "Failed to parse MHA file");
        
//         let volume = volume_result.unwrap();
        
//         // Validate the volume
//         let validation_result = volume.validate();
//         assert!(validation_result.is_valid, "Volume validation failed: {:?}", validation_result.errors);
        
//         // Check all properties
//         assert_eq!(volume.metadata.dimensions, [32, 32, 16]);
//         assert_eq!(volume.metadata.pixel_type, PixelType::Float32);
//         assert_eq!(volume.metadata.spacing, [1.0, 1.0, 2.0]);
//         assert_eq!(volume.source_format, ImageFormat::MHA);
//     }

//     #[test]
//     fn test_end_to_end_mhd_processing() {
//         // Create MHD header
//         let mhd_data = create_test_mhd_header(
//             [64, 64, 32],
//             PixelType::Int16,
//             [0.5, 0.5, 1.0],
//             Some("test_data.raw")
//         );
        
//         // Parse metadata
//         let metadata_result = MhdParser::parse_single_file(&mhd_data);
//         assert!(metadata_result.is_ok(), "Failed to parse MHD metadata");
        
//         let metadata = metadata_result.unwrap();
        
//         // Create pixel data
//         let data_size = 64 * 64 * 32 * 2; // Int16 = 2 bytes per pixel
//         let pixel_data = PixelData::Int16(vec![0i16; 64 * 64 * 32]);
        
//         // Create volume
//         let volume_result = MedicalVolume::new(metadata, pixel_data, ImageFormat::MHD);
//         assert!(volume_result.is_ok(), "Failed to create medical volume");
        
//         let volume = volume_result.unwrap();
//         let validation_result = volume.run_integrity_checks();
//         assert!(validation_result.is_valid, "Volume validation failed: {:?}", validation_result.errors);
//     }

//     #[test]
//     fn test_format_detection_and_parsing() {
//         let formats_and_data = vec![
//             (ImageFormat::MHA, create_test_mha_data([8, 8, 8], PixelType::UInt8, [1.0, 1.0, 1.0])),
//             (ImageFormat::MHD, create_test_mhd_header([8, 8, 8], PixelType::UInt8, [1.0, 1.0, 1.0], None)),
//         ];

//         for (expected_format, data) in formats_and_data {
//             match expected_format {
//                 ImageFormat::MHA => {
//                     let result = MhaParser::parse_bytes(&data);
//                     assert!(result.is_ok(), "Failed to parse {:?} format", expected_format);
//                     let volume = result.unwrap();
//                     assert_eq!(volume.source_format, expected_format);
//                 },
//                 ImageFormat::MHD => {
//                     let result = MhdParser::parse_single_file(&data);
//                     assert!(result.is_ok(), "Failed to parse {:?} format", expected_format);
//                 },
//                 _ => {}
//             }
//         }
//     }

//     #[test]
//     fn test_cross_format_consistency() {
//         let dimensions = [16, 16, 8];
//         let pixel_type = PixelType::Float32;
//         let spacing = [1.5, 1.5, 3.0];

//         // Create same data in both formats
//         let mha_data = create_test_mha_data(dimensions, pixel_type, spacing);
//         let mhd_data = create_test_mhd_header(dimensions, pixel_type, spacing, None);

//         // Parse both
//         let mha_volume = MhaParser::parse_bytes(&mha_data).unwrap();
//         let mhd_metadata = MhdParser::parse_single_file(&mhd_data).unwrap();

//         // Compare metadata
//         assert_eq!(mha_volume.metadata.dimensions, mhd_metadata.dimensions);
//         assert_eq!(mha_volume.metadata.pixel_type, mhd_metadata.pixel_type);
//         assert_eq!(mha_volume.metadata.spacing, mhd_metadata.spacing);
//     }

//     #[test]
//     fn test_validation_system_integration() {
//         let mut validator = MedicalImageValidator::new();
        
//         // Add comprehensive validation
//         validator.add_integrity_checker(Box::new(DataSizeChecker::new(100, Some(10000))));
//         validator.add_integrity_checker(Box::new(MedicalHeaderChecker::new(
//             vec![0x4D, 0x48, 0x41], 256
//         )));

//         // Test with valid MHA data
//         let valid_data = create_test_mha_data([10, 10, 10], PixelType::UInt8, [1.0, 1.0, 1.0]);
//         let result = validator.run_integrity_checks(&valid_data);
        
//         // Should have results from all checkers
//         assert!(result.is_valid || !result.errors.is_empty(), "Should run integrity checks");
        
//         // Test with invalid data
//         let invalid_data = create_malformed_data("invalid_header");
//         let result = validator.run_integrity_checks(&invalid_data);
        
//         // Should detect issues
//         assert!(!result.is_valid || !result.errors.is_empty(), "Should detect validation failures");
//     }
// }

// // ============================================================================
// // Performance Tests
// // ============================================================================

// #[cfg(test)]
// mod performance_tests {
//     use super::*;
//     use std::time::Instant;

//     #[test]
//     fn test_large_volume_parsing_performance() {
//         // Create a reasonably large test volume
//         let dimensions = [256, 256, 64]; // ~4MB for UInt8
//         let mha_data = create_test_mha_data(dimensions, PixelType::UInt8, [1.0, 1.0, 1.0]);
        
//         let start = Instant::now();
//         let result = MhaParser::parse_bytes(&mha_data);
//         let parse_duration = start.elapsed();
        
//         assert!(result.is_ok(), "Failed to parse large volume");
//         assert!(parse_duration.as_secs() < 5, "Parsing took too long: {:?}", parse_duration);
        
//         println!("Large volume parsing took: {:?}", parse_duration);
//     }

//     #[test]
//     fn test_metadata_parsing_performance() {
//         let mha_data = create_test_mha_data([512, 512, 100], PixelType::Float32, [0.5, 0.5, 1.0]);
        
//         let start = Instant::now();
//         for _ in 0..100 {
//             let _ = MhaParser::parse_metadata_only(&mha_data);
//         }
//         let duration = start.elapsed();
        
//         assert!(duration.as_millis() < 1000, "Metadata parsing too slow: {:?}", duration);
//         println!("100 metadata parses took: {:?}", duration);
//     }

//     #[test]
//     fn test_validation_performance() {
//         let mut validator = MedicalImageValidator::new();
//         validator.add_integrity_checker(Box::new(DataSizeChecker::new(1000, Some(100000))));
//         validator.add_integrity_checker(Box::new(MedicalHeaderChecker::new(
//             vec![0x4D, 0x48, 0x41], 256
//         )));

//         let test_data = create_test_mha_data([128, 128, 32], PixelType::UInt8, [1.0, 1.0, 1.0]);
        
//         let start = Instant::now();
//         for _ in 0..50 {
//             let _ = validator.run_integrity_checks(&test_data);
//         }
//         let duration = start.elapsed();
        
//         assert!(duration.as_millis() < 2000, "Validation too slow: {:?}", duration);
//         println!("50 validation runs took: {:?}", duration);
//     }

//     #[test]
//     fn test_memory_usage_large_volumes() {
//         // Test that we can handle multiple large volumes without excessive memory usage
//         let mut volumes = Vec::new();
        
//         for i in 0..5 {
//             let dimensions = [64, 64, 64];
//             let mha_data = create_test_mha_data(dimensions, PixelType::UInt8, [1.0, 1.0, 1.0]);
            
//             let result = MhaParser::parse_bytes(&mha_data);
//             assert!(result.is_ok(), "Failed to parse volume {}", i);
            
//             volumes.push(result.unwrap());
//         }
        
//         // Verify all volumes are valid
//         for (i, volume) in volumes.iter().enumerate() {
//             assert_eq!(volume.metadata.dimensions, [64, 64, 64], "Volume {} has wrong dimensions", i);
//         }
        
//         println!("Successfully created and stored {} volumes", volumes.len());
//     }
// }

// // ============================================================================
// // WASM-Specific Tests
// // ============================================================================

// #[cfg(target_arch = "wasm32")]
// mod wasm_tests {
//     use super::*;

//     fn test_wasm_mha_parsing() {
//         let mha_data = create_test_mha_data([32, 32, 16], PixelType::UInt8, [1.0, 1.0, 1.0]);
//         let result = MhaParser::parse_bytes(&mha_data);
//         assert!(result.is_ok());
//     }

//     fn test_wasm_mhd_parsing() {
//         let mhd_data = create_test_mhd_header([32, 32, 16], PixelType::UInt8, [1.0, 1.0, 1.0], None);
//         let result = MhdParser::parse_single_file(&mhd_data);
//         assert!(result.is_ok());
//     }

//     fn test_wasm_validation() {
//         let mut validator = MedicalImageValidator::new();
//         validator.add_integrity_checker(Box::new(DataSizeChecker::new(100, Some(10000))));
        
//         let test_data = create_test_mha_data([16, 16, 8], PixelType::UInt8, [1.0, 1.0, 1.0]);
//         let result = validator.run_integrity_checks(&test_data);
//         assert!(result.is_valid || !result.errors.is_empty());
//     }
// }

// // ============================================================================
// // Benchmark Tests (Optional - requires criterion feature)
// // ============================================================================

// #[cfg(all(test, feature = "criterion"))]
// mod benchmarks {
//     use super::*;
//     use criterion::{black_box, criterion_group, criterion_main, Criterion};

//     fn bench_mha_parsing(c: &mut Criterion) {
//         let mha_data = create_test_mha_data([128, 128, 64], PixelType::Float32, [1.0, 1.0, 1.0]);
        
//         c.bench_function("mha_parse_large", |b| {
//             b.iter(|| {
//                 let result = MhaParser::parse_bytes(black_box(&mha_data));
//                 black_box(result)
//             })
//         });
//     }

//     fn bench_validation(c: &mut Criterion) {
//         let mut validator = MedicalImageValidator::new();
//         validator.add_integrity_checker(Box::new(DataSizeChecker::new(1000, Some(100000))));
        
//         let test_data = create_test_mha_data([64, 64, 32], PixelType::UInt8, [1.0, 1.0, 1.0]);
        
//         c.bench_function("validation_comprehensive", |b| {
//             b.iter(|| {
//                 let result = validator.run_integrity_checks(black_box(&test_data));
//                 black_box(result)
//             })
//         });
//     }

//     criterion_group!(benches, bench_mha_parsing, bench_validation);
//     criterion_main!(benches);
// }