//! MHA/MHD format test data fixtures

use kepler_wgpu::data::medical_imaging::PixelType;

/// Create MHA test data with configurable parameters
pub fn create_mha_test_data(
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
    );

    let data_size = dimensions[0] * dimensions[1] * dimensions[2];
    let pixel_size = match pixel_type {
        PixelType::UInt8 => 1,
        PixelType::UInt16 => 2,
        PixelType::Int16 => 2,
        PixelType::Int32 => 4,
        PixelType::Float32 => 4,
        PixelType::Float64 => 8,
    };

    let data: Vec<u8> = vec![0; data_size * pixel_size];

    let mut mha_data = header.into_bytes();
    mha_data.extend_from_slice(b"\n\n");
    mha_data.extend_from_slice(&data);

    mha_data
}

/// Create MHD test files (header + data file)
pub fn create_mhd_test_files(
    dimensions: [usize; 3],
    compressed: bool,
) -> (String, Vec<u8>, Vec<u8>) {
    let header = format!(
        "ObjectType = Image\n\
         NDims = 3\n\
         DimSize = {} {} {}\n\
         ElementSpacing = 1.0 1.0 2.0\n\
         ElementType = MET_SHORT\n\
         ElementDataFile = {}\n",
        dimensions[0],
        dimensions[1],
        dimensions[2],
        if compressed {
            "data.raw.gz"
        } else {
            "data.raw"
        }
    );

    let data = vec![0u8; dimensions[0] * dimensions[1] * dimensions[2] * 2];

    (header, data, vec![])
}
