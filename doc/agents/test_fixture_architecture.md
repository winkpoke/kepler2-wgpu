# Test Fixture Architecture

## Overview

Test fixtures provide reusable test data and helper functions to reduce duplication, ensure consistency, and improve maintainability across the test suite.

## Fixture Location

### Central Module: `tests/common/mod.rs`

All fixtures are centralized in a single module to prevent duplication and ensure all tests use the same data generation logic.

```rust
// tests/common/mod.rs
pub mod fixtures;

// Re-export all fixtures for convenience
pub use fixtures::*;

pub use fixtures::ct_volume::{
    create_test_ct_image,
    create_test_volume_512x512x100,
    create_dicom_with_invalid_rescale,
    create_invalid_dicom_missing_uid,
};

pub use fixtures::patient::{create_test_patient, create_invalid_patient};

pub use fixtures::format::{create_mha_test_data, create_mhd_test_files};
```

## Fixture Categories

### CT Image Fixtures

**File**: `tests/common/fixtures/ct_volume.rs`

```rust
use crate::data::dicom::CTImage;

/// Create valid DICOM CT image with all mandatory fields
pub fn create_test_ct_image() -> CTImage {
    CTImage {
        sop_instance_uid: "1.2.840.113619.2.55.3.603610938272815658.20190101.120000.1".to_string(),
        series_instance_uid: "1.2.840.113619.2.55.3.603610938272815658.20190101.120000".to_string(),
        rows: 512,
        columns: 512,
        pixel_representation: 1, // Signed
        bits_allocated: 16,
        bits_stored: 16,
        pixel_data: vec![0i16; 512 * 512],
        rescale_slope: 1.0,
        rescale_intercept: -1024.0,
        patient_name: "Test^Patient".to_string(),
        // ... all other mandatory fields
    }
}

/// Create invalid DICOM with missing SOPInstanceUID
pub fn create_invalid_dicom_missing_uid() -> CTImage {
    let mut img = create_test_ct_image();
    img.sop_instance_uid = String::new(); // Missing UID
    img
}

/// Create DICOM with invalid rescale slope (zero)
pub fn create_dicom_with_invalid_rescale() -> CTImage {
    let mut img = create_test_ct_image();
    img.rescale_slope = 0.0; // Invalid: division by zero
    img
}
```

### Patient Fixtures

**File**: `tests/common/fixtures/patient.rs`

```rust
use crate::data::dicom::Patient;

/// Create test patient with configurable ID
pub fn create_test_patient(id: &str) -> Patient {
    Patient {
        id: id.to_string(),
        name: "Test^Patient^Middle^Prefix^Suffix".to_string(),
        birth_date: "19800101", // YYYYMMDD
        sex: "M", // Male
        // ... other fields
    }
}

/// Create patient with empty ID (invalid)
pub fn create_invalid_patient() -> Patient {
    let mut patient = create_test_patient("");
    patient.id = String::new(); // Empty ID is invalid
    patient
}
```

### Volume Fixtures

**File**: `tests/common/fixtures/volume.rs`

```rust
use crate::data::ct_volume::CTVolume;

/// Create test CT volume 512x512x100
pub fn create_test_volume_512x512x100() -> CTVolume {
    CTVolume {
        dimensions: (512, 512, 100),
        voxel_spacing: (1.0, 1.0, 2.0), // 1mm x 1mm x 2mm
        origin: (0.0, 0.0, 0.0),
        orientation: crate::rendering::Orientation::Axial,
        data: vec![0i16; 512 * 512 * 100],
    }
}
```

### Format Fixtures

**File**: `tests/common/fixtures/format.rs`

```rust
/// Create MHA test data with configurable parameters
pub fn create_mha_test_data(
    dimensions: (usize, usize, usize),
    endianness: Endianness,
    compression: bool,
) -> Vec<u8> {
    let mut header = String::new();
    header.push_str("ObjectType = Image\n");
    header.push_str(&format!("NDims = {}\n", dimensions.2.len()));
    header.push_str(&format!("DimSize = {} {} {}\n", dimensions.0, dimensions.1, dimensions.2));
    header.push_str(&format!("ElementSpacing = 1.0 1.0 2.0\n"));
    header.push_str("ElementType = MET_SHORT\n");
    header.push_str(&format!("ElementByteOrderMSB = {}\n", endianness == Endianness::Big));

    let pixel_data: Vec<u8> = vec![0; dimensions.0 * dimensions.1 * dimensions.2 * 2];

    let mut mha_data = header.into_bytes();
    mha_data.extend_from_slice(b"\n\n");
    mha_data.extend_from_slice(&pixel_data);

    mha_data
}

/// Create MHD test files (header + data file)
pub fn create_mhd_test_files(
    dimensions: (usize, usize, usize),
    compressed: bool,
) -> (String, Vec<u8>, Vec<u8>) {
    let header = format!(
        "ObjectType = Image\n\
         NDims = 3\n\
         DimSize = {} {} {}\n\
         ElementSpacing = 1.0 1.0 2.0\n\
         ElementType = MET_SHORT\n\
         ElementDataFile = {}\n",
        dimensions.0, dimensions.1, dimensions.2,
        if compressed { "data.raw.gz" } else { "data.raw" }
    );

    let data = vec![0u8; dimensions.0 * dimensions.1 * dimensions.2 * 2];

    (header, data, vec![]) // Returns header, raw_data, compressed_data
}
```

## Fixture Export Policy

### Re-Export Fixtures

All fixtures are re-exported from `tests/common/mod.rs` to enable convenient imports:

```rust
// In test files:
use tests::common::*;

// This imports: create_test_ct_image, create_test_volume_512x512x100, etc.
```

### Prevent Duplication

**Rule**: All test data generation MUST use fixtures from `tests/common/mod.rs`

**Audit Process**:
1. Search for local fixture definitions: `rg "create_test_" tests/`
2. Refactor local fixtures to use common module
3. Update imports to use `use tests::common::*`

**Example**:
```rust
// BEFORE (duplication):
#[test]
fn test_dicom_parsing() {
    let ct_image = CTImage {
        sop_instance_uid: "1.2.840...".to_string(),
        rows: 512,
        columns: 512,
        // ... 50 lines of setup
    };
}

// AFTER (using fixture):
#[test]
fn test_dicom_parsing() {
    let ct_image = create_test_ct_image();
    // Modify only what's needed for this test
    let mut ct_image = create_test_ct_image();
    ct_image.rows = 256; // Test smaller image
}
```

## Fixture Versioning

### Directory Structure

```
tests/fixtures/
├── v001/
│   ├── valid_dicom.dcm          # Valid DICOM with all mandatory fields
│   ├── invalid_uid.dcm          # Invalid UID format
│   ├── malformed_pixel_data.dcm # Truncated pixel data
│   └── README.md               # Fixture descriptions
├── v002/
│   ├── valid_dicom.dcm          # Updated DICOM spec version
│   └── ...
└── manifest.toml               # Fixture metadata
```

### Manifest Format

**File**: `tests/fixtures/manifest.toml`

```toml
[[fixture]]
version = "v001"
name = "valid_dicom"
file = "valid_dicom.dcm"
description = "Valid DICOM with all mandatory fields"
tags = ["baseline", "ct", "mandatory-fields"]
used_by_tests = [
    "dicom_validation_critical_tests.rs::test_missing_sop_instance_uid",
    "dicom_validation_critical_tests.rs::test_valid_dicom",
]

[[fixture]]
version = "v001"
name = "invalid_uid"
file = "invalid_uid.dcm"
description = "Invalid UID format (non-numeric characters)"
tags = ["rejection", "uid", "validation"]
used_by_tests = [
    "dicom_validation_critical_tests.rs::test_invalid_uid_format",
]
```

### Versioning Strategy

1. **Increment version** when DICOM spec changes or test requirements evolve
2. **Keep old versions** for regression tests that depend on specific formats
3. **Document changes** in `vXXX/README.md`
4. **Update manifest** to reflect new fixtures

## Fixture Creation Guidelines

### DICOM Fixtures

```bash
# Use dcm4che to create valid DICOM
java -jar dcm4che-tool-dcm2dcm \
  --out-file tests/fixtures/v001/valid_dicom.dcm \
  --transfer-syntax 1.2.840.10008.1.2 \
  --patient-id TEST123 \
  --patient-name "Test^Patient" \
  --study-date 20240101 \
  --series-date 20240101
```

### Synthetic Data

```rust
// Generate synthetic pixel data
pub fn generate_synthetic_pixels(rows: usize, cols: usize) -> Vec<i16> {
    (0..rows * cols)
        .map(|i| (i % 4096) as i16 - 2048) // Hounsfield units: -2048 to +2047
        .collect()
}
```

### Malformed Data

```rust
// Create malformed DICOM for rejection tests
pub fn create_malformed_dicom() -> Vec<u8> {
    let mut data = create_test_ct_image().to_bytes();
    data.truncate(data.len() / 2); // Truncate last half
    data
}
```

## Fixture Usage Examples

### DICOM Validation Tests

```rust
use tests::common::*;

#[test]
fn test_missing_sop_instance_uid_rejected() {
    let invalid_dicom = create_invalid_dicom_missing_uid();
    let result = CTImage::from_bytes(&invalid_dicom.to_bytes());
    assert!(result.is_err());
    assert!(matches!(result, Err(DicomError::MissingField("SOPInstanceUID")));
}
```

### Volume Integrity Tests

```rust
use tests::common::*;

#[test]
fn test_volume_dimensions_validation() {
    let mut volume = create_test_volume_512x512x100();
    volume.dimensions = (0, 512, 100); // Invalid: zero dimension

    let result = CTVolume::validate(&volume);
    assert!(result.is_err());
}
```

### Patient Safety Tests

```rust
use tests::common::*;

#[test]
fn test_patient_id_required() {
    let invalid_patient = create_invalid_patient();
    assert!(invalid_patient.id.is_empty());

    let result = Patient::validate(&invalid_patient);
    assert!(result.is_err());
}
```

## Fixture Maintenance

### Adding New Fixtures

1. Create fixture function in `tests/common/fixtures/`
2. Export from `tests/common/mod.rs`
3. Add to `tests/fixtures/manifest.toml` if file-based
4. Document usage in `tests/common/fixtures/README.md`

### Updating Existing Fixtures

1. Update fixture function implementation
2. Increment version if breaking change
3. Update all test files that use the fixture
4. Run full test suite: `cargo test`
5. Update fixture documentation

### Removing Fixtures

1. Check usage with `rg "create_test_*" tests/`
2. Remove fixture function
3. Update `tests/common/mod.rs` exports
4. Remove from `tests/fixtures/manifest.toml`
5. Remove test files that are no longer needed

## References

- Rust testing patterns: https://doc.rust-lang.org/book/ch11-03-test-organization.html
- DICOM conformance statements: http://medical.nema.org/medical/dicom/current/output/chtml/part14/chapter_A.html
- Fixture anti-patterns: https://testing.googleblog.com/2015/01/testing-on-toilet-shared-fixtures.html
