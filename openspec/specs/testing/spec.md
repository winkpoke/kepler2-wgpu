# Testing Specification

## Purpose

Provides comprehensive testing infrastructure, fixtures, and validation for medical imaging software. Ensures patient safety, data integrity, and rendering correctness through systematic test coverage.

## Requirements

### Requirement: Test Fixture Infrastructure

The system SHALL provide reusable test fixtures for generating valid and invalid test data across all medical imaging formats.

#### Scenario: Generating Test DICOM Data
- **WHEN** a test needs valid DICOM CT image data
- **THEN** the `create_test_ct_image()` helper SHALL generate a complete DICOM with all mandatory fields populated
- **AND** the generated image SHALL be parseable by `CTImage::from_bytes()`
- **AND** it SHALL include valid UID, dimensions, pixel data, and metadata

#### Scenario: Creating Malformed DICOM Data
- **WHEN** a test needs to validate error handling
- **THEN** helper functions SHALL provide malformed DICOM data (missing fields, corrupted headers, invalid tags)
- **AND** each malformed variant SHALL produce predictable, documented parse errors

#### Scenario: Generating Volume Test Data
- **WHEN** a test needs a CTVolume with specific dimensions
- **THEN** the `create_test_volume_512x512x100()` helper SHALL generate a volume with exact dimensions
- **AND** it SHALL include valid voxel spacing and orientation metadata
- **AND** voxel data SHALL match the dimensions (512 * 512 * 100 voxels)

### Requirement: DICOM Validation Tests

All DICOM parsing functions SHALL validate mandatory fields and reject invalid inputs to prevent patient safety issues.

#### Scenario: Missing Mandatory SOP Instance UID
- **WHEN** a DICOM file is missing the SOPInstanceUID tag (0008,0018)
- **THEN** `CTImage::from_bytes()` SHALL return a `Result::Err`
- **AND** the error message SHALL explicitly indicate "Missing SOPInstanceUID"
- **AND** the system SHALL NOT proceed to parsing other fields

#### Scenario: Missing Mandatory Pixel Representation
- **WHEN** a DICOM file is missing the PixelRepresentation tag (0028,0103)
- **THEN** `CTImage::from_bytes()` SHALL return a `Result::Err`
- **AND** the error SHALL indicate that pixel representation is required for correct Hounsfield unit calculation

#### Scenario: Invalid Rescale Slope (Zero or Negative)
- **WHEN** a DICOM file has rescale_slope = 0.0
- **THEN** the system SHALL either reject it with clear error OR apply it correctly without division by zero
- **AND** if negative rescale_slope is provided, it SHALL invert Hounsfield units as expected
- **AND** the transformation SHALL preserve numerical precision

#### Scenario: Missing Rescale Parameters
- **WHEN** a DICOM file lacks both RescaleSlope and RescaleIntercept
- **THEN** `CTImage::get_pixel_data()` SHALL use default values (slope=1.0, intercept=0.0)
- **AND** this SHALL NOT cause panics or crashes
- **AND** log messages SHALL indicate default values were used

### Requirement: Patient Metadata Validation

Patient identification and study/series metadata SHALL be validated to ensure patient safety and data integrity.

#### Scenario: Empty or Missing Patient ID
- **WHEN** a Patient record is created with empty or missing ID
- **THEN** the Patient struct SHALL be rejected during creation
- **OR** empty ID SHALL return validation error when queried
- **AND** the error SHALL indicate that patient ID is mandatory

#### Scenario: UID Uniqueness
- **WHEN** multiple Patient, Study, or Series records are created
- **THEN** each SHALL have a unique UID following DICOM standard format (2.25.x)
- **AND** the UID generation function SHALL NOT produce collisions
- **AND** tests SHALL verify uniqueness across 1000+ generated UIDs

#### Scenario: Study-Series Hierarchy Integrity
- **WHEN** an ImageSeries is created
- **THEN** its SeriesInstanceUID SHALL match the parent Study's StudyInstanceUID
- **AND** when images are added to a series, their SeriesInstanceUID SHALL match the series
- **AND** mismatched UIDs SHALL cause validation errors

### Requirement: Coordinate Transformation Safety

All coordinate transformations (world ↔ screen ↔ voxel) SHALL preserve precision and prevent invalid states that could display wrong anatomy.

#### Scenario: MPR Slice Position Out of Bounds
- **WHEN** an MPR view attempts to set slice position < 0 or > volume depth
- **THEN** the position SHALL be clamped to valid range [0, max_depth]
- **AND** a warning SHALL be logged indicating the clamping occurred
- **AND** the view SHALL NOT crash or enter invalid state

#### Scenario: Orientation Matrix Orthogonality
- **WHEN** an orientation matrix is created for axial, coronal, or sagittal view
- **THEN** the matrix SHALL be orthogonal (determinant = ±1)
- **AND** basis vectors SHALL be normalized (unit length)
- **AND** tests SHALL verify orthogonality with epsilon tolerance (|det - 1| < 0.01)

#### Scenario: World-Voxel Coordinate Roundtrip
- **WHEN** a world coordinate is transformed to voxel space and back to world space
- **THEN** the resulting coordinate SHALL match the original within floating-point precision
- **AND** error accumulation SHALL be less than 0.001 world units for valid ranges
- **AND** precision SHALL be preserved across multiple transformations

### Requirement: Medical Format Parsing (MHA/MHD)

MHA and MHD file parsers SHALL validate headers, detect corruption, and handle all supported pixel types and endianness.

#### Scenario: Missing Mandatory MHA Header Fields
- **WHEN** an MHA file is missing ObjectType, NDims, DimSize, or ElementType
- **THEN** the parser SHALL return a `MedicalImagingResult::Err`
- **AND** the error SHALL specify which mandatory field is missing
- **AND** the parser SHALL NOT proceed to data extraction

#### Scenario: MHD External Data File Resolution
- **WHEN** an MHD file references an external data file via ElementDataFile
- **THEN** the path SHALL be resolved relative to the MHD file location
- **AND** if the external file is missing, a clear file-not-found error SHALL be returned
- **AND** the error message SHALL include the expected file path

#### Scenario: Endianness Handling
- **WHEN** an MHA/MHD file specifies big-endian or little-endian byte order
- **THEN** the parser SHALL apply appropriate byte-swapping based on the specified endianness
- **AND** pixel data SHALL be correctly interpreted regardless of source endianness
- **AND** tests SHALL verify correct interpretation using known values

#### Scenario: Unsupported Pixel Type
- **WHEN** an MHA/MHD file specifies an unsupported ElementType
- **THEN** the parser SHALL return an `MedicalImagingResult::Err`
- **AND** the error SHALL list the unsupported type and supported types
- **AND** the system SHALL NOT attempt to parse pixel data

### Requirement: Volume Data Integrity

CTVolumed volumes SHALL validate dimensions, spacing, and data size to prevent memory corruption or incorrect rendering.

#### Scenario: Invalid Volume Dimensions
- **WHEN** a CTVolume is created with dimensions (0, 0, 0) or negative values
- **THEN** the volume SHALL be rejected or marked as invalid
- **AND** clear error messages SHALL indicate dimension constraints
- **AND** the system SHALL NOT attempt memory allocation

#### Scenario: Zero or Negative Voxel Spacing
- **WHEN** voxel spacing values (mm per voxel) are 0.0 or negative
- **THEN** the volume SHALL be rejected
- **AND** the error SHALL indicate that spacing must be positive
- **AND** this SHALL prevent division-by-zero errors in coordinate calculations

#### Scenario: Voxel Data Size Mismatch
- **WHEN** a CTVolume has dimensions (512, 512, 100) but voxel_data length ≠ 512*512*100
- **THEN** the volume SHALL be rejected
- **AND** the error SHALL specify expected and actual data sizes
- **AND** this SHALL prevent indexing errors during rendering

### Requirement: Property-Based Testing for Mathematical Correctness

Mathematical functions (window/level, coordinate transformations, clamping) SHALL be verified through property-based tests to ensure correctness across all input ranges.

#### Scenario: Window/Level Midpoint Property
- **FOR ALL** valid window width (-1000 to 2000) and level (-1000 to 1000) pairs
- **THEN** the transformation at level SHALL equal window / 2 (midpoint)
- **AND** property SHALL hold: `transform(level) = window / 2.0`
- **AND** tolerance SHALL be < 0.01 Hounsfield units

#### Scenario: Scale Clamping Bounds
- **FOR ALL** scale values in realistic range (0.0 to 200.0)
- **THEN** clamped scale SHALL be in range [0.01, 100.0]
- **AND** values at bounds SHALL equal bounds
- **AND** values within bounds SHALL remain unchanged

#### Scenario: Matrix Determinant for Rotations
- **FOR ALL** rotation angles (0 to 6.28 radians, full 360°)
- **THEN** rotation matrices SHALL have determinant ≈ 1 (orthogonal transformation)
- **AND** absolute error SHALL be < 0.01
- **AND** this SHALL preserve volume and handedness

### Requirement: GPU Pipeline Safety

GPU initialization, pipeline creation, and resource management SHALL handle failures gracefully and ensure proper cleanup.

#### Scenario: GPU Device Creation Failure
- **WHEN** WGPU device cannot be created (no compatible GPU available)
- **THEN** the system SHALL return a clear `KeplerResult::Err`
- **AND** the error SHALL indicate the specific GPU limitation
- **AND** the application SHALL NOT crash or enter undefined state
- **AND** this SHALL only apply to native builds (#[cfg(not(target_arch = "wasm32"))])

#### Scenario: Shader Compilation Error
- **WHEN** a WGSL shader fails to compile due to syntax or type errors
- **THEN** the pipeline creation SHALL fail with a detailed error
- **AND** the error SHALL include the shader stage (vertex/fragment) and line number if available
- **AND** the system SHALL NOT proceed to rendering

#### Scenario: Texture Memory Limits Exceeded
- **WHEN** attempting to create a texture larger than GPU memory limits
- **THEN** the creation SHALL fail with a clear memory allocation error
- **AND** the error SHALL specify the requested dimensions and estimated memory
- **AND** no GPU resources SHALL be leaked

### Requirement: View Manager State Consistency

View manager SHALL maintain consistent state across concurrent operations and ensure proper resource cleanup.

#### Scenario: Concurrent View Add/Remove Operations
- **WHEN** multiple threads add and remove views simultaneously
- **THEN** the view count SHALL always be accurate
- **AND** race conditions SHALL NOT cause view count errors
- **AND** removed views SHALL be fully cleaned up (no memory leaks)
- **AND** active view pointer SHALL remain valid

#### Scenario: View State Restoration
- **WHEN** switching between different layouts (grid vs single view)
- **THEN** the view manager SHALL restore view state correctly
- **AND** slice positions, pan, and scale SHALL be preserved
- **AND** orientation and window/level settings SHALL be maintained
- **AND** no data loss SHALL occur

### Requirement: Visual Correctness

Rendering output SHALL maintain visual fidelity through proper window/level application, aspect ratio preservation, and clamping.

#### Scenario: Window/Level Clamping
- **WHEN** user specifies window center outside valid Hounsfield range (-1024 to +3071)
- **THEN** the center SHALL be clamped to valid range
- **AND** a warning SHALL be logged indicating the clamping
- **AND** the displayed contrast SHALL remain meaningful

#### Scenario: Aspect Ratio Preservation
- **WHEN** fitting a 512×512 source into a 1920×1080 destination (letterbox)
- **THEN** the fit SHALL preserve the source aspect ratio (1:1)
- **AND** black bars SHALL be added to sides or top/bottom as appropriate
- **AND** source image SHALL NOT be stretched

#### Scenario: Aspect Fit Exact Match
- **WHEN** source and destination have identical aspect ratios
- **THEN** the fit SHALL use full destination resolution
- **AND** no padding or cropping SHALL occur
- **AND** every pixel from source SHALL map to destination

### Requirement: Error Propagation and User-Friendly Messages

All error paths SHALL propagate context and provide actionable messages for debugging and user feedback.

#### Scenario: File I/O Error Context
- **WHEN** a DICOM file read fails (not found, permission denied)
- **THEN** the error SHALL include the full file path
- **AND** the error SHALL indicate the specific I/O operation that failed
- **AND** system-specific error codes SHALL be preserved (ENOENT, EACCES)

#### Scenario: Parse Error with Tag Information
- **WHEN** DICOM tag parsing fails
- **THEN** the error SHALL include the tag identifier (e.g., "(0028,0010) Rows")
- **AND** the error SHALL indicate what value was expected vs what was found
- **AND** this SHALL aid in debugging malformed DICOM files

#### Scenario: GPU Allocation Error with Size
- **WHEN** GPU buffer/texture allocation fails
- **THEN** the error SHALL specify the requested size in bytes/dimensions
- **AND** the error SHALL indicate if the size exceeds known GPU limits
- **AND** this SHALL help diagnose memory pressure issues

### Requirement: Edge Case Handling

The system SHALL handle boundary conditions, degenerate inputs, and stress cases without crashing or producing undefined behavior.

#### Scenario: Single-Slice Volume
- **WHEN** a CTVolume has dimensions (512, 512, 1)
- **THEN** the system SHALL process it correctly as a degenerate 2D case
- **AND** all coordinate transformations SHALL work with z-depth = 1
- **AND** rendering SHALL NOT crash

#### Scenario: Maximum Dimensions Volume
- **WHEN** a CTVolume is created with maximum dimensions (2048×2048×2048)
- **THEN** the system SHALL handle memory allocation without overflow
- **AND** if allocation fails, it SHALL fail gracefully with clear OOM error
- **AND** partial volumes SHALL NOT be created

#### Scenario: Empty DICOM Series
- **WHEN** an ImageSeries is created with 0 images
- **THEN** the series SHALL be in a valid empty state
- **AND** iteration over the series SHALL yield no images
- **AND** metadata SHALL still be accessible (UID, modality)

### Requirement: Performance Benchmarks

Critical operations SHALL have performance benchmarks with defined targets to ensure acceptable frame rates and response times.

#### Scenario: DICOM Parsing Performance
- **WHEN** parsing a 512×512 DICOM image
- **THEN** the operation SHALL complete in < 10 milliseconds
- **AND** performance SHALL be measured and compared against baseline
- **AND** regressions SHALL be detected if time increases > 50%

#### Scenario: Volume Rendering Performance
- **WHEN** rendering a single frame of 512×512×100 volume
- **THEN** the frame SHALL render in < 16 milliseconds (≥ 60 FPS)
- **AND** this SHALL include MPR slice extraction and upload to GPU
- **AND** performance SHALL be tracked across runs

#### Scenario: Memory Usage Limits
- **WHEN** loading and rendering large volumes
- **THEN** total memory usage SHALL remain < 2GB for typical 512³ volume
- **AND** memory SHALL be measured and logged
- **AND** leaks SHALL be detected across repeated load/unload cycles

### Requirement: Regression Test Suite

Every bug fix SHALL have a corresponding regression test to prevent recurrence.

#### Scenario: Adding Regression Test
- **WHEN** a bug is fixed (tracked in GitHub Issue #XXX)
- **THEN** a regression test SHALL be added following naming: `regression_issue_XXX_brief_description`
- **AND** the test SHALL reproduce the original bug
- **AND** the test SHALL verify the fix prevents the bug
- **AND** the test SHALL reference the issue number in documentation

#### Scenario: Regression Test Validation
- **WHEN** regression tests are added
- **THEN** each test SHALL have clear documentation of the bug
- **AND** the test SHALL be added to `tests/regression_tests.rs`
- **AND** the test SHALL run on all future PRs to prevent regression
- **AND** CI SHALL fail if the regression test fails

### Requirement: Test Coverage Targets

The system SHALL maintain minimum coverage percentages across different modules to ensure patient safety and code quality.

#### Scenario: Medical Path Coverage Target
- **WHEN** measuring code coverage for critical medical paths
- **THEN** coverage SHALL be ≥ 80% for:
  - DICOM parsing (patient safety)
  - Patient metadata (identity)
  - Coordinate transformations (display correctness)
- **AND** tests SHALL explicitly validate these paths
- **AND** CI SHALL fail if coverage drops below 80%

#### Scenario: Overall Coverage Targets by Phase
- **WHEN** Phase 1 is complete (Week 2)
- **THEN** overall coverage SHALL be ≥ 45%
- **WHEN** Phase 2 is complete (Week 4)
- **THEN** overall coverage SHALL be ≥ 60%
- **WHEN** Phase 3 is complete (Week 6)
- **THEN** overall coverage SHALL be ≥ 55%
- **WHEN** Phase 4 is complete (Week 8)
- **THEN** overall coverage SHALL be ≥ 70%
- **AND** progress SHALL be tracked via coverage reports

### Requirement: CI/CD Coverage Gating

CI/CD pipeline SHALL enforce coverage targets and prevent regressions.

#### Scenario: Coverage Gate Failure
- **WHEN** a pull request is submitted and critical medical path coverage < 80%
- **THEN** the CI pipeline SHALL fail
- **AND** the failure SHALL indicate which module is below threshold
- **AND** the PR SHALL NOT be mergable until coverage improves

#### Scenario: Coverage Trend Detection
- **WHEN** coverage is measured on successive builds
- **THEN** significant decreases (> 5% absolute) SHALL trigger warnings
- **AND** the CI SHALL report which files/lines lost coverage
- **AND** this SHALL enable early detection of coverage regressions
