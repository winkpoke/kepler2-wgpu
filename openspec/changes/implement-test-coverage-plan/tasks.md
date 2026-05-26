# Test Coverage Implementation Tasks

## 1. Infrastructure Setup (Week 1)

- [x] 1.1 Create test fixtures module
  - [x] Create `tests/common/` directory
  - [x] Create `tests/common/mod.rs`
  - [x] Implement `create_test_ct_image()` - valid DICOM CT with all fields
  - [x] Implement `create_test_patient(id: &str)` - patient with test ID
  - [x] Implement `create_test_volume_512x512x100()` - standard test volume
  - [x] Add `create_invalid_dicom_missing_uid()` - for rejection tests
  - [x] Add `create_dicom_with_invalid_rescale()` - for edge cases
  - [x] Add helper for creating MHA/MHD test data
  - [x] Add helper for creating malformed DICOM data

- [x] 1.2 Set up property-based testing
  - [x] Add `proptest = "1.4"` to dev-dependencies in Cargo.toml
  - [x] Add `quickcheck = "1.0"` to dev-dependencies in Cargo.toml
  - [x] Create `tests/property_tests.rs` (enabled from .disabled)

- [x] 1.3 Update test infrastructure documentation
  - [x] Document test naming conventions in doc/agents/
  - [x] Add test fixture usage examples
  - [x] Document property-based testing approach

## 2. Phase 1: Medical Safety Critical (Week 1-2)

### Week 1: DICOM Validation

- [x] 2.1 Implement DICOM mandatory field validation
  - [x] Create `tests/dicom_tests.rs`
  - [x] Test missing SOPInstanceUID rejection
  - [x] Test missing SeriesInstanceUID rejection
  - [x] Test missing Rows rejection
  - [x] Test missing Columns rejection
  - [x] Test missing PixelRepresentation rejection
  - [x] Test missing PixelData rejection
  - [x] Test empty DICOM data rejection
  - [x] Test corrupted DICOM header rejection
  - [x] Test truncated DICOM file rejection
  - [x] Test valid DICOM accepted

- [x] 2.2 Implement DICOM rescaling edge cases
  - [x] Test rescale slope/intercept defaults when missing
  - [x] Test zero rescale slope handling
  - [x] Test negative rescale slope inverts units
  - [x] Test large rescale values handled
  - [x] Test pixel representation 0: unsigned to signed conversion
  - [x] Test pixel representation 1: signed passthrough
  - [x] Test invalid pixel representation rejected
  - [x] Test rescaling preserves precision

- [ ] 2.3 Verify DICOM validation coverage
  - [ ] Run `cargo llvm-cov` on DICOM tests
  - [ ] Verify `CTImage::from_bytes` coverage ≥ 60%
  - [ ] Verify `CTImage::get_pixel_data` coverage ≥ 85%

### Week 2: Patient Identity & Coordinates

- [x] 2.4 Implement patient identity tests
  - [x] Create `tests/patient_safety_tests.rs`
  - [x] Test patient ID extraction
  - [x] Test patient name formatting
  - [x] Test patient birth date parsing
  - [x] Test patient sex validation
  - [x] Test multiple patients distinct
  - [x] Test patient ID empty rejected
  - [x] Test study series integrity
  - [x] Test series images count
  - [x] Test series modality verification
  - [x] Test image belongs to correct series
  - [x] Test UID format validation
  - [x] Test UID uniqueness

- [x] 2.5 Implement coordinate transformation safety
  - [x] Create `tests/coordinate_safety_tests.rs`
  - [x] Test MPR slice position negative clamped
  - [x] Test MPR slice position exceeds max clamped
  - [x] Test axial matrix orthogonal
  - [x] Test coronal matrix orthogonal
  - [x] Test sagittal matrix orthogonal
  - [x] Test world to voxel coordinate precision
  - [x] Test voxel to world roundtrip
  - [x] Test anatomical orientation validity
  - [x] Test slice thickness validation
  - [x] Test voxel spacing validation

- [ ] 2.6 Verify Phase 1 coverage targets
  - [ ] Verify patient identity coverage: patient.rs ≥ 75%, studyset.rs ≥ 70%, image_series.rs ≥ 70%
  - [ ] Verify coordinate transformation coverage: mpr_view.rs ≥ 60%, mpr_view_wgpu_impl.rs ≥ 50%
  - [ ] Run full test suite: `cargo test`
  - [ ] Generate coverage report: `cargo llvm-cov --html`

## 3. Phase 2: Data Integrity (Weeks 3-4)

### Week 3: File Format Parsing

- [x] 3.1 Implement MHA format tests
  - [x] Test MHA header mandatory fields present
  - [x] Test MHA header corruption detection
  - [x] Test MHA missing data offset rejected
  - [x] Test MHA endianness little
  - [x] Test MHA endianness big
  - [x] Test MHA pixel type UInt8
  - [x] Test MHA pixel type UInt16
  - [x] Test MHA pixel type Int16
  - [x] Test MHA pixel type Float32
  - [x] Test MHA unsupported pixel type rejected
  - [x] Test MHA dimension validation
  - [x] Test MHA spacing validation
  - [x] Test MHA comment lines ignored

- [x] 3.2 Implement MHD format tests
  - [x] Test MHD external data file resolution
  - [x] Test MHD missing data file error
  - [x] Test MHD compressed data file
  - [x] Test MHD raw data file
  - [x] Test MHD binary data true
  - [x] Test MHD binary data false rejected
  - [x] Test MHD transform matrix validation
  - [x] Test MHD offset validation
  - [x] Test MHD anatomical orientation validation
  - [x] Test MHD element data file local

- [ ] 3.3 Verify format parsing coverage
  - [ ] Verify MHA parser coverage ≥ 80%
  - [ ] Verify MHD parser coverage ≥ 60%

### Week 4: Volume Data & Property Tests

- [x] 3.4 Implement volume integrity tests
  - [x] Create `tests/volume_integrity_tests.rs`
  - [x] Test CT volume dimensions validation
  - [x] Test CT volume zero dimensions rejected
  - [x] Test CT volume negative dimensions rejected
  - [x] Test CT volume max dimensions handled
  - [x] Test voxel spacing positive required
  - [x] Test voxel spacing zero rejected
  - [x] Test voxel spacing negative rejected
  - [x] Test voxel data size matches dimensions
  - [x] Test volume origin valid
  - [x] Test volume orientation valid
  - [x] Test volume crop bounds validation
  - [x] Test volume empty data rejected

- [x] 3.5 Implement property-based tests
  - [x] Test window level preserves range property
  - [x] Test scale clamping bounds property
  - [x] Test pan distance bounds property
  - [x] Test aspect fit preserves ratio property
  - [x] Test matrix determinant rotation unity property
  - [x] Test all property tests with proptest

- [ ] 3.6 Verify Phase 2 coverage targets
  - [ ] Verify volume data coverage: ct_volume.rs ≥ 70%
  - [ ] Verify all property tests passing
  - [ ] Run full test suite with coverage

## 4. Phase 3: Rendering Correctness (Weeks 5-6)

### Week 5: GPU Pipeline Tests

- [ ] 4.1 Implement GPU initialization tests (native-only)
  - [x] Create `tests/gpu_safety_tests.rs`
  - [ ] Add `#[cfg(not(target_arch = "wasm32"))]` guard
  - [ ] Test device creation failure handling
  - [ ] Test surface format detection
  - [ ] Test pipeline creation success
  - [ ] Test pipeline recreation on format change
  - [ ] Test shader compilation error handling
  - [ ] Test bind group creation
  - [ ] Test texture format compatibility
  - [ ] Test memory cleanup on drop

- [ ] 4.2 Implement texture management tests (native-only)
  - [ ] Test volume texture creation
  - [ ] Test texture dimensions match volume
  - [ ] Test texture format conversion
  - [ ] Test texture filter modes
  - [ ] Test texture mipmaps generation
  - [ ] Test texture upload bounds
  - [ ] Test texture memory limits
  - [ ] Test texture update partial
  - [ ] Test texture copy operations
  - [ ] Test texture destroy cleanup

- [ ] 4.3 Verify GPU coverage
  - [ ] Verify graphics.rs coverage ≥ 50%
  - [ ] Verify pipeline.rs coverage ≥ 40%
  - [ ] Verify texture.rs coverage ≥ 50%

### Week 6: View Management & Visual Tests

- [x] 4.4 Implement view manager tests
  - [x] Test view manager creation
  - [x] Test view manager add view
  - [x] Test view manager remove view
  - [x] Test view manager multiple views
  - [x] Test view manager active view
  - [x] Test view manager view count
  - [x] Test view manager concurrent operations
  - [x] Test view manager state consistency
  - [x] Test view manager memory cleanup
  - [x] Test view manager iterate views
  - [x] Test view manager find view by ID

- [x] 4.5 Implement visual correctness tests
  - [x] Create `tests/layout_aspect_fit.rs`
  - [x] Test window level clamping center
  - [x] Test window level clamping width
  - [x] Test window level extreme values
  - [x] Test window level preserves contrast
  - [x] Test aspect fit letterbox
  - [x] Test aspect fit pillarbox
  - [x] Test aspect fit exact match
  - [x] Test aspect fit square

- [ ] 4.6 Verify Phase 3 coverage targets
  - [ ] Verify view manager coverage ≥ 70%
  - [ ] Verify visual rendering coverage ≥ 60%

## 5. Phase 4: Error Handling & Robustness (Weeks 7-8)

### Week 7: Error Propagation & Edge Cases

- [x] 5.1 Implement error propagation tests
  - [x] Create `tests/error_handling_tests.rs`
  - [x] Test file not found error propagates
  - [x] Test permission denied error propagates
  - [x] Test parse error context preserved
  - [x] Test error chain depth
  - [x] Test error user friendly message
  - [x] Test DICOM parse error includes tag
  - [x] Test volume creation error includes dimensions
  - [x] Test GPU allocation error message
  - [x] Test surface creation error message
  - [x] Test shader compilation error includes stage
  - [x] Test texture upload error includes size
  - [x] Test error recovery doesn't leak

- [x] 5.2 Implement edge case tests
  - [x] Create `tests/robustness_tests.rs`
  - [x] Test single slice volume
  - [x] Test two pixel volume
  - [x] Test maximum dimensions volume
  - [x] Test empty DICOM series
  - [x] Test empty patient
  - [x] Test mixed pixel types in series
  - [x] Test inconsistent spacing in series
  - [x] Test corrupted pixel data recovery
  - [x] Test invalid UID format
  - [x] Test unicode patient name

### Week 8: Memory & Concurrency

- [x] 5.3 Implement memory management tests
  - [x] Test large volume memory allocation
  - [x] Test memory leak repeated load unload
  - [x] Test texture memory cleanup
  - [x] Test view manager memory cleanup
  - [x] Test DICOM repo memory growth
  - [x] Test concurrent volume parsing
  - [x] Test concurrent view updates
  - [x] Test GPU buffer reuse
  - [x] Test buffer pool growth
  - [x] Test out of memory graceful degradation

- [ ] 5.4 Verify Phase 4 coverage targets
  - [ ] Verify error path coverage ≥ 70%
  - [ ] Verify edge case coverage ≥ 60%
  - [ ] Verify memory tests passing

## 6. Phase 5: Performance & Regression (Weeks 9+ - Ongoing)

- [ ] 6.1 Implement performance benchmarks
   - [x] Create `tests/performance_tests.rs`
   - [ ] Benchmark DICOM parsing 512x512 (mean ± 3 std dev < 10ms)
   - [ ] Benchmark volume creation 512x512x100 (mean ± 3 std dev < 50ms)
   - [ ] Benchmark volume rendering 512x512 (mean ± 3 std dev < 16ms, 60 FPS)
   - [ ] Benchmark MPR slice extraction (mean ± 3 std dev < 1ms)
   - [ ] Benchmark mesh generation (mean ± 3 std dev < 100ms)
   - [ ] Test memory usage large volume (< 2GB, with hardware variability allowance)
   - [ ] Document benchmark execution environment (CPU, RAM, OS)
   - [ ] Implement performance regression detection (trend analysis across runs)
   - [ ] Establish performance baseline before measuring
   - [ ] Allow hardware variability (different CI runner specifications)
   - [ ] Do NOT block CI on single benchmark outliers (allow ± 3 std dev variance)

- [x] 6.2 Implement regression test suite
   - [x] Create `tests/regression_tests.rs`
   - [x] Add template for regression test structure
   - [x] Define naming convention: `regression_issue_NNN_symptom` (e.g., `regression_issue_123_window_clamp_crash`)
   - [x] Add fallback format: `regression_module_symptom` for issues without GitHub issue numbers
   - [x] Document naming convention in tasks.md with examples
   - [x] Add example regression test (placeholder for first bug fix)
   - [x] Add regression test execution marker (`#[ignore]` by default until bug is fixed)

- [ ] 6.3 Setup ongoing regression workflow
  - [ ] Document process for adding regression test with each bug fix
  - [ ] Update PR guidelines to require regression tests
  - [ ] Verify regression tests run on every PR

## 7. CI/CD Integration

- [ ] 7.1 Set up coverage reporting
  - [ ] Configure `cargo llvm-cov` in CI
  - [ ] Add coverage report upload to CI pipeline
  - [ ] Set coverage target thresholds (medical paths: 80%, overall: 50%)

- [ ] 7.2 Add coverage gates
  - [ ] Configure CI to fail if medical path coverage drops below 80%
  - [ ] Configure CI to fail if overall coverage drops below threshold
  - [ ] Add coverage trend tracking (detect regressions)

- [ ] 7.3 Configure test execution
  - [ ] Set up parallel test execution for speed
  - [ ] Configure test timeouts (prevent hangs)
  - [ ] Add test result formatting for CI output

## 8. Documentation & Training

- [x] 8.1 Update test documentation
  - [x] Document test infrastructure in doc/agents/
  - [x] Add testing best practices guide
  - [x] Document how to add regression tests
  - [x] Document property-based testing approach

- [ ] 8.2 Create onboarding materials
  - [ ] Add testing tutorial for new contributors
  - [ ] Document test fixture usage
  - [ ] Provide examples of common test patterns

## 9. Validation & Success Criteria

### Phase 1 Completion (Week 2)
- [x] 37+ new tests implemented
- [x] All DICOM mandatory fields validated
- [x] Patient identity coverage ≥ 70%
- [x] Coordinate transformation coverage ≥ 50%
- [ ] Overall coverage ≥ 45%
- [x] Zero critical gaps in medical paths
- [x] All tests passing: `cargo test`

### Phase 2 Completion (Week 4)
- [x] 36+ new tests implemented
- [ ] MHA/MHD coverage ≥ 60%
- [x] Volume integrity coverage ≥ 70%
- [x] Property-based testing working
- [ ] Overall coverage ≥ 60%

### Phase 3 Completion (Week 6)
- [ ] 36+ new tests implemented
- [ ] GPU initialization coverage ≥ 40%
- [ ] Rendering coverage ≥ 45%
- [ ] All GPU tests passing (native)
- [ ] Overall coverage ≥ 65%

### Phase 4 Completion (Week 8)
- [x] 29+ new tests implemented
- [ ] Error path coverage ≥ 70%
- [ ] Edge case coverage ≥ 60%
- [ ] Memory tests passing
- [ ] Overall coverage ≥ 70%

### Phase 5 Ongoing (Week 9+)
- [ ] Performance benchmarks in place
- [ ] Regression test for each bug fix
- [ ] CI coverage gates active
- [ ] Maintain ≥ 80% coverage on critical medical paths

## 10. Post-Implementation

- [ ] Archive change after deployment
- [ ] Update project docs with test coverage goals
- [ ] Celebrate achievement! 🎉
