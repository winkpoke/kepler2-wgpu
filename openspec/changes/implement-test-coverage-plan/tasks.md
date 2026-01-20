# Test Coverage Implementation Tasks

## 1. Infrastructure Setup (Week 1)

- [ ] 1.1 Create test fixtures module
  - [ ] Create `tests/common/` directory
  - [ ] Create `tests/common/mod.rs`
  - [ ] Implement `create_test_ct_image()` - valid DICOM CT with all fields
  - [ ] Implement `create_test_patient(id: &str)` - patient with test ID
  - [ ] Implement `create_test_volume_512x512x100()` - standard test volume
  - [ ] Add `create_invalid_dicom_missing_uid()` - for rejection tests
  - [ ] Add `create_dicom_with_invalid_rescale()` - for edge cases
  - [ ] Add helper for creating MHA/MHD test data
  - [ ] Add helper for creating malformed DICOM data

- [ ] 1.2 Set up property-based testing
  - [ ] Add `proptest = "1.4"` to dev-dependencies in Cargo.toml
  - [ ] Add `quickcheck = "1.0"` to dev-dependencies in Cargo.toml
  - [ ] Create `tests/property_tests.rs` scaffold

- [ ] 1.3 Update test infrastructure documentation
  - [ ] Document test naming conventions in doc/agents/
  - [ ] Add test fixture usage examples
  - [ ] Document property-based testing approach

## 2. Phase 1: Medical Safety Critical (Week 1-2)

### Week 1: DICOM Validation

- [ ] 2.1 Implement DICOM mandatory field validation
  - [ ] Create `tests/dicom_validation_critical_tests.rs`
  - [ ] Test missing SOPInstanceUID rejection
  - [ ] Test missing SeriesInstanceUID rejection
  - [ ] Test missing Rows rejection
  - [ ] Test missing Columns rejection
  - [ ] Test missing PixelRepresentation rejection
  - [ ] Test missing PixelData rejection
  - [ ] Test empty DICOM data rejection
  - [ ] Test corrupted DICOM header rejection
  - [ ] Test truncated DICOM file rejection
  - [ ] Test valid DICOM accepted

- [ ] 2.2 Implement DICOM rescaling edge cases
  - [ ] Test rescale slope/intercept defaults when missing
  - [ ] Test zero rescale slope handling
  - [ ] Test negative rescale slope inverts units
  - [ ] Test large rescale values handled
  - [ ] Test pixel representation 0: unsigned to signed conversion
  - [ ] Test pixel representation 1: signed passthrough
  - [ ] Test invalid pixel representation rejected
  - [ ] Test rescaling preserves precision

- [ ] 2.3 Verify DICOM validation coverage
  - [ ] Run `cargo llvm-cov` on DICOM tests
  - [ ] Verify `CTImage::from_bytes` coverage ≥ 60%
  - [ ] Verify `CTImage::get_pixel_data` coverage ≥ 85%

### Week 2: Patient Identity & Coordinates

- [ ] 2.4 Implement patient identity tests
  - [ ] Create `tests/patient_safety_tests.rs`
  - [ ] Test patient ID extraction
  - [ ] Test patient name formatting
  - [ ] Test patient birth date parsing
  - [ ] Test patient sex validation
  - [ ] Test multiple patients distinct
  - [ ] Test patient ID empty rejected
  - [ ] Test study series integrity
  - [ ] Test series images count
  - [ ] Test series modality verification
  - [ ] Test image belongs to correct series
  - [ ] Test UID format validation
  - [ ] Test UID uniqueness

- [ ] 2.5 Implement coordinate transformation safety
  - [ ] Create `tests/coordinate_safety_tests.rs`
  - [ ] Test MPR slice position negative clamped
  - [ ] Test MPR slice position exceeds max clamped
  - [ ] Test axial matrix orthogonal
  - [ ] Test coronal matrix orthogonal
  - [ ] Test sagittal matrix orthogonal
  - [ ] Test world to voxel coordinate precision
  - [ ] Test voxel to world roundtrip
  - [ ] Test anatomical orientation validity
  - [ ] Test slice thickness validation
  - [ ] Test voxel spacing validation

- [ ] 2.6 Verify Phase 1 coverage targets
  - [ ] Verify patient identity coverage: patient.rs ≥ 75%, studyset.rs ≥ 70%, image_series.rs ≥ 70%
  - [ ] Verify coordinate transformation coverage: mpr_view.rs ≥ 60%, mpr_view_wgpu_impl.rs ≥ 50%
  - [ ] Run full test suite: `cargo test`
  - [ ] Generate coverage report: `cargo llvm-cov --html`

## 3. Phase 2: Data Integrity (Weeks 3-4)

### Week 3: File Format Parsing

- [ ] 3.1 Implement MHA format tests
  - [ ] Test MHA header mandatory fields present
  - [ ] Test MHA header corruption detection
  - [ ] Test MHA missing data offset rejected
  - [ ] Test MHA endianness little
  - [ ] Test MHA endianness big
  - [ ] Test MHA pixel type UInt8
  - [ ] Test MHA pixel type UInt16
  - [ ] Test MHA pixel type Int16
  - [ ] Test MHA pixel type Float32
  - [ ] Test MHA unsupported pixel type rejected
  - [ ] Test MHA dimension validation
  - [ ] Test MHA spacing validation
  - [ ] Test MHA comment lines ignored

- [ ] 3.2 Implement MHD format tests
  - [ ] Test MHD external data file resolution
  - [ ] Test MHD missing data file error
  - [ ] Test MHD compressed data file
  - [ ] Test MHD raw data file
  - [ ] Test MHD binary data true
  - [ ] Test MHD binary data false rejected
  - [ ] Test MHD transform matrix validation
  - [ ] Test MHD offset validation
  - [ ] Test MHD anatomical orientation validation
  - [ ] Test MHD element data file local

- [ ] 3.3 Verify format parsing coverage
  - [ ] Verify MHA parser coverage ≥ 80%
  - [ ] Verify MHD parser coverage ≥ 60%

### Week 4: Volume Data & Property Tests

- [ ] 3.4 Implement volume integrity tests
  - [ ] Create `tests/volume_integrity_tests.rs`
  - [ ] Test CT volume dimensions validation
  - [ ] Test CT volume zero dimensions rejected
  - [ ] Test CT volume negative dimensions rejected
  - [ ] Test CT volume max dimensions handled
  - [ ] Test voxel spacing positive required
  - [ ] Test voxel spacing zero rejected
  - [ ] Test voxel spacing negative rejected
  - [ ] Test voxel data size matches dimensions
  - [ ] Test volume origin valid
  - [ ] Test volume orientation valid
  - [ ] Test volume crop bounds validation
  - [ ] Test volume empty data rejected

- [ ] 3.5 Implement property-based tests
  - [ ] Test window level preserves range property
  - [ ] Test scale clamping bounds property
  - [ ] Test pan distance bounds property
  - [ ] Test aspect fit preserves ratio property
  - [ ] Test matrix determinant rotation unity property
  - [ ] Test all property tests with proptest

- [ ] 3.6 Verify Phase 2 coverage targets
  - [ ] Verify volume data coverage: ct_volume.rs ≥ 70%
  - [ ] Verify all property tests passing
  - [ ] Run full test suite with coverage

## 4. Phase 3: Rendering Correctness (Weeks 5-6)

### Week 5: GPU Pipeline Tests

- [ ] 4.1 Implement GPU initialization tests (native-only)
  - [ ] Create `tests/rendering_correctness_tests.rs`
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

- [ ] 4.4 Implement view manager tests
  - [ ] Test view manager creation
  - [ ] Test view manager add view
  - [ ] Test view manager remove view
  - [ ] Test view manager multiple views
  - [ ] Test view manager active view
  - [ ] Test view manager view count
  - [ ] Test view manager concurrent operations
  - [ ] Test view manager state consistency
  - [ ] Test view manager memory cleanup
  - [ ] Test view manager iterate views
  - [ ] Test view manager find view by ID

- [ ] 4.5 Implement visual correctness tests
  - [ ] Create `tests/visual_correctness_tests.rs`
  - [ ] Test window level clamping center
  - [ ] Test window level clamping width
  - [ ] Test window level extreme values
  - [ ] Test window level preserves contrast
  - [ ] Test aspect fit letterbox
  - [ ] Test aspect fit pillarbox
  - [ ] Test aspect fit exact match
  - [ ] Test aspect fit square

- [ ] 4.6 Verify Phase 3 coverage targets
  - [ ] Verify view manager coverage ≥ 70%
  - [ ] Verify visual rendering coverage ≥ 60%

## 5. Phase 4: Error Handling & Robustness (Weeks 7-8)

### Week 7: Error Propagation & Edge Cases

- [ ] 5.1 Implement error propagation tests
  - [ ] Create `tests/error_propagation_tests.rs`
  - [ ] Test file not found error propagates
  - [ ] Test permission denied error propagates
  - [ ] Test parse error context preserved
  - [ ] Test error chain depth
  - [ ] Test error user friendly message
  - [ ] Test DICOM parse error includes tag
  - [ ] Test volume creation error includes dimensions
  - [ ] Test GPU allocation error message
  - [ ] Test surface creation error message
  - [ ] Test shader compilation error includes stage
  - [ ] Test texture upload error includes size
  - [ ] Test error recovery doesn't leak

- [ ] 5.2 Implement edge case tests
  - [ ] Create `tests/edge_case_tests.rs`
  - [ ] Test single slice volume
  - [ ] Test two pixel volume
  - [ ] Test maximum dimensions volume
  - [ ] Test empty DICOM series
  - [ ] Test empty patient
  - [ ] Test mixed pixel types in series
  - [ ] Test inconsistent spacing in series
  - [ ] Test corrupted pixel data recovery
  - [ ] Test invalid UID format
  - [ ] Test unicode patient name

### Week 8: Memory & Concurrency

- [ ] 5.3 Implement memory management tests
  - [ ] Create `tests/robustness_tests.rs`
  - [ ] Test large volume memory allocation
  - [ ] Test memory leak repeated load unload
  - [ ] Test texture memory cleanup
  - [ ] Test view manager memory cleanup
  - [ ] Test DICOM repo memory growth
  - [ ] Test concurrent volume parsing
  - [ ] Test concurrent view updates
  - [ ] Test GPU buffer reuse
  - [ ] Test buffer pool growth
  - [ ] Test out of memory graceful degradation

- [ ] 5.4 Verify Phase 4 coverage targets
  - [ ] Verify error path coverage ≥ 70%
  - [ ] Verify edge case coverage ≥ 60%
  - [ ] Verify memory tests passing

## 6. Phase 5: Performance & Regression (Weeks 9+ - Ongoing)

- [ ] 6.1 Implement performance benchmarks
   - [ ] Create `tests/performance_benchmarks.rs`
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

- [ ] 6.2 Implement regression test suite
   - [ ] Create `tests/regression_tests.rs`
   - [ ] Add template for regression test structure
   - [ ] Define naming convention: `regression_issue_NNN_symptom` (e.g., `regression_issue_123_window_clamp_crash`)
   - [ ] Add fallback format: `regression_module_symptom` for issues without GitHub issue numbers
   - [ ] Document naming convention in tasks.md with examples
   - [ ] Add example regression test (placeholder for first bug fix)
   - [ ] Add regression test execution marker (`#[ignore]` by default until bug is fixed)

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

- [ ] 8.1 Update test documentation
  - [ ] Document test infrastructure in doc/agents/
  - [ ] Add testing best practices guide
  - [ ] Document how to add regression tests
  - [ ] Document property-based testing approach

- [ ] 8.2 Create onboarding materials
  - [ ] Add testing tutorial for new contributors
  - [ ] Document test fixture usage
  - [ ] Provide examples of common test patterns

## 9. Validation & Success Criteria

### Phase 1 Completion (Week 2)
- [ ] 37+ new tests implemented
- [ ] All DICOM mandatory fields validated
- [ ] Patient identity coverage ≥ 70%
- [ ] Coordinate transformation coverage ≥ 50%
- [ ] Overall coverage ≥ 45%
- [ ] Zero critical gaps in medical paths
- [ ] All tests passing: `cargo test`

### Phase 2 Completion (Week 4)
- [ ] 36+ new tests implemented
- [ ] MHA/MHD coverage ≥ 60%
- [ ] Volume integrity coverage ≥ 70%
- [ ] Property-based testing working
- [ ] Overall coverage ≥ 60%

### Phase 3 Completion (Week 6)
- [ ] 36+ new tests implemented
- [ ] GPU initialization coverage ≥ 40%
- [ ] Rendering coverage ≥ 45%
- [ ] All GPU tests passing (native)
- [ ] Overall coverage ≥ 65%

### Phase 4 Completion (Week 8)
- [ ] 29+ new tests implemented
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
