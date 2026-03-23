# Test Coverage Analysis & Comprehensive Test Strategy
## Kepler2-WGPU Medical Imaging Software

---

## 1. Current Coverage Assessment

### Overall Metrics
- **Total Source Files**: 51 files (~16,828 LOC)
- **Test Files**: 10 files (~3,768 LOC)
- **Overall Coverage**: 
  - Function: 24.18%
  - Line: 23.65%
  - Region: 23.33%

### Coverage by Module

| Module | Files | Avg Func% | Avg Line% | Avg Region% | Status |
|--------|--------|-----------|-----------|-------------|--------|
| **Core** | 1 | 83.33% | 95.06% | 98.22% | ✅ GOOD |
| **Data** | 1 | 21.43% | 52.94% | 32.37% | ⚠️ MODERATE |
| **Rendering** | 25+ | ~23% | ~21% | ~21% | ❌ CRITICAL |
| **Application** | 5+ | 0% | 0% | 0% | ❌ CRITICAL |

### Critical Coverage Gaps (< 30%)

**Zero Coverage Files (5):**
- application/app.rs (0%)
- application/appview.rs (0%)
- application/render_app.rs (0%)
- data/dicom/fileio.rs (0%)
- rendering/core/graphics.rs (0%)

**Severely Undercovered (< 10%):**
- data/dicom/image_series.rs (13%)
- data/dicom/patient.rs (16%)
- data/dicom/studyset.rs (12%)
- data/dicom/export_dicom.rs (8%)
- data/medical_imaging/common.rs (0%)
- data/medical_imaging/mhd.rs (0%)

---

## 2. Critical Medical Imaging Pathways

### High-Risk Areas (Patient Safety)

1. **DICOM Parsing & Validation**
   - File: `src/data/dicom/ct_image.rs` (Critical: CTImage::from_bytes, get_pixel_data)
   - Risk: Incorrect parsing → misdiagnosis
   - Current: Only 2 calls covered, many branches uncovered

2. **Patient Metadata Handling**
   - File: `src/data/dicom/patient.rs` (16%)
   - Risk: Patient identification errors → wrong treatment
   - Missing: Validation tests for mandatory fields

3. **CT Volume Generation**
   - File: `src/data/ct_volume.rs` (partial)
   - Risk: Incorrect volume reconstruction → misalignment
   - Missing: Coordinate transformation tests

4. **MPR Coordinate Transformations**
   - Files: `mpr_view.rs` (8%), `mpr_view_wgpu_impl.rs` (5%)
   - Risk: Slice position errors → wrong anatomy displayed
   - Missing: Matrix transformation edge cases

5. **Window/Level Processing**
   - File: `src/core/window_level.rs` (95% - GOOD)
   - Status: Well-tested, critical for CT display

### Medium-Risk Areas

1. **MHA/MHD File Parsing**
   - Files: `mha.rs`, `mhd.rs` (0-52%)
   - Risk: Format errors → data corruption
   - Missing: Endianness handling, compression support

2. **GPU Pipeline Management**
   - Files: `pipeline.rs` (5%), `texture.rs` (0%)
   - Risk: Rendering errors → display artifacts
   - Missing: Pipeline recreation on format changes

---

## 3. Comprehensive Test Strategy for Medical Software

### Phase 1: Medical Safety Critical Tests (Priority 1)
**Timeline**: Immediate (Weeks 1-2)
**Coverage Target**: 80%+ for patient safety code

#### 1.1 DICOM Validation Tests
```rust
// tests/dicom_validation_critical_tests.rs
mod critical_dicom_validation {
    // Mandatory field validation
    #[test]
    fn test_missing_sop_instance_uid_rejected() {
        // Test that CTImage::from_bytes rejects files without SOPInstanceUID
        // This is CRITICAL for patient safety
    }

    #[test]
    fn test_missing_pixel_representation_rejected() {
        // Test rejection when pixel_representation is missing
        // This determines Hounsfield unit calculation
    }

    #[test]
    fn test_rescale_slope_intercept_handling() {
        // Test edge cases:
        // - Missing rescale slope/intercept → default values
        // - Zero rescale slope → division by zero protection
        // - Negative rescale slope → inverted Hounsfield units
    }

    #[test]
    fn test_pixel_representation_edge_cases() {
        // Test all supported pixel representations:
        // - 0: Unsigned to signed conversion
        // - 1: Signed passthrough
        // - Invalid values → proper error
    }
}
```

#### 1.2 Patient Identity Tests
```rust
// tests/patient_safety_tests.rs
mod patient_identity {
    #[test]
    fn test_patient_id_uniqueness() {
        // Verify patient ID is properly extracted
        // Test collisions with multiple patients
    }

    #[test]
    fn test_study_series_integrity() {
        // Verify series belong to correct study
        // Verify images belong to correct series
    }

    #[test]
    fn test_dicom_uid_generation() {
        // Test UID generation follows DICOM standard
        // Verify uniqueness across multiple calls
    }
}
```

#### 1.3 Coordinate Transformation Tests
```rust
// tests/coordinate_safety_tests.rs
mod coordinate_safety {
    #[test]
    fn test_mpr_slice_position_bounds() {
        // Test that slice positions are clamped to valid range
        // Test negative positions
        // Test positions beyond volume bounds
    }

    #[test]
    fn test_orientation_matrix_orthogonality() {
        // Verify axial/coronal/sagittal matrices are orthogonal
        // Test matrix determinants = ±1
    }

    #[test]
    fn test_world_to_voxel_coordinate_precision() {
        // Test coordinate transformations preserve precision
        // Test floating point accumulation errors
    }
}
```

### Phase 2: Data Integrity Tests (Priority 2)
**Timeline**: Short-term (Weeks 3-4)
**Coverage Target**: 60%+ for data layer

#### 2.1 File Format Parsing Tests
```rust
// tests/file_format_integrity_tests.rs
mod format_parsing {
    #[test]
    fn test_mha_header_corruption_detection() {
        // Test corrupted headers are rejected
        // Test missing mandatory fields
        // Test invalid data offsets
    }

    #[test]
    fn test_mhd_external_data_file_handling() {
        // Test MHD with separate data file
        // Test missing data file → proper error
        // Test data file path resolution
    }

    #[test]
    fn test_endianness_conversion() {
        // Test big-endian and little-endian
        // Test mixed-endian scenarios
        // Test byte-swapping for all pixel types
    }

    #[test]
    fn test_compression_support() {
        // Test raw (uncompressed) parsing
        // Test zlib compression (if supported)
        // Test invalid compression → graceful error
    }
}
```

#### 2.2 Volume Data Tests
```rust
// tests/volume_integrity_tests.rs
mod volume_data {
    #[test]
    fn test_ct_volume_dimensions_validation() {
        // Test valid dimensions (1-2048 range)
        // Test zero dimensions → rejection
        // Test oversized volumes → memory limit protection
    }

    #[test]
    fn test_voxel_spacing_validation() {
        // Test zero/negative spacing → rejection
        // Test realistic spacing (0.1-10.0 mm range)
        // Test asymmetric spacing handling
    }

    #[test]
    fn test_volume_data_integrity_checks() {
        // Test voxel data size matches dimensions
        // Test data corruption detection
        // Test overflow/underflow protection
    }
}
```

### Phase 3: Rendering Correctness Tests (Priority 3)
**Timeline**: Medium-term (Weeks 5-6)
**Coverage Target**: 50%+ for rendering layer

#### 3.1 GPU Pipeline Tests
```rust
// tests/rendering_correctness_tests.rs
mod gpu_rendering {
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    async fn test_pipeline_creation_failure() {
        // Test GPU creation failures
        // Test surface format changes trigger recreation
        // Test shader compilation errors
    }

    #[test]
    fn test_texture_format_compatibility() {
        // Test all supported texture formats
        // Test format conversion edge cases
        // Test unsupported format → graceful fallback
    }

    #[test]
    fn test_view_manager_state_consistency() {
        // Test concurrent view creation/destruction
        // Test state transitions are atomic
        // Test memory cleanup on view removal
    }
}
```

#### 3.2 Visual Correctness Tests
```rust
// tests/visual_correctness_tests.rs
mod visual_correctness {
    #[test]
    fn test_window_level_clamping() {
        // Test window center/width bounds
        // Test extreme values → clamping
        // Test preservation of contrast ratios
    }

    #[test]
    fn test_aspect_ratio_preservation() {
        // Test aspect fit calculations
        // Test pillarbox/letterbox scenarios
        // Test anisotropic voxel spacing
    }

    #[test]
    fn test_mesh_quality_levels() {
        // Test quality level transitions
        // Test LOD (level-of-detail) correctness
        // Test mesh simplification preserves topology
    }
}
```

### Phase 4: Error Handling & Robustness (Priority 4)
**Timeline**: Ongoing
**Coverage Target**: 70%+ for error paths

#### 4.1 Error Propagation Tests
```rust
// tests/error_propagation_tests.rs
mod error_handling {
    #[test]
    fn test_file_not_found_error_chain() {
        // Test file I/O errors propagate correctly
        // Test error context is preserved
        // Test user-friendly error messages
    }

    #[test]
    fn test_memory_pressure_handling() {
        // Test OOM scenarios
        // Test graceful degradation
        // Test cleanup on allocation failure
    }

    #[test]
    fn test_concurrent_error_recovery() {
        // Test async operations handle errors individually
        // Test partial file processing continues
        // Test error isolation between tasks
    }
}
```

#### 4.2 Edge Case Tests
```rust
// tests/edge_case_tests.rs
mod edge_cases {
    #[test]
    fn test_single_slice_volume() {
        // Test 1-slice CT volumes
        // Test 2D image handling
    }

    #[test]
    fn test_maximum_dimensions_volume() {
        // Test volume at 2048x2048x2048
        // Test memory allocation limits
    }

    #[test]
    fn test_empty_dicom_series() {
        // Test series with 0 images
        // Test repository with 0 series
    }
}
```

### Phase 5: Performance & Regression Tests (Priority 5)
**Timeline**: Ongoing with CI integration

#### 5.1 Performance Benchmarks
```rust
// tests/performance_benchmarks.rs
mod performance {
    #[test]
    fn benchmark_dicom_parsing_512x512() {
        // Benchmark 512x512 image parsing
        // Compare against baseline (< 10ms)
    }

    #[test]
    fn benchmark_volume_rendering_512x512x100() {
        // Benchmark volume rendering
        // Target: 60 FPS for 512x512x100
    }

    #[test]
    fn test_memory_usage_large_volume() {
        // Verify memory usage is bounded
        // Test memory leaks with repeated loading/unloading
    }
}
```

#### 5.2 Regression Tests
```rust
// tests/regression_tests.rs
mod regression {
    // Document and prevent recurrence of bugs
    
    #[test]
    fn regression_issue_001_pixel_rescaling() {
        // Test: Fix for incorrect pixel rescaling
        // Reference: GitHub Issue #001
    }

    #[test]
    fn regression_issue_002_orientation_matrix() {
        // Test: Fix for sagittal matrix bug
        // Reference: GitHub Issue #002
    }
}
```

---

## 4. Testing Infrastructure Improvements

### 4.1 Test Utilities & Fixtures

Create shared test infrastructure:

```rust
// tests/common/test_fixtures.rs
pub mod fixtures {
    pub use kepler_wgpu::data::{CTImage, Patient, StudySet, ImageSeries};
    
    /// Reusable DICOM test data
    pub fn create_test_ct_image() -> CTImage {
        // Generate valid DICOM CT image with all mandatory fields
    }
    
    pub fn create_test_patient(id: &str) -> Patient {
        // Generate valid patient with test ID
    }
    
    pub fn create_test_volume_512x512x100() -> CTVolume {
        // Generate 512x512x100 test volume
    }
    
    /// Common test DICOM files (embed binary data)
    pub const SAMPLE_AXIAL_CT: &[u8] = include_bytes!("../test_data/axial_ct.dcm");
    pub const SAMPLE_CORONAL_CT: &[u8] = include_bytes!("../test_data/coronal_ct.dcm");
    pub const SAMPLE_SAGITTAL_CT: &[u8] = include_bytes!("../test_data/sagittal_ct.dcm");
}

/// Mock GPU device for rendering tests
pub mod gpu_mocks {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn create_mock_device() -> wgpu::Device {
        // Create test-compatible GPU device
    }
}
```

### 4.2 Property-Based Testing

Add property-based tests for mathematical correctness:

```toml
# Cargo.toml
[dev-dependencies]
proptest = "1.4"
quickcheck = "1.0"
```

```rust
// tests/property_tests.rs
mod property_based {
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn test_window_level_preserves_range(window in -1000.0..2000.0_f32,
                                        level in -1000.0..1000.0_f32) {
            // Property: Window/level transformation preserves input range
            let wl = WindowLevel::new(window, level);
            // Verify: f(level) = window/2 (midpoint property)
        }
        
        #[test]
        fn test_matrix_determinant_unity(elements in -1.0..1.0_f32) {
            // Property: Orientation matrices should have det = ±1
            let matrix = Mat4::from_cols_array_2d(&[...]);
            // Verify: abs(det - 1.0) < epsilon
        }
    }
}
```

### 4.3 Fuzz Testing

For security-critical parsing:

```toml
[dev-dependencies]
cargo-fuzz = "0.11"
```

```bash
# Fuzz DICOM parser
cargo fuzz run dicom_parser_fuzzer

# Fuzz MHA header parser
cargo fuzz run mha_header_fuzzer
```

---

## 5. CI/CD Integration

### 5.1 Coverage Gates

```yaml
# .github/workflows/test.yml
name: Test Coverage

on: [push, pull_request]

jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Run tests with coverage
        run: |
          cargo llvm-cov --html --output-dir coverage
          
      - name: Upload coverage to Codecov
        uses: codecov/codecov-action@v3
        
      - name: Coverage Gate
        run: |
          # Critical medical paths must have > 80% coverage
          DATA_COVERAGE=$(scripts/check_coverage.sh data 80)
          RENDERING_COVERAGE=$(scripts/check_coverage.sh rendering 50)
          
          if [ "$DATA_COVERAGE" -lt "80" ]; then
            echo "❌ Data coverage $DATA_COVERAGE% below 80% threshold"
            exit 1
          fi
```

### 5.2 Pre-commit Hooks

```bash
# .git/hooks/pre-commit
#!/bin/bash
cargo test --quiet
cargo clippy -- -D warnings

# Run critical medical tests
cargo test --test patient_safety_tests
cargo test --test dicom_validation_critical_tests
```

---

## 6. Implementation Roadmap

### Month 1: Medical Safety Foundation
- [ ] Week 1-2: Implement Phase 1 (DICOM validation, patient identity, coordinates)
- [ ] Add test fixtures repository
- [ ] Set up coverage gates in CI

### Month 2: Data Integrity
- [ ] Week 3-4: Implement Phase 2 (format parsing, volume tests)
- [ ] Add property-based testing infrastructure
- [ ] Fuzz testing setup for parsers

### Month 3: Rendering & Robustness
- [ ] Week 5-6: Implement Phase 3 (rendering correctness)
- [ ] Week 7-8: Implement Phase 4 (error handling)
- [ ] GPU mock infrastructure

### Month 4+: Maintenance & Regression
- [ ] Week 9+: Implement Phase 5 (performance, regression)
- [ ] Ongoing: Add tests for each bug fix
- [ ] Ongoing: Maintain >80% coverage on critical paths

---

## 7. Success Metrics

### Coverage Targets

| Component | Current | Target (Month 3) | Target (Month 6) |
|-----------|----------|-------------------|-------------------|
| Data Layer (DICOM) | 20% | **80%** | 90% |
| Rendering Core | 23% | **60%** | 75% |
| Application Layer | 0% | **40%** | 60% |
| **Critical Medical Paths** | ~25% | **80%** | **90%** |
| **Overall** | 24% | **50%** | 70% |

### Quality Metrics

1. **Test Execution Time**: Full test suite < 5 minutes
2. **Flaky Test Rate**: < 1% (automatically disabled)
3. **False Positive Rate**: 0%
4. **Bug Detection Rate**: > 90% of regressions caught by tests

---

## 8. Medical Software-Specific Considerations

### 8.1 Regulatory Compliance

For FDA/CE marking, ensure:

- ✅ **Traceability**: Each test maps to a requirement
- ✅ **Reproducibility**: Tests are deterministic (no randomness without seed)
- ✅ **Documentation**: Test purpose and expected outcome documented
- ✅ **Validation**: Critical algorithms verified by independent means

### 8.2 Clinical Safety Rules

1. **Never silently fall back**: Reject invalid data, never guess
2. **Always validate inputs**: Patient safety > convenience
3. **Log all rejections**: Error logs must indicate why data was rejected
4. **Preserve precision**: Don't lose medical accuracy for performance

### 8.3 Data Protection

- ✅ No real patient data in tests (use synthetic data)
- ✅ Test data anonymized (remove all PHIs)
- ✅ Secure test artifact handling (no credentials in logs)

---

## 9. Recommended Immediate Actions

### Week 1 Priorities

1. **Add Critical DICOM Validation Tests**
   ```bash
   # Create new test file
   touch tests/dicom_validation_critical_tests.rs
   # Implement 10+ mandatory field validation tests
   ```

2. **Implement Test Fixtures**
   ```bash
   mkdir tests/common
   touch tests/common/test_fixtures.rs
   # Add reusable test data generators
   ```

3. **Setup Coverage Reporting**
   ```bash
   # Add to CI
   - Run coverage script on every PR
   - Fail if critical coverage drops below 80%
   ```

4. **Document Critical Paths**
   ```bash
   # Create test requirements document
   touch doc/medical_testing_requirements.md
   # Map tests to patient safety requirements
   ```

---

## Conclusion

Kepler2-WGPU has a **critical need for improved test coverage**, especially in medical safety pathways. The current 24% coverage is **insufficient for clinical software**.

**Key Priorities:**
1. Immediate: Reach 80%+ coverage on DICOM parsing and patient metadata
2. Short-term: Reach 60%+ on data integrity and coordinate transformations  
3. Medium-term: Reach 50%+ on GPU rendering and visualization
4. Long-term: Maintain >80% on all patient-critical paths

**Medical software demands higher standards than general-purpose applications.** This strategy ensures patient safety through rigorous testing while maintaining development velocity through proper infrastructure.
