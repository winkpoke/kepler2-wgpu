//! CT volume and DICOM image fixtures

use glam::Mat4;
use kepler_wgpu::core::coord::Base;
use kepler_wgpu::data::ct_volume::CTVolume;
use kepler_wgpu::data::medical_imaging::PixelType;

/// Create dummy pixel data for testing
pub fn create_dummy_pixel_data(num_pixels: usize) -> String {
    format!("{:0width$}", "", width = num_pixels * 2)
}

/// Create a simple test CTVolume for testing coordinate systems
pub fn create_test_ct_volume() -> CTVolume {
    CTVolume {
        dimensions: (512, 512, 100),
        voxel_spacing: (1.0, 1.0, 2.0),
        voxel_data: vec![0i16; 512 * 512 * 100],
        base: Base {
            label: "test_volume".to_string(),
            matrix: Mat4::IDENTITY,
        },
    }
}

/// Create MHA test data header with embedded data
pub fn create_mha_test_data(
    dimensions: [usize; 3],
    pixel_type: PixelType,
    spacing: [f64; 3],
    _verbose: bool,
) -> String {
    let _data_size = dimensions[0] * dimensions[1] * dimensions[2];
    let header = format!(
        "ObjectType = Image
NDims = 3
BinaryData = True
BinaryDataByteOrderMSB = False
CompressedData = False
TransformMatrix = 1 0 0 0 0 0 1
Offset = 0 0 0
CenterOfRotation = 0 0 0
ElementSpacing = {} {} {}
DimSize = {} {} {}
ElementType = {}
ElementDataFile = LOCAL
",
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
    );

    let element_data = format!("{:0width$} ", "", width = dimensions[0]);

    let full_data = format!("{}{}", header, element_data);

    full_data
}

/// Create MHD header file for testing
pub fn create_mhd_header_file(
    dimensions: [usize; 3],
    pixel_type: PixelType,
    spacing: [f64; 3],
) -> String {
    let data_type = match pixel_type {
        PixelType::UInt8 => "MET_UCHAR",
        PixelType::UInt16 => "MET_USHORT",
        PixelType::Int16 => "MET_SHORT",
        PixelType::Int32 => "MET_INT",
        PixelType::Float32 => "MET_FLOAT",
        PixelType::Float64 => "MET_DOUBLE",
    };

    format!(
        "ObjectType = Image
NDims = 3
DimSize = {} {} {}
ElementSpacing = {} {} {}
ElementType = {}
ElementDataFile = CT_512x512x100.raw
",
        dimensions[0], dimensions[1], dimensions[2], spacing[0], spacing[1], spacing[2], data_type
    )
}
