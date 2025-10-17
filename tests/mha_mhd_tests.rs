//! Comprehensive test suite for MHA/MHD medical imaging functionality
//! 
//! This test suite validates all critical paths, edge cases, and error handling scenarios
//! for the medical imaging module, ensuring robust functionality across different platforms
mod mha_mhd_tests{
    use std::time::Instant;
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
        if verbose{
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
        }
        
        header
    }

    // ============================================================================
    // Unit Tests - Metadata and Data Structures
    // ============================================================================
    
    #[test]
    fn test_pixel_data_from_bytes() {
        let bytes = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
        
        // Test UInt8
        let pixel_data = PixelData::from_bytes(&bytes, PixelType::UInt8);
        assert!(pixel_data.is_ok());
        
        match pixel_data.unwrap() {
            PixelData::UInt8(data) => assert_eq!(data, bytes),
            _ => panic!("Expected UInt8 pixel data"),
        }
    }

    #[test]
    fn test_create_pixel_data_int16() {
        // Test Int16 pixel type
        let raw_data = vec![
            0x00, 0x01, // 256 in little endian
            0xFF, 0x7F, // 32767 in little endian
            0x00, 0x80, // -32768 in little endian
            0x00, 0xFC, // -1024 in little endian
        ];
        let voxel_count = 4;
        let slope = 1.0;
        let intercept = 0.0;

        let result = PixelData::create_pixel_data(
            raw_data,
            PixelType::Int16,
            voxel_count,
            slope,
            intercept,
        );

        assert!(result.is_ok());
        let voxel_data = result.unwrap();
        assert_eq!(voxel_data.len(), 4);
        assert_eq!(voxel_data[0], 256);
        assert_eq!(voxel_data[1], 32767);
        assert_eq!(voxel_data[2], -1024);
        assert_eq!(voxel_data[3], -1024);
    }

    #[test]
    fn test_create_pixel_data_float32() {
        // Test Float32 pixel type with slope and intercept
        let val1 = 100.5f32;
        let val2 = -50.25f32;
        let val3 = 0.0f32;
        let val4 = -2000.0f32;

        let mut raw_data = Vec::new();
        raw_data.extend_from_slice(&val1.to_le_bytes());
        raw_data.extend_from_slice(&val2.to_le_bytes());
        raw_data.extend_from_slice(&val3.to_le_bytes());
        raw_data.extend_from_slice(&val4.to_le_bytes());

        let voxel_count = 4;
        let slope = 2.0;
        let intercept = 10.0;

        let result = PixelData::create_pixel_data(
            raw_data,
            PixelType::Float32,
            voxel_count,
            slope,
            intercept,
        );

        assert!(result.is_ok());
        let voxel_data = result.unwrap();
        assert_eq!(voxel_data.len(), 4);
        
        // val1: (100.5 * 2.0 + 10.0).round() = 211
        assert_eq!(voxel_data[0], 211);
        // val2: (-50.25 * 2.0 + 10.0).round() = -91
        assert_eq!(voxel_data[1], -91);
        // val3: (0.0 * 2.0 + 10.0).round() = 10
        assert_eq!(voxel_data[2], 10);
        // val4: (-2000.0 * 2.0 + 10.0).round() = -3990, clamped to -1024
        assert_eq!(voxel_data[3], -1024);
    }

    #[test]
    fn test_create_pixel_data_clamping() {
        // Test that values below -1024 are clamped
        let raw_data = vec![
            0x00, 0x80, // -32768 in little endian
            0x01, 0x80, // -32767 in little endian
            0x00, 0xFC, // -1024 in little endian (should not be clamped)
            0xFF, 0xFB, // -1025 in little endian (should be clamped to -1024)
        ];
        let voxel_count = 4;
        let slope = 1.0;
        let intercept = 0.0;

        let result = PixelData::create_pixel_data(
            raw_data,
            PixelType::Int16,
            voxel_count,
            slope,
            intercept,
        );

        assert!(result.is_ok());
        let voxel_data = result.unwrap();
        assert_eq!(voxel_data.len(), 4);
        assert_eq!(voxel_data[0], -1024); // Clamped from -32768
        assert_eq!(voxel_data[1], -1024); // Clamped from -32767
        assert_eq!(voxel_data[2], -1024); // Not clamped, already -1024
        assert_eq!(voxel_data[3], -1024); // Clamped from -1025
    }

    #[test]
    fn test_create_pixel_data_voxel_count_limit() {
        // Test that only voxel_count elements are processed
        let raw_data = vec![
            0x01, 0x00, // 1 in little endian
            0x02, 0x00, // 2 in little endian
            0x03, 0x00, // 3 in little endian
            0x04, 0x00, // 4 in little endian
        ];
        let voxel_count = 2; // Only process first 2 voxels
        let slope = 1.0;
        let intercept = 0.0;

        let result = PixelData::create_pixel_data(
            raw_data,
            PixelType::Int16,
            voxel_count,
            slope,
            intercept,
        );

        assert!(result.is_ok());
        let voxel_data = result.unwrap();
        assert_eq!(voxel_data.len(), 2); // Should only have 2 elements
        assert_eq!(voxel_data[0], 1);
        assert_eq!(voxel_data[1], 2);
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

    // ============================================================================
    // Performance Tests
    // ============================================================================

    #[test]
    fn test_large_volume_parsing_performance() {
        // Create a reasonably large test volume
        let dimensions = [256, 256, 64]; // ~4MB for UInt8
        let mha_data = create_test_mha_data(dimensions, PixelType::UInt8, [1.0, 1.0, 1.0], true);
        
        let start = Instant::now();
        let result = MhaParser::parse_bytes(&mha_data);
        let parse_duration = start.elapsed();
        
        assert!(result.is_ok(), "Failed to parse large volume");
        assert!(parse_duration.as_secs() < 5, "Parsing took too long: {:?}", parse_duration);
        
        println!("Large volume parsing took: {:?}", parse_duration);
    }

    #[test]
    fn test_metadata_parsing_performance() {
        let mha_data = create_test_mha();
        let mhd_data = create_test_mhd().0;
        
        let start = Instant::now();
        for _ in 0..100 {
            let _ = MhaParser::parse_metadata_only(&mha_data);
        }
        let duration_mha = start.elapsed();

        let start = Instant::now();
        for _ in 0..100 {
            let _ = MhdParser::parse_metadata_only(&mhd_data);
        }
        let duration_mhd = start.elapsed();
        
        assert!(duration_mha.as_millis() < 1000, "Metadata parsing too slow: {:?}", duration_mha);
        println!("100 metadata parses took(mha): {:?}", duration_mha);
        assert!(duration_mhd.as_millis() < 1000, "Metadata parsing too slow: {:?}", duration_mhd);
        println!("100 metadata parses took(mhd): {:?}", duration_mhd);
    }

    #[test]
    fn test_validation_performance() {
        let mut validator = MedicalImageValidator::new();
        validator.add_integrity_checker(Box::new(DataSizeChecker::new(1000, Some(100000))));
        validator.add_integrity_checker(Box::new(MedicalHeaderChecker::new(
            vec![0x4D, 0x48, 0x41], 256
        )));

        let test_data = create_test_mha();
        
        let start = Instant::now();
        for _ in 0..50 {
            let _ = validator.run_integrity_checks(&test_data);
        }
        let duration = start.elapsed();
        
        assert!(duration.as_millis() < 2000, "Validation too slow: {:?}", duration);
        println!("50 validation runs took: {:?}", duration);
    }

    #[test]
    fn test_memory_usage_large_volumes() {
        // Test that we can handle multiple large volumes without excessive memory usage
        let mut volumes = Vec::new();
        
        for i in 0..5 {
            let dimensions = [64, 64, 64];
            let mha_data = create_test_mha_data(dimensions, PixelType::UInt8, [1.0, 1.0, 1.0], true);
            
            let result = MhaParser::parse_bytes(&mha_data);
            assert!(result.is_ok(), "Failed to parse volume {}", i);
            
            volumes.push(result.unwrap());
        }
        
        // Verify all volumes are valid
        for (i, volume) in volumes.iter().enumerate() {
            assert_eq!(volume.metadata.dimensions, [64, 64, 64], "Volume {} has wrong dimensions", i);
        }
        
        println!("Successfully created and stored {} volumes", volumes.len());
    }

    // ============================================================================
    // Edge Case and Error Handling Tests
    // ============================================================================

    #[test]
    fn test_create_pixel_data_unsupported_type() {
        let raw_data = vec![0x01, 0x02, 0x03, 0x04];
        let voxel_count = 1;
        let slope = 1.0;
        let intercept = 0.0;

        let result = PixelData::create_pixel_data(
            raw_data,
            PixelType::UInt8, // Unsupported type
            voxel_count,
            slope,
            intercept,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            MedicalImagingError::UnsupportedPixelType { pixel_type } => {
                assert!(pixel_type.contains("UInt8"));
            }
            _ => panic!("Expected UnsupportedPixelType error"),
        }
    }

        #[test]
    fn test_create_pixel_data_empty_input() {
        let raw_data = Vec::new();
        let voxel_count = 0;
        let slope = 1.0;
        let intercept = 0.0;

        let result = PixelData::create_pixel_data(
            raw_data,
            PixelType::Int16,
            voxel_count,
            slope,
            intercept,
        );

        assert!(result.is_ok());
        let voxel_data = result.unwrap();
        assert_eq!(voxel_data.len(), 0);
    }
}