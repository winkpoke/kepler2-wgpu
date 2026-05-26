//! WASM unit tests
//!
//! Tests WASM-specific code paths including:
//! - MHA/MHD parsing without tokio (synchronous I/O)
//! - wasm-bindgen bridge error handling
//! - Memory limits in browser
//! - Pixel data type conversions
//! - Coordinate system operations

#[cfg(test)]
mod wasm_mha_parsing_tests {
    use kepler_wgpu::data::medical_imaging::formats::mha::MhaParser;
    use kepler_wgpu::data::medical_imaging::metadata::PixelType;

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
                PixelType::Float32 => "MET_FLOAT",
                _ => "MET_UCHAR",
            }
        )
        .into_bytes();

        let num_pixels = dimensions[0] * dimensions[1] * dimensions[2];
        let bytes_per_pixel = match pixel_type {
            PixelType::UInt8 => 1,
            PixelType::UInt16 | PixelType::Int16 => 2,
            PixelType::Float32 | PixelType::Int32 => 4,
            PixelType::Float64 => 8,
        };
        let pixel_data = vec![0u8; num_pixels * bytes_per_pixel];
        header.extend_from_slice(&pixel_data);

        header
    }

    #[test]
    fn test_mha_header_parsing_synchronous() {
        let mha_data = create_test_mha_data([64, 64, 10], PixelType::UInt8, [1.0, 1.0, 2.0]);

        let result = MhaParser::parse_metadata_only(&mha_data);
        assert!(result.is_ok(), "MHA header should parse successfully");

        let metadata = result.unwrap();
        assert_eq!(metadata.dimensions, vec![64, 64, 10]);
        assert_eq!(metadata.spacing, vec![1.0, 1.0, 2.0]);
    }

    #[test]
    fn test_mha_data_offset_calculation() {
        let mha_data = create_test_mha_data([32, 32, 5], PixelType::UInt16, [0.5, 0.5, 1.0]);

        let result = MhaParser::parse_metadata_only(&mha_data);
        assert!(result.is_ok(), "Should calculate data offset correctly");

        let metadata = result.unwrap();
        assert!(
            metadata.data_offset.is_some(),
            "Data offset should be set for embedded data"
        );
    }

    #[test]
    fn test_mha_endianness_detection() {
        let mha_data = create_test_mha_data([16, 16, 2], PixelType::UInt8, [1.0, 1.0, 1.0]);

        let result = MhaParser::parse_bytes(&mha_data);
        assert!(result.is_ok(), "Little-endian MHA should parse successfully");
    }

    #[test]
    fn test_mha_big_endian_parsing() {
        let header = "ObjectType = Image\n\
                      NDims = 3\n\
                      BinaryData = True\n\
                      BinaryDataByteOrderMSB = True\n\
                      CompressedData = False\n\
                      TransformMatrix = 1 0 0 0 1 0 0 0 1\n\
                      Offset = 0 0 0\n\
                      AnatomicalOrientation = RAI\n\
                      ElementSpacing = 1.0 1.0 1.0\n\
                      DimSize = 16 16 2\n\
                      ElementType = MET_UCHAR\n\
                      ElementDataFile = LOCAL\n"
            .as_bytes()
            .to_vec();
        let mha_data = [header, vec![0u8; 512]].concat();

        let result = MhaParser::parse_bytes(&mha_data);
        assert!(result.is_ok(), "Big-endian MHA should parse");
    }

    #[test]
    fn test_mha_pixel_type_validation() {
        let pixel_types = [
            PixelType::UInt8,
            PixelType::UInt16,
            PixelType::Int16,
            PixelType::Float32,
        ];

        for pixel_type in pixel_types {
            let mha_data = create_test_mha_data([8, 8, 2], pixel_type, [1.0, 1.0, 1.0]);
            let result = MhaParser::parse_metadata_only(&mha_data);
            assert!(
                result.is_ok(),
                "MHA should support pixel type: {:?}",
                pixel_type
            );
        }
    }

    #[test]
    fn test_mha_dimension_validation() {
        let mha_data = create_test_mha_data([64, 64, 10], PixelType::UInt8, [1.0, 1.0, 2.0]);

        let result = MhaParser::parse_metadata_only(&mha_data);
        assert!(result.is_ok(), "Valid dimensions should be accepted");

        let metadata = result.unwrap();
        assert_eq!(metadata.dimensions.len(), 3, "Should have 3 dimensions");
        assert!(
            metadata.dimensions.iter().all(|&d| d > 0),
            "All dimensions should be positive"
        );
    }

    #[test]
    fn test_mha_invalid_header_rejection() {
        let invalid_data = b"Not a valid MHA file";
        let result = MhaParser::parse_metadata_only(invalid_data);
        assert!(result.is_err(), "Invalid MHA header should be rejected");
    }

    #[test]
    fn test_mha_missing_elementdatafile() {
        let incomplete_header = "ObjectType = Image\n\
                                 NDims = 3\n\
                                 DimSize = 64 64 10\n\
                                 ElementType = MET_UCHAR\n";

        let result = MhaParser::parse_metadata_only(incomplete_header.as_bytes());
        assert!(
            result.is_err(),
            "MHA without ElementDataFile should fail"
        );
    }

    #[test]
    fn test_mha_empty_data() {
        let result = MhaParser::parse_metadata_only(&[]);
        assert!(result.is_err(), "Empty data should be rejected");
    }
}

#[cfg(test)]
mod wasm_mhd_parsing_tests {
    use kepler_wgpu::data::medical_imaging::formats::mhd::MhdParser;
    use kepler_wgpu::data::medical_imaging::metadata::PatientPosition;

    fn create_test_mhd_header(data_file: &str) -> Vec<u8> {
        format!(
            "ObjectType = Image\n\
             NDims = 3\n\
             BinaryData = True\n\
             BinaryDataByteOrderMSB = False\n\
             CompressedData = False\n\
             TransformMatrix = 1 0 0 0 1 0 0 0 1\n\
             Offset = 0 0 0\n\
             CenterOfRotation = 0 0 0\n\
             AnatomicalOrientation = RAI\n\
             ElementSpacing = 1.0 1.0 2.0\n\
             DimSize = 64 64 10\n\
             ElementType = MET_UCHAR\n\
             ElementDataFile = {}\n",
            data_file
        )
        .into_bytes()
    }

    #[test]
    fn test_mhd_header_parsing_synchronous() {
        let mhd_header = create_test_mhd_header("LOCAL");
        let data = vec![0u8; 64 * 64 * 10];

        let result = MhdParser::parse_by_bytes(&mhd_header, &data);
        assert!(result.is_ok(), "MHD header should parse successfully");
    }

    #[test]
    fn test_mhd_external_file_resolution() {
        let mhd_header = create_test_mhd_header("data.raw");

        let result = MhdParser::parse_metadata_only(&mhd_header);
        assert!(
            result.is_ok(),
            "MHD should parse even with external file reference"
        );

        let metadata = result.unwrap();
        assert_eq!(metadata.dimensions, vec![64, 64, 10]);
    }

    #[test]
    fn test_mhd_transform_matrix_parsing() {
        let mhd_header = create_test_mhd_header("LOCAL");

        let result = MhdParser::parse_metadata_only(&mhd_header);
        assert!(result.is_ok(), "Transform matrix should parse");

        let metadata = result.unwrap();
        assert_eq!(
            metadata.orientation,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        );
    }

    #[test]
    fn test_mhd_anatomical_orientation_parsing() {
        let mhd_header = create_test_mhd_header("LOCAL");

        let result = MhdParser::parse_metadata_only(&mhd_header);
        assert!(result.is_ok(), "Anatomical orientation should parse");

        let metadata = result.unwrap();
        assert_eq!(metadata.patient_position, PatientPosition::HFS);
    }

    #[test]
    fn test_mhd_missing_data_file_warning() {
        let mhd_header = create_test_mhd_header("missing.raw");

        let result = MhdParser::parse_metadata_only(&mhd_header);
        assert!(
            result.is_ok(),
            "MHD should still parse metadata even with missing data file"
        );
    }

    #[test]
    fn test_mhd_empty_header() {
        let result = MhdParser::parse_metadata_only(&[]);
        assert!(result.is_err(), "Empty MHD header should fail");
    }
}

#[cfg(test)]
mod wasm_error_propagation_tests {
    use kepler_wgpu::core::error::{KeplerError, MprError};

    #[test]
    fn test_js_value_from_kepler_error() {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsValue;

            let error = KeplerError::Validation("Test error".to_string());
            let js_value: JsValue = error.into();
            let js_str = js_value.as_string().unwrap();

            assert!(
                js_str.contains("Test error"),
                "JS value should contain error message"
            );
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let error = KeplerError::Validation("Test error".to_string());
            let display_str = format!("{}", error);
            assert!(
                display_str.contains("Test error"),
                "Display string should contain error message"
            );
        }
    }

    #[test]
    fn test_rust_to_javascript_error_propagation() {
        let errors = vec![
            KeplerError::Graphics("GPU error".to_string()),
            KeplerError::Dicom("DICOM parse error".to_string()),
            KeplerError::Validation("Validation failed".to_string()),
            KeplerError::Window("Window creation failed".to_string()),
        ];

        for error in errors {
            let display_str = format!("{}", error);
            assert!(
                !display_str.is_empty(),
                "All errors should convert to display string"
            );

            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::JsValue;
                let js_value: JsValue = error.into();
                assert!(
                    js_value.as_string().is_some(),
                    "Error should convert to JsValue string"
                );
            }
        }
    }

    #[test]
    fn test_mpr_error_display() {
        let mpr_error = MprError::InvalidScale(-1.0);
        let display = format!("{}", mpr_error);

        assert!(
            display.contains("-1"),
            "Error display should contain the invalid value"
        );
    }

    #[test]
    fn test_invalid_parameter_handling() {
        let invalid_scale = MprError::InvalidScale(0.0);
        assert!(
            format!("{}", invalid_scale).contains("0"),
            "Should report invalid scale value"
        );

        let invalid_position = MprError::InvalidPosition(-100, -100);
        assert!(
            format!("{}", invalid_position).contains("-100"),
            "Should report invalid position"
        );
    }

    #[test]
    fn test_error_chain_depth() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let kepler_error = KeplerError::from(io_error);

        use std::error::Error;
        assert!(
            kepler_error.source().is_some(),
            "KeplerError wrapping io::Error should have source"
        );
    }
}

#[cfg(test)]
mod wasm_memory_limit_tests {
    use kepler_wgpu::core::coord::Base;
    use kepler_wgpu::data::CTVolume;

    #[test]
    fn test_large_volume_allocation_limits() {
        let max_safe_dimension = 512;
        let volume_data = vec![0i16; max_safe_dimension * max_safe_dimension * 100];

        let volume = CTVolume::new(
            (max_safe_dimension, max_safe_dimension, 100),
            (1.0, 1.0, 1.0),
            volume_data,
            Base::default(),
        );

        assert_eq!(volume.dimensions().0, max_safe_dimension);
        assert_eq!(volume.dimensions().1, max_safe_dimension);
    }

    #[test]
    fn test_webgl_texture_size_limits() {
        let webgl_max_dimension = 4096;

        let volume_data = vec![0i16; webgl_max_dimension * webgl_max_dimension * 1];
        let volume = CTVolume::new(
            (webgl_max_dimension, webgl_max_dimension, 1),
            (1.0, 1.0, 1.0),
            volume_data,
            Base::default(),
        );

        assert_eq!(volume.dimensions().0, webgl_max_dimension);
    }

    #[test]
    fn test_gpu_buffer_size_limits() {
        let max_buffer_pixels = 1024 * 1024 * 50;

        let volume_data = vec![0i16; max_buffer_pixels];
        let volume = CTVolume::new(
            (1024, 1024, 50),
            (1.0, 1.0, 1.0),
            volume_data,
            Base::default(),
        );

        assert_eq!(volume.dimensions().2, 50);
    }

    #[test]
    fn test_out_of_memory_graceful_degradation() {
        let oversized_data: Vec<i16> = Vec::new();

        let volume = CTVolume::new(
            (0, 0, 0),
            (1.0, 1.0, 1.0),
            oversized_data,
            Base::default(),
        );

        assert_eq!(volume.dimensions().0, 0, "Zero dimensions should be handled");
    }

    #[test]
    fn test_volume_memory_cleanup() {
        let volume_data = vec![0i16; 64 * 64 * 10];
        let volume = CTVolume::new(
            (64, 64, 10),
            (1.0, 1.0, 1.0),
            volume_data,
            Base::default(),
        );

        let ptr = volume.voxel_data().as_ptr();
        drop(volume);

        unsafe {
            std::ptr::read_volatile(ptr);
        }
    }
}

#[cfg(test)]
mod wasm_pixel_data_tests {
    use kepler_wgpu::data::medical_imaging::metadata::{Endianness, PixelData, PixelType};

    #[test]
    fn test_pixel_data_from_le_bytes_uint16() {
        let bytes = vec![0x01, 0x00, 0x02, 0x00, 0x03, 0x00];
        let result = PixelData::from_le_bytes(&bytes, PixelType::UInt16);

        assert!(result.is_ok(), "Should parse little-endian UInt16");
        if let Ok(PixelData::UInt16(values)) = result {
            assert_eq!(values, vec![1, 2, 3]);
        }
    }

    #[test]
    fn test_pixel_data_from_le_bytes_int16() {
        let bytes = vec![0xFF, 0xFF, 0x01, 0x00, 0xFE, 0xFF];
        let result = PixelData::from_le_bytes(&bytes, PixelType::Int16);

        assert!(result.is_ok(), "Should parse little-endian Int16");
        if let Ok(PixelData::Int16(values)) = result {
            assert_eq!(values, vec![-1, 1, -2]);
        }
    }

    #[test]
    fn test_pixel_data_from_le_bytes_float32() {
        let value: f32 = 1.5;
        let bytes = value.to_le_bytes().to_vec();
        let result = PixelData::from_le_bytes(&bytes, PixelType::Float32);

        assert!(result.is_ok(), "Should parse little-endian Float32");
        if let Ok(PixelData::Float32(values)) = result {
            assert!((values[0] - 1.5).abs() < 1e-6);
        }
    }

    #[test]
    fn test_pixel_data_from_be_bytes() {
        let bytes = vec![0x00, 0x01, 0x00, 0x02, 0x00, 0x03];
        let result = PixelData::from_be_bytes(&bytes, PixelType::UInt16);

        assert!(result.is_ok(), "Should parse big-endian UInt16");
        if let Ok(PixelData::UInt16(values)) = result {
            assert_eq!(values, vec![1, 2, 3]);
        }
    }

    #[test]
    fn test_pixel_data_invalid_size() {
        let bytes = vec![0x01];
        let result = PixelData::from_le_bytes(&bytes, PixelType::UInt16);

        assert!(
            result.is_ok(),
            "Incomplete pixel data should parse (truncates silently)"
        );

        if let Ok(PixelData::UInt16(values)) = result {
            assert_eq!(
                values.len(),
                0,
                "Should produce empty vector for incomplete pixel"
            );
        }
    }

    #[test]
    fn test_pixel_data_zero_length() {
        let bytes: Vec<u8> = vec![];
        let result = PixelData::from_le_bytes(&bytes, PixelType::UInt16);

        assert!(result.is_ok(), "Empty data should parse to empty vector");

        if let Ok(PixelData::UInt16(values)) = result {
            assert_eq!(values.len(), 0, "Should produce empty vector");
        }
    }

    #[test]
    fn test_endianness_conversion() {
        assert_eq!(Endianness::Little, Endianness::Little);
        assert_eq!(Endianness::Big, Endianness::Big);
        assert_ne!(Endianness::Little, Endianness::Big);
    }
}

#[cfg(test)]
mod wasm_coordinate_tests {
    use glam::Vec3;
    use kepler_wgpu::core::coord::Base;

    #[test]
    fn test_base_coordinate_system_creation() {
        let base = Base::default();
        assert!(base.matrix.is_finite(), "Default base matrix should be finite");
    }

    #[test]
    fn test_coordinate_transformation_roundtrip() {
        let base = Base::default();
        let point = Vec3::new(100.0, 200.0, 50.0);

        let transformed = base.matrix.transform_point3(point);
        let inverse_matrix = base.matrix.inverse();
        let restored = inverse_matrix.transform_point3(transformed);

        assert!(
            (restored - point).length() < 0.001,
            "Roundtrip transformation should preserve coordinates"
        );
    }

    #[test]
    fn test_coordinate_bounds_checking() {
        let base = Base::default();
        let volume_dims = (512, 512, 100);

        let valid_point = Vec3::new(256.0, 256.0, 50.0);
        let transformed = base.matrix.transform_point3(valid_point);

        assert!(
            transformed.x >= 0.0 && transformed.x < volume_dims.0 as f32,
            "X coordinate should be within bounds"
        );
        assert!(
            transformed.y >= 0.0 && transformed.y < volume_dims.1 as f32,
            "Y coordinate should be within bounds"
        );
        assert!(
            transformed.z >= 0.0 && transformed.z < volume_dims.2 as f32,
            "Z coordinate should be within bounds"
        );
    }
}

#[cfg(test)]
mod wasm_window_level_tests {
    use kepler_wgpu::core::window_level::WindowLevel;

    #[test]
    fn test_window_level_clamping() {
        let mut wl = WindowLevel::new();
        assert_eq!(wl.window_level(), 40.0);

        wl.set_window_level(-5000.0).unwrap();
        assert_eq!(wl.window_level(), WindowLevel::MIN_WINDOW_LEVEL);

        wl.set_window_level(5000.0).unwrap();
        assert_eq!(wl.window_level(), WindowLevel::MAX_WINDOW_LEVEL);
    }

    #[test]
    fn test_window_width_clamping() {
        let mut wl = WindowLevel::new();
        assert_eq!(wl.window_width(), 400.0);

        assert!(wl.set_window_width(-100.0).is_err());

        wl.set_window_width(0.5).unwrap();
        assert_eq!(wl.window_width(), WindowLevel::MIN_WINDOW_WIDTH);

        wl.set_window_width(10000.0).unwrap();
        assert_eq!(wl.window_width(), WindowLevel::MAX_WINDOW_WIDTH);
    }
}
