## MODIFIED Requirements

### Requirement: Medical Safety Critical Testing
The system SHALL provide comprehensive test coverage for medical safety critical paths including DICOM parsing, patient metadata validation, and coordinate transformations to ensure patient safety and data integrity.

#### Scenario: DICOM Mandatory Field Validation
- **WHEN** DICOM files are parsed for patient-critical data
- **THEN** the system SHALL validate all mandatory DICOM fields (SOPInstanceUID, SeriesInstanceUID, Rows, Columns, PixelRepresentation, PixelData)
- **AND** SHALL reject DICOM files with missing mandatory fields with clear error messages
- **AND** SHALL provide DICOM tag information in error messages (tag ID, VR type)

#### Scenario: DICOM Tag Format Validation
- **WHEN** DICOM files are parsed
- **THEN** the system SHALL validate DICOM tag formats including VR (Value Representation) types (UL, SH, DS, FL, etc.)
- **AND** SHALL validate transfer syntax (explicit vs implicit VR)
- **AND** SHALL detect and reject VR mismatches
- **AND** SHALL handle private DICOM tags (odd group/element numbers) correctly
- **AND** SHALL validate tag sequences (nested DICOM groups)
- **AND** SHALL achieve ≥ 85% coverage on DICOM parsing code

#### Scenario: DICOM Pixel Data Edge Cases
- **WHEN** DICOM pixel data is processed
- **THEN** the system SHALL handle odd-sized pixel data (not divisible by 2) correctly
- **AND** SHALL detect endianness mismatches (big-endian vs little-endian)
- **AND** SHALL reject truncated pixel data (incomplete last chunk)
- **AND** SHALL prevent overflow in rescaling (pixel value × slope exceeds i16 range)
- **AND** SHALL preserve precision (test rounding errors in transformed_value.round() as i16)
- **AND** SHALL handle negative rescale slope correctly (inverts units)
- **AND** SHALL handle zero rescale slope correctly
- **AND** SHALL achieve ≥ 75% coverage on pixel data processing code

#### Scenario: Patient Metadata Validation
- **WHEN** patient metadata is parsed
- **THEN** the system SHALL validate patient name format (Surname^GivenName^MiddleName^Prefix^Suffix per DICOM standard)
- **AND** SHALL validate birth date format (YYYYMMDD, no future dates)
- **AND** SHALL validate sex codes (M, F, O only)
- **AND** SHALL enforce patient ID character limits
- **AND** SHALL reject empty patient IDs
- **AND** SHALL achieve ≥ 75% coverage on patient metadata code

#### Scenario: Patient Identity and Study Series Integrity
- **WHEN** patient, study, series, and image relationships are validated
- **THEN** the system SHALL ensure Series UID matches study's SeriesInstanceUID
- **AND** SHALL enforce Image SOPInstanceUID uniqueness within series
- **AND** SHALL validate study/series/patient hierarchy
- **AND** SHALL reject adding image to non-existent series
- **AND** SHALL detect orphaned series
- **AND** SHALL achieve ≥ 70% coverage on patient.rs and studyset.rs

#### Scenario: UID Format and Uniqueness Validation
- **WHEN** UIDs are generated or validated
- **THEN** the system SHALL enforce valid UID format (DICOM 2.25 + ISO OID syntax)
- **AND** SHALL enforce max UID length (64 characters)
- **AND** SHALL validate UID characters (0-9 digits and periods only)
- **AND** SHALL enforce UID uniqueness across study/series/images
- **AND** SHALL reject invalid UID formats with clear error messages
- **AND** SHALL achieve ≥ 80% coverage on UID generation and validation code

### Requirement: Coordinate Transformation Safety
The system SHALL provide comprehensive test coverage for coordinate transformation safety, precision, and correctness to ensure accurate medical display and prevent anatomical distortion.

#### Scenario: Coordinate Roundtrip Precision
- **WHEN** coordinate transformations are tested
- **THEN** the system SHALL verify `screen_coord_to_world()` and `world_coord_to_screen()` roundtrip accuracy
- **AND** SHALL enforce roundtrip error < 0.001 mm tolerance (as specified in medical imaging standards)
- **AND** SHALL test large coordinate values (>10000 mm)
- **AND** SHALL test NaN propagation through transformations
- **AND** SHALL test infinity propagation through transformations
- **AND** SHALL achieve ≥ 70% coverage on mpr_view.rs coordinate transformation methods

#### Scenario: Coordinate Transformation Input Validation
- **WHEN** MPR view slice position is set
- **THEN** the system SHALL validate that slice position is finite (not NaN or infinite)
- **AND** SHALL validate that transformation scale factors are finite and non-zero
- **AND** SHALL return appropriate errors for invalid inputs (MprError::InvalidSlicePosition, MprError::InvalidTransformation)
- **AND** SHALL NOT clamp slice position (only validate and return error)
- **AND** SHALL achieve ≥ 60% coverage on MprView input validation methods

#### Scenario: Matrix Orthogonality and Determinants
- **WHEN** orientation matrices are tested for axial, coronal, sagittal views
- **THEN** the system SHALL verify row vectors are orthogonal (dot products = 0 ± epsilon)
- **AND** SHALL verify column vectors are orthogonal (dot products = 0 ± epsilon)
- **AND** SHALL verify determinant = 1.0 for rotation matrices (± epsilon)
- **AND** SHALL verify row vectors are unit length (1.0 ± epsilon)
- **AND** SHALL verify column vectors are unit length (1.0 ± epsilon)
- **AND** SHALL achieve ≥ 60% coverage on Orientation::build_base() methods

#### Scenario: Voxel Spacing Validation
- **WHEN** CT volumes are validated
- **THEN** the system SHALL require voxel spacing values be positive
- **AND** SHALL reject zero voxel spacing
- **AND** SHALL reject negative voxel spacing
- **AND** SHALL validate realistic spacing bounds (< 0.001 mm is too small, > 100 mm is too large)
- **AND** SHALL detect anisotropic spacing (>10x difference between axes)
- **AND** SHALL validate spacing consistency across slices in series
- **AND** SHALL achieve ≥ 70% coverage on ct_volume.rs

### Requirement: Property-Based Testing
The system SHALL provide property-based testing using `proptest` with specific property definitions, input ranges, and expected invariants to comprehensively test mathematical and geometric operations.

#### Scenario: Window/Level Property Testing
- **WHEN** window/level operations are tested with proptest
- **THEN** the system SHALL test monotonicity property (increasing window level increases displayed brightness)
- **AND** SHALL test invertibility property (window level + bias transformation is reversible)
- **AND** SHALL test bounds preservation property (effective_level always in [MIN_WINDOW_LEVEL, MAX_WINDOW_LEVEL])
- **AND** SHALL test range preservation property (output values always in [0.0, 1.0] after normalization)
- **AND** SHALL use appropriate input ranges (window_width: [1.0, 4096.0], window_level: [-2048.0, 2048.0], bias: [-2048.0, 2048.0])
- **AND** SHALL execute 100+ random test cases per property
- **AND** SHALL achieve ≥ 60% coverage on WindowLevel code

#### Scenario: Coordinate Transformation Property Testing
- **WHEN** coordinate transformation operations are tested with proptest
- **THEN** the system SHALL test matrix determinant property (det(rotation) = 1.0 ± epsilon)
- **AND** SHALL test scale clamping property (scale always in [MIN_SCALE, MAX_SCALE] = [0.01, 100.0])
- **AND** SHALL test pan distance property (pan always clamped to ±MAX_PAN_DISTANCE = ±10000.0 mm)
- **AND** SHALL test aspect ratio preservation property (aspect_fit maintains content proportions)
- **AND** SHALL use appropriate input ranges (scale: [0.001, 1000.0], coordinates: [-50000.0, 50000.0])
- **AND** SHALL execute 100+ random test cases per property
- **AND** SHALL achieve ≥ 65% coverage on coordinate transformation code

### Requirement: GPU and Rendering Testing
The system SHALL provide comprehensive test coverage for GPU resource management, texture operations, shader compilation, and rendering correctness with appropriate strategies for untestable native-only code.

#### Scenario: Texture Upload Bounds and Safety
- **WHEN** textures are created and uploaded to GPU
- **THEN** the system SHALL test upload size validation against GPU max texture size limits
- **AND** SHALL test format compatibility checks (reject mismatched formats like R8Unorm vs R16Snorm)
- **AND** SHALL test mipmap generation validation (reject non-power-of-2 dimensions)
- **AND** SHALL test partial update bounds checking (prevent updates exceeding texture dimensions)
- **AND** SHALL test texture format conversion validation
- **AND** SHALL test memory limit enforcement
- **AND** SHALL test texture upload error handling with context
- **AND** SHALL achieve ≥ 50% coverage on texture.rs upload methods

#### Scenario: GPU Resource Management
- **WHEN** GPU resources (textures, buffers, pipelines) are created or destroyed
- **THEN** the system SHALL test device creation failure handling
- **AND** SHALL test surface format detection
- **AND** SHALL test pipeline creation success
- **AND** SHALL test pipeline recreation on format change
- **AND** SHALL test shader compilation error handling
- **AND** SHALL test bind group creation and validation
- **AND** SHALL test texture format compatibility
- **AND** SHALL test memory cleanup on drop
- **AND** SHALL achieve ≥ 30% coverage on graphics.rs and ≥ 25% on pipeline.rs (adjusted for testability)

#### Scenario: Concurrent View State Testing
- **WHEN** view state operations are tested concurrently
- **THEN** the system SHALL test multiple threads calling MprView methods simultaneously
- **AND** SHALL test race conditions in dirty flag updates
- **AND** SHALL test view manager add/remove during iteration
- **AND** SHALL test state restoration during concurrent updates
- **AND** SHALL test concurrent pan operations (set_pan, set_pan_mm)
- **AND** SHALL verify thread safety across all MPR view public methods
- **AND** SHALL achieve ≥ 60% coverage on view manager code

#### Scenario: Shader Compilation Error Handling
- **WHEN** shaders are compiled and validated
- **THEN** the system SHALL test invalid WGSL syntax error handling
- **AND** SHALL test resource binding count limit enforcement
- **AND** SHALL test missing uniform definition detection
- **AND** SHALL test type mismatch detection between vertex/fragment/compute shaders
- **AND** SHALL test shader compilation error messages include stage information (vertex/fragment/compute)
- **AND** SHALL achieve ≥ 40% coverage on shader compilation and validation code

### Requirement: Memory Management Testing
The system SHALL provide memory management tests with a defined detection strategy (valgrind integration, custom allocator, or explicit removal) to prevent memory leaks in medical imaging workflows.

#### Scenario: Memory Leak Detection
- **WHEN** memory leak tests are defined
- **THEN** the system SHALL use valgrind integration in CI for native tests
- **OR** SHALL implement custom allocator tracking if valgrind is unavailable
- **OR** SHALL explicitly remove memory leak tests if neither approach is viable
- **AND** SHALL test repeated volume load/unload cycles
- **AND** SHALL test texture memory cleanup
- **AND** SHALL test view manager memory cleanup
- **AND** SHALL test DICOM repo memory growth
- **AND** SHALL detect memory leaks (> 10 KB growth after cleanup)
- **AND** SHALL achieve ≥ 70% coverage on memory management code

### Requirement: Performance Benchmarking with Realistic Targets
The system SHALL provide performance benchmarks with realistic targets based on statistical analysis and hardware variability to ensure CI gates are reliable and not flaky.

#### Scenario: Statistical Performance Benchmarking
- **WHEN** performance benchmarks are executed
- **THEN** the system SHALL use statistical thresholds (mean ± 3 standard deviations) instead of hard time limits
- **AND** SHALL allow for hardware variability (different CI runner specifications)
- **AND** SHALL implement performance regression detection (trend analysis across runs)
- **AND** SHALL establish performance baselines before measuring
- **AND** SHALL document benchmark execution environment (CPU, RAM, OS)
- **AND** SHALL measure key metrics: DICOM parsing (< 10ms), volume creation (< 50ms), volume rendering (< 16ms at 60 FPS), MPR slice extraction (< 1ms)
- **AND** SHALL NOT block CI on single benchmark outliers (allow ± 3 std dev variance)

### Requirement: WASM Testing
The system SHALL provide comprehensive test coverage for WebAssembly targets including unit tests with `#[cfg(target_arch = "wasm32")]` and browser integration tests to ensure medical safety in web deployment.

#### Scenario: WASM Unit Testing
- **WHEN** WASM-specific code paths are tested
- **THEN** the system SHALL test MHA parsing without tokio
- **AND** SHALL test MHD parsing without tokio
- **AND** SHALL test wasm-bindgen bridge error handling
- **AND** SHALL test browser-specific error paths
- **AND** SHALL test memory limits in browser environment
- **AND** SHALL use `#[cfg(target_arch = "wasm32")]` guards appropriately
- **AND** SHALL achieve ≥ 50% coverage on WASM-specific code paths

#### Scenario: Browser Integration Testing
- **WHEN** browser integration is tested
- **THEN** the system SHALL test browser DOM interaction (canvas element access)
- **AND** SHALL test WASM-specific file I/O (IndexedDB, File System Access API)
- **AND** SHALL test cross-browser compatibility (Chrome, Firefox, Safari)
- **AND** SHALL test wasm-pack build and loading workflow
- **AND** SHALL document integration test execution (npm test + wasm-pack)
- **AND** SHALL use Playwright or Puppeteer for browser automation
- **AND** SHALL achieve ≥ 40% coverage on WASM and browser interaction code

### Requirement: Test Fixture Infrastructure
The system SHALL provide a centralized test fixture infrastructure with defined architecture, export policy, and versioning strategy to prevent duplication and ensure maintainability.

#### Scenario: Centralized Fixtures
- **WHEN** test fixtures are created and used
- **THEN** the system SHALL provide fixtures in central location: `tests/common/mod.rs`
- **AND** SHALL re-export fixtures for use: `use tests::common::*` instead of local definitions
- **AND** SHALL prevent fixture duplication across test modules
- **AND** SHALL document fixture usage with examples
- **AND** SHALL include fixtures for: valid DICOM, invalid DICOM (missing UID, invalid rescale), test volumes, test patients
- **AND** SHALL verify existing tests use common fixtures instead of local duplicates

#### Scenario: Fixture Versioning and Medical Datasets
- **WHEN** fixtures are versioned
- **THEN** the system SHALL create `tests/fixtures/` directory
- **AND** SHALL include versioned DICOM datasets (v001_valid_dicom.dcm, v002_invalid_uid.dcm)
- **AND** SHALL include fixture manifest: `tests/fixtures/manifest.toml`
- **AND** SHALL document fixture creation guidelines
- **AND** SHALL version fixtures when DICOM spec changes or test requirements evolve
- **AND** SHALL document fixture purpose and expected test coverage

### Requirement: Regression Testing
The system SHALL provide regression test infrastructure with defined naming conventions, workflow, and tracking to prevent bug recurrence.

#### Scenario: Regression Test Naming Convention
- **WHEN** regression tests are created
- **THEN** the system SHALL use naming convention: `regression_issue_NNN_symptom` (e.g., `regression_issue_123_window_clamp_crash`)
- **AND** SHALL use fallback format `regression_module_symptom` for issues without GitHub issue numbers
- **AND** SHALL document naming convention in `doc/agents/regression_testing_guide.md`
- **AND** SHALL include test templates with examples
- **AND** SHALL enable tests with `#[ignore]` by default until bug is fixed

#### Scenario: Regression Test Workflow
- **WHEN** bugs are fixed
- **THEN** the system SHALL require adding regression test with each bug fix
- **AND** SHALL document process for adding regression test in PR guidelines
- **AND** SHALL document regression test execution (remove #[ignore] when fixed)
- **AND** SHALL track regression tests by issue number, fix commit, and test addition date
- **AND** SHALL verify regression tests run on every PR

### Requirement: Test Coverage Methodology
The system SHALL provide a defined coverage calculation methodology including coverage type, tool configuration, metrics definition, thresholds, and exclusion policy to ensure coverage targets are measurable and meaningful.

#### Scenario: Coverage Calculation and Tooling
- **WHEN** test coverage is measured
- **THEN** the system SHALL use `cargo-llvm-cov` as coverage tool
- **AND** SHALL measure branch coverage (not just line coverage)
- **AND** SHALL calculate coverage as `covered_lines / total_lines * 100`
- **AND** SHALL configure cargo-llvm-cov with appropriate settings
- **AND** SHALL generate HTML coverage reports for CI upload
- **AND** SHALL store coverage reports for PR comparison and trend analysis

#### Scenario: Medical Path Coverage Definition
- **WHEN** medical path coverage is measured
- **THEN** the system SHALL define medical paths as: DICOM parsing code (src/data/dicom/), patient metadata code (src/data/dicom/patient.rs, src/data/dicom/studyset.rs), coordinate transformation code (src/rendering/view/mpr/)
- **AND** SHALL set coverage threshold for medical paths at ≥ 80%
- **AND** SHALL enforce medical path coverage gates in CI (fail if < 80%)
- **AND** SHALL measure medical path coverage separately from overall coverage

#### Scenario: Coverage Exclusion Policy
- **WHEN** coverage targets are calculated
- **THEN** the system SHALL define exclusion policy for code that cannot be reasonably tested
- **AND** SHALL exclude unsafe blocks from coverage calculations
- **AND** SHALL exclude FFI calls and external library bindings
- **AND** SHALL exclude platform-specific code (native-only or WASM-only paths) when running on opposite platform
- **AND** SHALL document all exclusions in `doc/agents/coverage_methodology.md`
- **AND** SHALL adjust coverage targets based on excluded code percentages

### Requirement: CI/CD Coverage Reporting and Gating
The system SHALL provide CI/CD pipeline configuration for coverage reporting, upload, trend analysis, and threshold enforcement to ensure code quality standards are maintained.

#### Scenario: Coverage Report Generation
- **WHEN** tests run in CI
- **THEN** the system SHALL install and configure `cargo-llvm-cov`
- **AND** SHALL generate coverage reports with `cargo llvm-cov --html --output-dir coverage`
- **AND** SHALL generate JSON reports for trend analysis
- **AND** SHALL upload coverage reports to Codecov or Codecov
- **AND** SHALL store coverage artifacts for PR comparison
- **AND** SHALL configure parallel test execution: `cargo test --jobs 4`

#### Scenario: Coverage Gate Enforcement
- **WHEN** pull requests are validated
- **THEN** the system SHALL enforce medical path coverage gate: fail if < 80% on dicom/, patient.rs, studyset.rs, coordinate transformation code
- **AND** SHALL enforce overall coverage gates by phase: Phase 1 ≥ 45%, Phase 2 ≥ 60%, Phase 3 ≥ 65%, Phase 4 ≥ 70%
- **AND** SHALL implement coverage trend detection: fail if coverage decreases > 5% from baseline
- **AND** SHALL add coverage comment in PR via bot integration
- **AND** SHALL provide clear failure messages for threshold violations
- **AND** SHALL allow temporary coverage decreases with explicit approval tags

### Requirement: Phase-Based Coverage Targets
The system SHALL provide phase-based coverage targets that increase or stay consistent across phases to ensure measurable progress.

#### Scenario: Phase 1 Coverage Target
- **WHEN** Phase 1 is complete (Week 2)
- **THEN** overall coverage SHALL be ≥ 45% (from 24% baseline)
- **AND** 40+ new tests SHALL be implemented
- **AND** medical path coverage SHALL be ≥ 80%
- **AND** patient identity coverage SHALL be ≥ 70% on patient.rs
- **AND** coordinate transformation coverage SHALL be ≥ 50% on mpr_view.rs

#### Scenario: Phase 2 Coverage Target
- **WHEN** Phase 2 is complete (Week 4)
- **THEN** overall coverage SHALL be ≥ 60% (increase from Phase 1)
- **AND** 54+ new tests SHALL be implemented
- **AND** MHA/MHD coverage SHALL be ≥ 60%
- **AND** volume integrity coverage SHALL be ≥ 70%
- **AND** property-based testing SHALL be working

#### Scenario: Phase 3 Coverage Target
- **WHEN** Phase 3 is complete (Week 6)
- **THEN** overall coverage SHALL be ≥ 65% (increase from Phase 2)
- **AND** 48+ new tests SHALL be implemented
- **AND** GPU initialization coverage SHALL be ≥ 40% on graphics.rs
- **AND** rendering coverage SHALL be ≥ 45% on view manager and textures
- **AND** all GPU tests SHALL pass on native platform

#### Scenario: Phase 4 Coverage Target
- **WHEN** Phase 4 is complete (Week 8)
- **THEN** overall coverage SHALL be ≥ 70% (increase from Phase 3)
- **AND** 42+ new tests SHALL be implemented
- **AND** error path coverage SHALL be ≥ 70%
- **AND** edge case coverage SHALL be ≥ 60%
- **AND** memory tests SHALL pass (valgrind clean or equivalent)

#### Scenario: Phase 5 Ongoing
- **WHEN** Phase 5 is ongoing (Week 9+)
- **THEN** medical path coverage SHALL be maintained ≥ 80%
- **AND** regression test SHALL exist for each bug fix
- **AND** performance benchmarks SHALL be in place
- **AND** CI coverage gates SHALL be active
- **AND** coverage trend analysis SHALL detect regressions early
- **AND** overall project coverage SHALL be tracked across all PRs
