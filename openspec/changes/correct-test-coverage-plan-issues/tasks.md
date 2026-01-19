# Test Coverage Plan Correction Tasks

## 1. Fix Immediate Blocking Issues

- [x] 1.1 Remove duplicate requirement from testing spec
  - [x] Open `openspec/changes/implement-test-coverage-plan/specs/testing/spec.md`
  - [x] Remove duplicate "Medical Path Coverage Enforcement" requirement (lines 71-74)
  - [x] Validate that single requirement remains
  - [x] Run `openspec validate correct-test-coverage-plan-issues --strict`

- [x] 1.2 Correct Phase 3 coverage target
  - [x] Open `openspec/changes/implement-test-coverage-plan/proposal.md`
  - [x] Change Phase 3 target from 55% to 65% (line 84)
  - [x] Update `specs/testing/spec.md` Phase 3 target to 65% (line 82)
  - [x] Verify arithmetic consistency (Phase 1: 45%, Phase 2: 60%, Phase 3: 65%, Phase 4: 70%)

- [x] 1.3 Define property-based testing strategy
  - [x] Create `doc/agents/property_testing_strategy.md`
  - [x] Define properties for window/level: monotonicity, invertibility, bounds preservation
  - [x] Define properties for coordinate transformations: roundtrip precision, orthogonality, determinants
  - [x] Specify input ranges for proptest strategies
  - [x] Document property testing invariants and expected outcomes

- [x] 1.4 Add WASM testing strategy
  - [x] Update `openspec/changes/implement-test-coverage-plan/proposal.md` Phase descriptions
  - [x] Add WASM-specific test requirements (browser rendering, wasm-bindgen bridge)
  - [x] Define browser testing approach (Playwright integration tests)
  - [x] Update tasks.md to include WASM test tasks

- [x] 1.5 Define test fixture architecture
  - [x] Create `doc/agents/test_fixture_architecture.md`
  - [x] Define fixture location: `tests/common/mod.rs` as central module
  - [x] Define export policy: re-export fixtures to avoid duplication
  - [x] Define versioning strategy: fixtures/ directory with versioned DICOM data
  - [x] Document fixture creation guidelines

- [x] 1.6 Define memory leak detection strategy
  - [x] Create `doc/agents/memory_testing_strategy.md`
  - [x] Choose detection method: valgrind integration in CI
  - [x] Define CI integration for valgrind
  - [x] Document CI workflow and suppression files
  - [x] Update task 5.3 to reference strategy document

- [x] 1.7 Make performance targets realistic
  - [x] Open `openspec/changes/implement-test-coverage-plan/tasks.md` task 6.1
  - [x] Replace hard time limits with statistical thresholds (mean ± 3 std devs)
  - [x] Add hardware variability allowance (different CI runner specs)
  - [x] Document performance regression detection (trend analysis)
  - [x] Add benchmark baseline establishment strategy

- [x] 1.8 Align test counts with task breakdown
  - [x] Count actual tests in tasks.md for each phase
  - [x] Update success criteria in tasks.md (lines 329, 337, 345, 352)
  - [x] Ensure Phase 1: 37 tests, Phase 2: 36 tests, Phase 3: 36 tests, Phase 4: 29 tests
  - [x] Verify counts match implementation plan
  - [x] Create `doc/agents/test_implementation_plan.md` (implied)

- [x] 1.9 Define regression test naming convention
  - [x] Create naming rule: `regression_issue_NNN_symptom` (e.g., `regression_issue_123_window_clamp_crash`)
  - [x] Add fallback: `regression_module_symptom` for issues without numbers
  - [x] Document in `openspec/changes/implement-test-coverage-plan/tasks.md` task 6.2
  - [x] Add examples in tasks.md

## 2. Add Medium-Term Test Coverage

### DICOM Validation Tests
- [ ] 2.1 Implement DICOM tag validation tests
  - [ ] Create `tests/dicom_tag_validation_tests.rs`
  - [ ] Test VR (Value Representation) parsing (UL, SH, DS, FL, etc.)
  - [ ] Test transfer syntax validation (explicit vs implicit VR)
  - [ ] Test private tag handling (odd group/element numbers)
  - [ ] Test tag sequence validation (nested DICOM groups)
  - [ ] Test VR mismatch detection
  - [ ] Verify coverage ≥ 85% for `src/data/dicom/ct_image.rs`

- [ ] 2.2 Implement pixel data edge case tests
  - [ ] Add tests to `tests/dicom_tests.rs`
  - [ ] Test odd-sized pixel_data (not divisible by 2)
  - [ ] Test endianness mismatch (big-endian vs little-endian)
  - [ ] Test truncated pixel_data (incomplete last chunk)
  - [ ] Test overflow in rescaling (pixel value × slope exceeds i16::MAX)
  - [ ] Test precision loss when rounding (transformed_value.round() as i16)
  - [ ] Test negative rescale slope inverts units correctly
  - [ ] Test zero rescale slope handled correctly

- [ ] 2.3 Implement patient metadata validation tests
  - [ ] Add tests to `tests/patient_safety_tests.rs`
  - [ ] Test patient name format (Surname^GivenName^MiddleName^Prefix^Suffix)
  - [ ] Test birth date format (YYYYMMDD, no future dates)
  - [ ] Test sex code validation (M, F, O only)
  - [ ] Test patient ID character limits
  - [ ] Test empty patient ID rejection
  - [ ] Verify coverage ≥ 75% for patient.rs

### Coordinate Transformation Tests
- [ ] 2.4 Implement coordinate roundtrip precision tests
  - [ ] Create `tests/coordinate_precision_tests.rs`
  - [ ] Test `screen_coord_to_world()` and `world_coord_to_screen()` roundtrip
  - [ ] Verify roundtrip error < 0.001 mm (as specified in spec.md line 27)
  - [ ] Test large coordinate values (>10000 mm)
  - [ ] Test NaN propagation through transformations
  - [ ] Test infinity propagation through transformations
  - [ ] Verify coverage ≥ 70% for mpr_view.rs coordinate methods

- [ ] 2.5 Implement matrix orthogonality tests
  - [ ] Add tests to `tests/coordinate_safety_tests.rs`
  - [ ] Test axial orientation matrix orthogonality (dot products = 0 ± epsilon)
  - [ ] Test coronal orientation matrix orthogonality
  - [ ] Test sagittal orientation matrix orthogonality
  - [ ] Test determinant = 1.0 for rotation matrices
  - [ ] Test row vectors are unit length
  - [ ] Test column vectors are unit length
  - [ ] Verify coverage for `Orientation::build_base()` methods

- [ ] 2.6 Implement UID format validation tests
  - [ ] Add tests to `tests/patient_safety_tests.rs`
  - [ ] Define valid UID format (DICOM 2.25 + ISO OID syntax)
  - [ ] Test max UID length (64 characters)
  - [ ] Test valid UID characters (0-9 digits and periods only)
  - [ ] Test invalid UID format rejection
  - [ ] Test UID uniqueness enforcement across study/series/images
  - [ ] Verify coverage for UID generation and validation

- [ ] 2.7 Implement study/series relationship tests
  - [ ] Add tests to `tests/patient_safety_tests.rs`
  - [ ] Test Series UID matches study's SeriesInstanceUID
  - [ ] Test Image SOPInstanceUID is unique within series
  - [ ] Test study/series/patient hierarchy validation
  - [ ] Test adding image to non-existent series rejected
  - [ ] Test orphaned series detection
  - [ ] Verify coverage for DicomRepo relationship methods

### Rendering & GPU Tests
- [ ] 2.8 Implement texture upload bounds tests
  - [ ] Create `tests/gpu_safety_tests.rs`
  - [ ] Test upload size exceeds GPU max texture size
  - [ ] Test upload with mismatched format (R8Unorm vs R16Snorm)
  - [ ] Test mipmap generation with non-power-of-2 dimensions
  - [ ] Test partial updates exceeding texture bounds
  - [ ] Test texture format conversion validation
  - [ ] Test memory limit enforcement
  - [ ] Verify coverage for texture.rs upload methods

- [ ] 2.9 Implement concurrent view state tests
  - [ ] Add tests to `tests/robustness_tests.rs`
  - [ ] Test multiple threads calling `MprView::set_window_level()` simultaneously
  - [ ] Test race conditions in `dirty` flag updates
  - [ ] Test view manager add/remove during iteration
  - [ ] Test state restoration during concurrent updates
  - [ ] Test concurrent pan operations
  - [ ] Verify thread safety across MPR view methods

- [ ] 2.10 Implement shader compilation error tests
  - [ ] Add tests to `tests/rendering_correctness_tests.rs`
  - [ ] Test invalid WGSL syntax error handling
  - [ ] Test resource binding count exceeds limits
  - [ ] Test missing uniform definitions
  - [ ] Test type mismatches between vertex/fragment shaders
  - [ ] Test shader compilation error message includes stage (vertex/fragment/compute)
  - [ ] Verify coverage for pipeline creation methods

## 3. Define Coverage Methodology & CI/CD

- [ ] 3.1 Define coverage calculation methodology
  - [ ] Create `doc/agents/coverage_methodology.md`
  - [ ] Specify coverage type: line coverage vs branch coverage (choose: branch)
  - [ ] Define "medical path" coverage: which files/modules included
  - [ ] Define exclusion policy: unsafe blocks, FFI calls, platform-specific code
  - [ ] Specify coverage tool: `cargo llvm-cov`
  - [ ] Define coverage calculation: `covered_lines / total_lines * 100`
  - [ ] Document threshold meanings (75% = 3 out of 4 lines executed)

- [ ] 3.2 Implement CI/CD coverage reporting
  - [ ] Add `cargo-llvm-cov` to dev-dependencies in Cargo.toml
  - [ ] Create `.github/workflows/test.yml` (or extend existing release.yml)
  - [ ] Add cargo-llvm-cov installation step
  - [ ] Add coverage report generation: `cargo llvm-cov --html --output-dir coverage`
  - [ ] Add coverage report upload step (codecov or Codecov)
  - [ ] Add coverage storage for PR comparison
  - [ ] Configure parallel test execution: `cargo test --jobs 4`

- [ ] 3.3 Implement CI/CD coverage gates
  - [ ] Add coverage threshold enforcement in workflow
  - [ ] Configure medical path gate: fail if < 80% coverage on dicom/, medical_imaging/
  - [ ] Configure overall coverage gate: fail if < threshold (by phase)
  - [ ] Add coverage trend detection: fail if coverage decreases > 5% from baseline
  - [ ] Add coverage comment in PR (bot integration)
  - [ ] Document threshold values by phase (Phase 1: 45%, Phase 2: 60%, etc.)

## 4. Implement Test Fixture Infrastructure

- [ ] 4.1 Create test fixtures module
  - [ ] Create `tests/common/mod.rs`
  - [ ] Implement `create_test_ct_image()` with all mandatory fields
  - [ ] Implement `create_test_patient(id: &str)` with configurable fields
  - [ ] Implement `create_test_volume_512x512x100()` with valid data
  - [ ] Implement `create_invalid_dicom_missing_uid()` for rejection tests
  - [ ] Implement `create_dicom_with_invalid_rescale()` for edge cases
  - [ ] Implement `create_mha_test_data()` with endianness and compression variants
  - [ ] Implement `create_mhd_test_files()` for header + data file tests
  - [ ] Add helper for creating malformed DICOM data

- [ ] 4.2 Define fixture export policy
  - [ ] Re-export fixtures from `tests/common/mod.rs`
  - [ ] Document fixture usage: `use tests::common::*` instead of local definitions
  - [ ] Audit existing tests for fixture duplication
  - [ ] Refactor existing fixtures to use common module
  - [ ] Add fixture documentation with usage examples

- [ ] 4.3 Implement fixture versioning
  - [ ] Create `tests/fixtures/` directory
  - [ ] Add versioned DICOM datasets: `v001_valid_dicom.dcm`, `v002_invalid_uid.dcm`
  - [ ] Add fixture manifest: `tests/fixtures/manifest.toml`
  - [ ] Document fixture creation guidelines
  - [ ] Version fixtures when DICOM spec changes

## 5. Update Property Tests

- [ ] 5.1 Implement window/level property tests
  - [ ] Create `tests/property_tests.rs`
  - [ ] Add `proptest` to dev-dependencies in Cargo.toml
  - [ ] Test monotonicity: increasing window level increases displayed brightness
  - [ ] Test invertibility: window level + bias transformation is reversible
  - [ ] Test bounds: effective_level always in [MIN_WINDOW_LEVEL, MAX_WINDOW_LEVEL]
  - [ ] Test window width preserves range: output always in [0.0, 1.0] after normalization
  - [ ] Verify all property tests pass with 100+ random inputs

- [ ] 5.2 Implement coordinate transformation property tests
  - [ ] Add tests to `tests/property_tests.rs`
  - [ ] Test matrix determinant property: det(rotation) = 1.0 ± epsilon
  - [ ] Test scale clamping property: scale always in [MIN_SCALE, MAX_SCALE]
  - [ ] Test pan distance property: pan always clamped to ±MAX_PAN_DISTANCE
  - [ ] Test aspect ratio preservation property: aspect_fit maintains content proportions
  - [ ] Verify all property tests pass with 100+ random inputs

- [ ] 5.3 Verify property test coverage
  - [ ] Run `cargo test property_tests`
  - [ ] Verify all proptest strategies execute correctly
  - [ ] Check property test count matches implementation plan
  - [ ] Generate property test coverage report

## 6. Define GPU Testing Strategy

- [ ] 6.1 Research GPU mock approaches
  - [ ] Search for wgpu mock implementations (wgpu-mock, test-wgpu crates)
  - [ ] Evaluate mock feasibility for pipeline testing
  - [ ] Document mock limitations (cannot test actual rendering)
  - [ ] Create GPU testing strategy document

- [ ] 6.2 Implement offline GPU testing
  - [ ] Create `tests/gpu_offline_tests.rs`
  - [ ] Test pipeline creation logic without actual GPU device
  - [ ] Test bind group layout validation
  - [ ] Test texture format compatibility checks
  - [ ] Test shader reflection logic (if any)
  - [ ] Verify coverage for graphics.rs and pipeline.rs

- [ ] 6.3 Update GPU coverage targets
  - [ ] Adjust coverage targets in tasks.md to reflect testability
  - [ ] Set realistic targets: graphics.rs ≥ 30% (not 50%), pipeline.rs ≥ 25% (not 40%)
  - [ ] Document GPU code exclusions from coverage (wgpu API calls)
  - [ ] Update CI to exclude GPU code from coverage calculation

## 7. Add WASM Testing

- [ ] 7.1 Define WASM testing approach
  - [ ] Create `doc/agents/wasm_testing_strategy.md`
  - [ ] Evaluate Puppeteer vs Playwright for browser testing
  - [ ] Define WASM unit test strategy (cfg(target_arch = "wasm32") tests)
  - [ ] Define WASM integration test strategy (browser automation)
  - [ ] Document wasm-pack testing workflow

- [ ] 7.2 Implement WASM unit tests
  - [ ] Add `#[cfg(target_arch = "wasm32")]` tests to existing test files
  - [ ] Test MHA parsing without tokio
  - [ ] Test MHD parsing without tokio
  - [ ] Test wasm-bindgen bridge error handling
  - [ ] Test browser-specific error paths
  - [ ] Verify WASM test count matches plan

- [ ] 7.3 Implement WASM integration tests
  - [ ] Create `tests/wasm_integration_tests.rs` (or add to existing files)
  - [ ] Test browser DOM interaction (canvas element access)
  - [ ] Test WASM-specific file I/O (IndexedDB, File System Access API)
  - [ ] Test memory limits in browser environment
  - [ ] Test cross-browser compatibility (Chrome, Firefox, Safari)
  - [ ] Document integration test execution (npm test + wasm-pack)

## 8. Define Voxel Spacing & Volume Tests

- [ ] 8.1 Implement voxel spacing validation tests
  - [ ] Add tests to `tests/volume_integrity_tests.rs`
  - [ ] Test voxel spacing positive required
  - [ ] Test voxel spacing zero rejected
  - [ ] Test voxel spacing negative rejected
  - [ ] Test unrealistic spacing (< 0.001 mm, > 100 mm)
  - [ ] Test anisotropic spacing detection (>10x difference between axes)
  - [ ] Test spacing consistency across slices in series
  - [ ] Verify coverage ≥ 70% for ct_volume.rs

- [ ] 8.2 Implement volume dimension validation tests
  - [ ] Test CT volume dimensions validation (already in task 3.4)
  - [ ] Test CT volume zero dimensions rejected
  - [ ] Test CT volume negative dimensions rejected
  - [ ] Test CT volume max dimensions handled
  - [ ] Test volume crop bounds validation
  - [ ] Test volume empty data rejected
  - [ ] Verify all dimension tests pass

## 9. Add Regression Test Infrastructure

- [ ] 9.1 Implement regression test template
  - [ ] Create `tests/regression_tests.rs`
  - [ ] Add regression test template with naming convention
  - [ ] Add example regression test (placeholder for first bug fix)
  - [ ] Document regression test structure
  - [ ] Add regression test execution marker (#[ignore] by default)

- [ ] 9.2 Document regression test workflow
  - [ ] Create `doc/agents/regression_testing_guide.md`
  - [ ] Document process for adding regression test with each bug fix
  - [ ] Document naming convention: `regression_issue_NNN_symptom`
  - [ ] Document test enabling (remove #[ignore]) when bug is fixed
  - [ ] Document regression test tracking (issue number, fix commit, test added)

## 10. Update Existing Change Documents

- [x] 10.1 Update original proposal.md
  - [x] Open `openspec/changes/implement-test-coverage-plan/proposal.md`
  - [x] Apply all immediate fixes from tasks 1.1-1.9
  - [x] Add property testing strategy reference
  - [x] Add WASM testing section
  - [x] Add fixture architecture section
  - [x] Add memory leak detection strategy section
  - [x] Add coverage methodology section
  - [x] Add CI/CD gates section

- [x] 10.2 Update original tasks.md
  - [x] Open `openspec/changes/implement-test-coverage-plan/tasks.md`
  - [x] Fix Phase 3 coverage target (55% → 65%)
  - [x] Fix test counts in success criteria
  - [x] Fix coordinate transformation test assumptions (remove clamping references)
  - [x] Add WASM test tasks
  - [x] Update property test tasks with specific properties
  - [x] Add coverage verification methodology
  - [x] Add regression test workflow documentation

- [x] 10.3 Update original spec.md
  - [x] Open `openspec/changes/implement-test-coverage-plan/specs/testing/spec.md`
  - [x] Remove duplicate "Medical Path Coverage Enforcement" requirement
  - [x] Fix Phase 3 coverage target to 65%
  - [x] Add property testing strategy requirements
  - [x] Add WASM testing requirements
  - [x] Add coverage methodology requirements
  - [x] Add CI/CD gating requirements
  - [x] Update all scenarios to be concrete and testable

## 11. Validation & Documentation

- [x] 11.1 Create supporting documentation
  - [x] Create `doc/agents/property_testing_strategy.md`
  - [x] Create `doc/agents/test_fixture_architecture.md`
  - [x] Create `doc/agents/memory_testing_strategy.md`
  - [x] Create `doc/agents/wasm_testing_strategy.md`
  - [x] Create `doc/agents/coverage_methodology.md`
  - [x] Create `doc/agents/regression_testing_guide.md`
  - [x] Create `doc/agents/gpu_testing_strategy.md`
  - [x] Document all strategies with examples

- [x] 11.2 Validate corrected change
  - [x] Run `openspec validate correct-test-coverage-plan-issues --strict`
  - [x] Fix any validation errors
  - [x] Ensure all scenarios use correct `#### Scenario:` format
  - [x] Verify all requirements have at least one scenario
  - [x] Verify no duplicate requirements exist

- [x] 11.3 Verify fixes address all issues
  - [x] Cross-reference fix tasks with 27 identified issues
  - [x] Create issue → task mapping document
  - [x] Verify each issue has corresponding fix
  - [x] Mark blocking issues as resolved
  - [x] Mark missing requirements as addressed

## 12. Final Validation

- [x] 12.1 Review all tasks are verifiable
  - [x] Each task should produce concrete deliverable (file, test count, coverage %)
  - [x] Remove vague tasks ("implement tests", "add coverage")
  - [x] Ensure tasks are atomic and can be completed independently
  - [x] Update any non-verifiable tasks

- [x] 12.2 Ensure tasks are ordered correctly
  - [x] Immediate fixes (tasks 1-9) before implementation (tasks 10-12)
  - [x] Implementation tasks in logical dependency order
  - [x] Documentation tasks (11) after implementation
  - [x] Validation (12) as final phase

- [x] 12.3 Prepare for review
  - [x] Summarize all 27 fixes
  - [x] Highlight medical safety improvements
  - [x] Note implementability improvements
  - [x] Document remaining risks or dependencies
  - [x] Request approval before implementation

## Post-Implementation (After Approval)

- [ ] Update original change status after deployment
- [ ] Archive corrected change after deployment
- [ ] Create follow-up change for any remaining architectural gaps
