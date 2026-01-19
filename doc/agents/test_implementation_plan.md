# Test Coverage Improvement Plan
## Kepler2-WGPU Medical Imaging Software

**Status**: Active Implementation Plan  
**Based on**: `test_strategy_comprehensive.md`  
**Target**: 80% coverage on critical medical paths (3 months)

---

## 📋 Overview

This plan breaks down the comprehensive test strategy into **concrete, actionable tasks** with file names, test counts, and effort estimates.

**Priority Order**:
1. **Patient Safety** - Immediate (Weeks 1-2)
2. **Data Integrity** - Short-term (Weeks 3-4)
3. **Rendering Correctness** - Medium-term (Weeks 5-6)
4. **Error Handling** - Ongoing (Weeks 7-8)
5. **Performance & Regression** - Ongoing (Weeks 9+)

---

## 🚨 Phase 1: Medical Safety Critical (Weeks 1-2)

### Week 1: DICOM Validation Infrastructure

**Objective**: Establish test infrastructure and cover critical DICOM parsing

#### Task 1.1: Create Test Fixtures (Day 1)
**File**: `tests/common/test_fixtures.rs` (NEW)  
**Effort**: 2 hours

**Deliverables**:
- [ ] Create `tests/common/` directory
- [ ] Implement `create_test_ct_image()` - valid DICOM CT with all fields
- [ ] Implement `create_test_patient(id: &str)` - patient with test ID
- [ ] Implement `create_test_volume_512x512x100()` - standard test volume
- [ ] Add `create_invalid_dicom_missing_uid()` - for rejection tests
- [ ] Add `create_dicom_with_invalid_rescale()` - for edge cases

**Impact**: Foundation for all DICOM tests

---

#### Task 1.2: DICOM Mandatory Field Validation (Days 2-3)
**File**: `tests/dicom_validation_critical_tests.rs` (NEW)  
**Effort**: 6 hours

**Deliverables** (10 tests):

```rust
// Critical: Patient safety - reject invalid data early

#[test]
fn test_missing_sop_instance_uid_rejected() {
    // Verify CTImage::from_bytes returns Err when SOPInstanceUID missing
    // Expected: Err("Missing SOPInstanceUID")
}

#[test]
fn test_missing_series_instance_uid_rejected() {
    // Verify rejection when SeriesInstanceUID missing
}

#[test]
fn test_missing_rows_rejected() {
    // Verify rejection when Rows (0028,0010) missing
}

#[test]
fn test_missing_columns_rejected() {
    // Verify rejection when Columns (0028,0011) missing
}

#[test]
fn test_missing_pixel_representation_rejected() {
    // Critical for Hounsfield unit calculation
}

#[test]
fn test_missing_pixel_data_rejected() {
    // PixelData (7FE0,0010) is mandatory
}

#[test]
fn test_empty_dicom_data_rejected() {
    // Zero-length input should fail
}

#[test]
fn test_corrupted_dicom_header_rejected() {
    // Invalid DICOM prefix "DICM" should fail
}

#[test]
fn test_truncated_dicom_file_rejected() {
    // Incomplete file should return parse error
}

#[test]
fn test_valid_dicom_accepted() {
    // Verify complete valid DICOM parses successfully
}
```

**Coverage Target**: `src/data/dicom/ct_image.rs` from ~25% → 60%

---

#### Task 1.3: DICOM Rescaling Tests (Days 4-5)
**File**: `tests/dicom_validation_critical_tests.rs` (APPEND)  
**Effort**: 6 hours

**Deliverables** (8 tests):

```rust
// Critical: Incorrect rescaling → wrong Hounsfield units → misdiagnosis

#[test]
fn test_rescale_slope_intercept_defaults_when_missing() {
    // Missing values should default to 1.0 and 0.0
}

#[test]
fn test_rescale_slope_zero_handled() {
    // Zero rescale slope should be protected against
    // Or explicitly handled with error
}

#[test]
fn test_negative_rescale_slope_inverts_units() {
    // Negative slope inverts Hounsfield scale
    // Verify this is correctly applied
}

#[test]
fn test_large_rescale_values_handled() {
    // Test extreme values (e.g., slope = 1000.0)
    // Verify no overflow/underflow
}

#[test]
fn test_pixel_representation_0_unsigned_conversion() {
    // Pixel representation 0: unsigned → signed conversion
    // Verify byte swapping is correct
}

#[test]
fn test_pixel_representation_1_signed_passthrough() {
    // Pixel representation 1: already signed
    // Verify no conversion needed
}

#[test]
fn test_invalid_pixel_representation_rejected() {
    // Values other than 0 or 1 should error
}

#[test]
fn test_rescaling_preserves_precision() {
    // Verify floating-point precision maintained
    // Test with known pixel values and expected outputs
}
```

**Coverage Target**: `CTImage::get_pixel_data()` from ~40% → 85%

---

### Week 2: Patient Identity & Coordinate Safety

#### Task 1.4: Patient Identity Tests (Days 1-3)
**File**: `tests/patient_safety_tests.rs` (NEW)  
**Effort**: 8 hours

**Deliverables** (12 tests):

```rust
// Critical: Patient identification errors → wrong treatment

#[test]
fn test_patient_id_extraction() {
    // Verify patient ID correctly extracted from DICOM
}

#[test]
fn test_patient_name_formatting() {
    // Verify patient name handling (PN field)
}

#[test]
fn test_patient_birth_date_parsing() {
    // Verify DA (Date) field parsing
}

#[test]
fn test_patient_sex_validation() {
    // Verify sex field validation (M/F/O/other)
}

#[test]
fn test_multiple_patients_distinct() {
    // Verify no UID collisions across patients
}

#[test]
fn test_patient_id_empty_rejected() {
    // Empty patient ID should be invalid
}

#[test]
fn test_study_series_integrity() {
    // Verify all series belong to correct study UID
}

#[test]
fn test_series_images_count() {
    // Verify image count matches actual images
}

#[test]
fn test_series_modality_verification() {
    // Verify modality is "CT" for CT volumes
}

#[test]
fn test_image_belongs_to_correct_series() {
    // Verify image SeriesInstanceUID matches parent
}

#[test]
fn test_uid_format_validation() {
    // Verify generated UIDs follow DICOM standard (2.25.x format)
}

#[test]
fn test_uid_uniqueness() {
    // Generate 1000 UIDs, verify all unique
}
```

**Coverage Target**: 
- `src/data/dicom/patient.rs` from 16% → 75%
- `src/data/dicom/studyset.rs` from 12% → 70%
- `src/data/dicom/image_series.rs` from 13% → 70%

---

#### Task 1.5: Coordinate Transformation Safety (Days 4-5)
**File**: `tests/coordinate_safety_tests.rs` (NEW)  
**Effort**: 6 hours

**Deliverables** (10 tests):

```rust
// Critical: Coordinate errors → wrong anatomy displayed

#[test]
fn test_mpr_slice_position_negative_clamped() {
    // Test slice position < 0 → clamped to 0
}

#[test]
fn test_mpr_slice_position_exceeds_max_clamped() {
    // Test slice position > volume depth → clamped to max
}

#[test]
fn test_axial_matrix_orthogonal() {
    // Verify axial orientation matrix is orthogonal
    // det(M) = 1
}

#[test]
fn test_coronal_matrix_orthogonal() {
    // Verify coronal orientation matrix is orthogonal
}

#[test]
fn test_sagittal_matrix_orthogonal() {
    // Verify sagittal orientation matrix is orthogonal
}

#[test]
fn test_world_to_voxel_coordinate_precision() {
    // Test transformations preserve precision
    // Verify error < epsilon after round-trip
}

#[test]
fn test_voxel_to_world_roundtrip() {
    // Test world → voxel → world returns original
}

#[test]
fn test_anatomical_orientation_validity() {
    // Verify RAI/LPI/etc orientations are valid
}

#[test]
fn test_slice_thickness_validation() {
    // Verify slice thickness > 0 enforced
}

#[test]
fn test_voxel_spacing_validation() {
    // Verify spacing values > 0 enforced
}
```

**Coverage Target**:
- `src/rendering/view/mpr/mpr_view.rs` from 8% → 60%
- `src/rendering/view/mpr/mpr_view_wgpu_impl.rs` from 5% → 50%

---

## ✅ Phase 1 Completion Checklist

**Week 1-2 Deliverables**:
- [ ] 3 new test files created
- [ ] 40+ new tests implemented
- [ ] `tests/common/test_fixtures.rs` infrastructure complete
- [ ] DICOM validation coverage: 60%+
- [ ] Patient identity coverage: 70%+
- [ ] Coordinate transformation coverage: 50%+
- [ ] All tests passing: `cargo test`
- [ ] Coverage report generated: `cargo llvm-cov --html`

**Expected Coverage Improvement**:
- **DICOM Layer**: 20% → **60%**
- **Critical Medical Paths**: ~25% → **70%**

---

## 📊 Phase 2: Data Integrity Tests (Weeks 3-4)

### Week 3: File Format Parsing

#### Task 2.1: MHA Format Tests (Days 1-3)
**File**: `tests/file_format_integrity_tests.rs` (NEW)  
**Effort**: 8 hours

**Deliverables** (14 tests):

```rust
// Medium Risk: Format errors → data corruption

#[test]
fn test_mha_header_mandatory_fields_present() {
    // ObjectType, NDims, DimSize, ElementType required
}

#[test]
fn test_mha_header_corruption_detection() {
    // Invalid header should return parse error
}

#[test]
fn test_mha_missing_data_offset_rejected() {
    // No newline before data → invalid
}

#[test]
fn test_mha_endianness_little() {
    // Verify little-endian byte order handled
}

#[test]
fn test_mha_endianness_big() {
    // Verify big-endian byte order handled
}

#[test]
fn test_mha_pixel_type_uint8() {
    // Test MET_UCHAR pixel type
}

#[test]
fn test_mha_pixel_type_uint16() {
    // Test MET_USHORT pixel type
}

#[test]
fn test_mha_pixel_type_int16() {
    // Test MET_SHORT pixel type
}

#[test]
fn test_mha_pixel_type_float32() {
    // Test MET_FLOAT pixel type
}

#[test]
fn test_mha_unsupported_pixel_type_rejected() {
    // Unknown ElementType should error
}

#[test]
fn test_mha_dimension_validation() {
    // DimSize values must be positive integers
}

#[test]
fn test_mha_spacing_validation() {
    // ElementSpacing must be positive floats
}

#[test]
fn test_mha_comment_lines_ignored() {
    // Lines with # should be skipped
}
```

**Coverage Target**: `src/data/medical_imaging/formats/mha.rs` from ~40% → 80%

---

#### Task 2.2: MHD Format Tests (Days 4-5)
**File**: `tests/file_format_integrity_tests.rs` (APPEND)  
**Effort**: 6 hours

**Deliverables** (10 tests):

```rust
#[test]
fn test_mhd_external_data_file_resolution() {
    // Test path resolution to separate .raw/.zraw file
}

#[test]
fn test_mhd_missing_data_file_error() {
    // Missing external file should return clear error
}

#[test]
fn test_mhd_compressed_data_file() {
    // Test .zraw (compressed) file handling
}

#[test]
fn test_mhd_raw_data_file() {
    // Test .raw (uncompressed) file handling
}

#[test]
fn test_mhd_binary_data_true() {
    // BinaryData = True required
}

#[test]
fn test_mhd_binary_data_false_rejected() {
    // ASCII format not supported → error
}

#[test]
fn test_mhd_transform_matrix_validation() {
    // Verify 3x3 matrix is valid
}

#[test]
fn test_mhd_offset_validation() {
    // Verify 3D offset vector parsed correctly
}

#[test]
fn test_mhd_anatomical_orientation_validation() {
    // Verify RAI/LPI/etc orientation parsed
}

#[test]
fn test_mhd_element_data_file_local() {
    // Verify ElementDataFile = LOCAL for embedded data
}
```

**Coverage Target**: `src/data/medical_imaging/formats/mhd.rs` from 0% → 60%

---

### Week 4: Volume Data & Property Tests

#### Task 2.3: Volume Integrity Tests (Days 1-3)
**File**: `tests/volume_integrity_tests.rs` (NEW)  
**Effort**: 8 hours

**Deliverables** (12 tests):

```rust
// Medium Risk: Volume errors → misalignment

#[test]
fn test_ct_volume_dimensions_validation() {
    // Verify dimensions (1-2048) enforced
}

#[test]
fn test_ct_volume_zero_dimensions_rejected() {
    // 0x0x0 dimensions should error
}

#[test]
fn test_ct_volume_negative_dimensions_rejected() {
    // Negative dimensions invalid
}

#[test]
fn test_ct_volume_max_dimensions_handled() {
    // Test 2048x2048x2048 volume
    // Verify memory allocation succeeds
}

#[test]
fn test_voxel_spacing_positive_required() {
    // Verify spacing > 0 enforced
}

#[test]
fn test_voxel_spacing_zero_rejected() {
    // Zero spacing → invalid
}

#[test]
fn test_voxel_spacing_negative_rejected() {
    // Negative spacing → invalid
}

#[test]
fn test_voxel_data_size_matches_dimensions() {
    // Verify data length = rows * cols * slices
}

#[test]
fn test_volume_origin_valid() {
    // Verify base matrix origin set correctly
}

#[test]
fn test_volume_orientation_valid() {
    // Verify base matrix orientation preserves axes
}

#[test]
fn test_volume_crop_bounds_validation() {
    // Test crop region within volume bounds
}

#[test]
fn test_volume_empty_data_rejected() {
    // Empty voxel data array should error
}
```

**Coverage Target**: `src/data/ct_volume.rs` from ~30% → 70%

---

#### Task 2.4: Property-Based Tests (Days 4-5)
**File**: `tests/property_tests.rs` (NEW)  
**Prerequisite**: Add `proptest = "1.4"` to dev-dependencies  
**Effort**: 6 hours

**Deliverables** (8 property tests):

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_window_level_preserves_range(window in -1000.0..2000.0_f32,
                                         level in -1000.0..1000.0_f32) {
        // Property: f(level) = window/2 (midpoint)
        let wl = WindowLevel::new(window, level);
        let result = wl.transform(level);
        let expected = window / 2.0;
        prop_assert!((result - expected).abs() < 0.01);
    }

    #[test]
    fn test_scale_clamping_bounds(scale in 0.0..200.0_f32) {
        // Property: Scale clamped to [0.01, 100.0]
        let clamped = scale.clamp(0.01, 100.0);
        prop_assert!(clamped >= 0.01 && clamped <= 100.0);
    }

    #[test]
    fn test_pan_distance_bounds(pan_x in -20000.0..20000.0_f32,
                                 pan_y in -20000.0..20000.0_f32,
                                 pan_z in -20000.0..20000.0_f32) {
        // Property: Pan clamped to ±10000
        let pan = Vec3::new(pan_x, pan_y, pan_z);
        let clamped = pan.clamp(Vec3::splat(-10000.0), Vec3::splat(10000.0));
        prop_assert!(clamped.x() >= -10000.0 && clamped.x() <= 10000.0);
        prop_assert!(clamped.y() >= -10000.0 && clamped.y() <= 10000.0);
        prop_assert!(clamped.z() >= -10000.0 && clamped.z() <= 10000.0);
    }

    #[test]
    fn test_aspect_fit_preserves_ratio(src_w in 1.0..2000.0_f32,
                                    src_h in 1.0..2000.0_f32,
                                    dst_w in 1.0..2000.0_f32,
                                    dst_h in 1.0..2000.0_f32) {
        // Property: Aspect ratio preserved when fitting
        let (fit_w, fit_h) = compute_aspect_fit(src_w, src_h, dst_w, dst_h);
        let src_ratio = src_w / src_h;
        let dst_ratio = dst_w / dst_h;
        // If dst is wider, fit fills width
        // If dst is taller, fit fills height
        // Verify one dimension fills
        prop_assert!(fit_w.approx_eq(dst_w, 1.0) || fit_h.approx_eq(dst_h, 1.0));
    }

    #[test]
    fn test_matrix_determinant_rotation_unity(angle in 0.0..6.28_f32) {
        // Property: Rotation matrices have det = 1
        let matrix = Mat4::from_rotation_y(angle);
        let det = matrix.determinant();
        prop_assert!((det - 1.0).abs() < 0.01);
    }
}
```

**Coverage Target**: Mathematical functions across modules → 85%+

---

## ✅ Phase 2 Completion Checklist

**Weeks 3-4 Deliverables**:
- [ ] 3 new test files created
- [ ] 54+ new tests implemented
- [ ] Property-based testing infrastructure added
- [ ] MHA/MHD coverage: 60%+
- [ ] Volume integrity coverage: 70%+
- [ ] All tests passing

**Expected Coverage Improvement**:
- **Medical Imaging Formats**: 25% → **70%**
- **Volume Data**: 30% → **70%**
- **Overall Data Layer**: 20% → **65%**

---

## 🎨 Phase 3: Rendering Correctness (Weeks 5-6)

### Week 5: GPU Pipeline Tests

#### Task 3.1: GPU Initialization Tests (Days 1-2)
**File**: `tests/rendering_correctness_tests.rs` (NEW)  
**Effort**: 6 hours  
**Platform**: Native only (`#[cfg(not(target_arch = "wasm32"))]`)

**Deliverables** (8 tests):

```rust
#[cfg(not(target_arch = "wasm32"))]
mod gpu_pipeline_tests {
    use wgpu::*;

    #[test]
    async fn test_device_creation_failure() {
        // Test GPU creation with invalid options fails gracefully
    }

    #[test]
    async fn test_surface_format_detection() {
        // Verify surface format correctly detected
    }

    #[test]
    async fn test_pipeline_creation_success() {
        // Verify render pipeline creates successfully
    }

    #[test]
    async fn test_pipeline_recreation_on_format_change() {
        // Verify pipeline recreated when surface format changes
    }

    #[test]
    async fn test_shader_compilation_error_handling() {
        // Test invalid shader returns clear error
    }

    #[test]
    async fn test_bind_group_creation() {
        // Verify bind groups created with correct layouts
    }

    #[test]
    async fn test_texture_format_compatibility() {
        // Test all supported texture formats
    }

    #[test]
    async fn test_memory_cleanup_on_drop() {
        // Verify GPU resources freed correctly
    }
}
```

**Coverage Target**: `src/rendering/core/graphics.rs` from 0% → 50%  
**Coverage Target**: `src/rendering/core/pipeline.rs` from 5% → 40%

---

#### Task 3.2: Texture Management Tests (Days 3-5)
**File**: `tests/rendering_correctness_tests.rs` (APPEND)  
**Effort**: 8 hours

**Deliverables** (10 tests):

```rust
#[cfg(not(target_arch = "wasm32"))]
mod texture_tests {
    #[test]
    async fn test_volume_texture_creation() {
        // Test 3D texture from CTVolume succeeds
    }

    #[test]
    async fn test_texture_dimensions_match_volume() {
        // Verify texture dimensions equal volume dimensions
    }

    #[test]
    async fn test_texture_format_conversion() {
        // Test format conversion (i16 → R16Float/etc)
    }

    #[test]
    async fn test_texture_filter_modes() {
        // Test nearest/linear filtering
    }

    #[test]
    async fn test_texture_mipmaps_generation() {
        // Verify mipmaps generated correctly
    }

    #[test]
    async fn test_texture_upload_bounds() {
        // Test max texture size (GPU limits)
    }

    #[test]
    async fn test_texture_memory_limits() {
        // Verify texture allocation within memory limits
    }

    #[test]
    async fn test_texture_update_partial() {
        // Test updating portion of texture
    }

    #[test]
    async fn test_texture_copy_operations() {
        // Test texture-to-texture copies
    }

    #[test]
    async fn test_texture_destroy_cleanup() {
        // Verify texture GPU memory freed
    }
}
```

**Coverage Target**: `src/rendering/core/texture.rs` from 0% → 50%

---

### Week 6: View Management & Visual Tests

#### Task 3.3: View Manager Tests (Days 1-3)
**File**: `tests/rendering_correctness_tests.rs` (APPEND)  
**Effort**: 8 hours

**Deliverables** (12 tests):

```rust
mod view_management_tests {
    use crate::rendering::view::view_manager::ViewManager;

    #[test]
    fn test_view_manager_creation() {
        // Test ViewManager initializes correctly
    }

    #[test]
    fn test_view_manager_add_view() {
        // Test adding single view succeeds
    }

    #[test]
    fn test_view_manager_remove_view() {
        // Test removing view cleans up resources
    }

    #[test]
    fn test_view_manager_multiple_views() {
        // Test multiple views can be managed
    }

    #[test]
    fn test_view_manager_active_view() {
        // Test active view switching works
    }

    #[test]
    fn test_view_manager_view_count() {
        // Verify view count is accurate
    }

    #[test]
    fn test_view_manager_concurrent_operations() {
        // Test concurrent add/remove operations
    }

    #[test]
    fn test_view_manager_state_consistency() {
        // Verify state remains consistent after operations
    }

    #[test]
    fn test_view_manager_memory_cleanup() {
        // Test memory freed when views removed
    }

    #[test]
    fn test_view_manager_iterate_views() {
        // Test iteration over all views
    }

    #[test]
    fn test_view_manager_find_view_by_id() {
        // Test lookup view by ID works
    }
}
```

**Coverage Target**: `src/rendering/view/view_manager.rs` from ~30% → 70%

---

#### Task 3.4: Visual Correctness Tests (Days 4-5)
**File**: `tests/visual_correctness_tests.rs` (NEW)  
**Effort**: 6 hours

**Deliverables** (8 tests):

```rust
mod visual_correctness_tests {
    use crate::core::WindowLevel;

    #[test]
    fn test_window_level_clamping_center() {
        // Test window center clamped to valid range
    }

    #[test]
    fn test_window_level_clamping_width() {
        // Test window width clamped to positive
    }

    #[test]
    fn test_window_level_extreme_values() {
        // Test very large window/width values
    }

    #[test]
    fn test_window_level_preserves_contrast() {
        // Test contrast ratios maintained
    }

    #[test]
    fn test_aspect_fit_letterbox() {
        // Test tall source → letterbox
    }

    #[test]
    fn test_aspect_fit_pillarbox() {
        // Test wide source → pillarbox
    }

    #[test]
    fn test_aspect_fit_exact_match() {
        // Test matching dimensions → no padding
    }

    #[test]
    fn test_aspect_fit_square() {
        // Test square → square
    }
}
```

**Coverage Target**: Visual rendering functions → 60%+

---

## ✅ Phase 3 Completion Checklist

**Weeks 5-6 Deliverables**:
- [ ] 2 new test files created
- [ ] 48+ new tests implemented
- [ ] GPU initialization tests passing (native)
- [ ] Texture management tests passing (native)
- [ ] View manager tests passing
- [ ] Visual correctness tests passing

**Expected Coverage Improvement**:
- **Rendering Core**: 5% → **45%**
- **GPU Pipeline**: 0% → **40%**
- **Overall Rendering**: 21% → **50%**

---

## 🛡️ Phase 4: Error Handling & Robustness (Weeks 7-8)

### Week 7: Error Propagation & Edge Cases

#### Task 4.1: Error Propagation Tests (Days 1-3)
**File**: `tests/error_propagation_tests.rs` (NEW)  
**Effort**: 8 hours

**Deliverables** (12 tests):

```rust
mod error_propagation_tests {
    use std::io;

    #[test]
    fn test_file_not_found_error_propagates() {
        // Verify File I/O errors bubble up correctly
    }

    #[test]
    fn test_permission_denied_error_propagates() {
        // Verify permission errors clear message
    }

    #[test]
    fn test_parse_error_context_preserved() {
        // Verify error context includes file/line
    }

    #[test]
    fn test_error_chain_depth() {
        // Verify error chains maintained
    }

    #[test]
    fn test_error_user_friendly_message() {
        // Verify errors have clear, actionable messages
    }

    #[test]
    fn test_dicom_parse_error_includes_tag() {
        // Verify DICOM errors include offending tag
    }

    #[test]
    fn test_volume_creation_error_includes_dimensions() {
        // Verify volume errors include dimensions
    }

    #[test]
    fn test_gpu_allocation_error_message() {
        // Verify OOM errors clear
    }

    #[test]
    fn test_surface_creation_error_message() {
        // Verify GPU surface failures descriptive
    }

    #[test]
    fn test_shader_compilation_error_includes_stage() {
        // Verify shader errors specify vertex/fragment
    }

    #[test]
    fn test_texture_upload_error_includes_size() {
        // Verify texture errors include dimensions
    }

    #[test]
    fn test_error_recovery_doesnt_leak() {
        // Verify resources cleaned up after errors
    }
}
```

**Coverage Target**: Error paths across modules → 70%+

---

#### Task 4.2: Edge Case Tests (Days 4-5)
**File**: `tests/edge_case_tests.rs` (NEW)  
**Effort**: 6 hours

**Deliverables** (10 tests):

```rust
mod edge_case_tests {
    #[test]
    fn test_single_slice_volume() {
        // Test 1x512x512 volume (single slice)
    }

    #[test]
    fn test_two_pixel_volume() {
        // Test 1x2x2 volume (minimal)
    }

    #[test]
    fn test_maximum_dimensions_volume() {
        // Test 2048x2048x2048 volume
    }

    #[test]
    fn test_empty_dicom_series() {
        // Test series with 0 images
    }

    #[test]
    fn test_empty_patient() {
        // Test patient with 0 studies
    }

    #[test]
    fn test_mixed_pixel_types_in_series() {
        // Test handling of inconsistent pixel types (error)
    }

    #[test]
    fn test_inconsistent_spacing_in_series() {
        // Test series with varying spacing
    }

    #[test]
    fn test_corrupted_pixel_data_recovery() {
        // Test graceful handling of corrupted data
    }

    #[test]
    fn test_invalid_uid_format() {
        // Test UID not following DICOM standard
    }

    #[test]
    fn test_unicode_patient_name() {
        // Test Unicode characters in patient name
    }
}
```

**Coverage Target**: Edge case handlers → 60%+

---

### Week 8: Memory & Concurrency

#### Task 4.3: Memory Management Tests (Days 1-3)
**File**: `tests/robustness_tests.rs` (NEW)  
**Effort**: 8 hours

**Deliverables** (10 tests):

```rust
mod memory_tests {
    #[test]
    fn test_large_volume_memory_allocation() {
        // Test 2048^3 volume allocation (~8GB)
        // Should fail gracefully with clear error
    }

    #[test]
    fn test_memory_leak_repeated_load_unload() {
        // Load/unload volume 100 times
        // Verify memory stable
    }

    #[test]
    fn test_texture_memory_cleanup() {
        // Create/destroy textures repeatedly
        // Verify GPU memory freed
    }

    #[test]
    fn test_view_manager_memory_cleanup() {
        // Add/remove views repeatedly
        // Verify no leaks
    }

    #[test]
    fn test_dicom_repo_memory_growth() {
        // Load 1000 DICOM files
        // Verify memory bounded
    }

    #[test]
    fn test_concurrent_volume_parsing() {
        // Parse 10 volumes concurrently
        // Verify no race conditions
    }

    #[test]
    fn test_concurrent_view_updates() {
        // Update view state from multiple threads
        // Verify thread-safe
    }

    #[test]
    fn test_gpu_buffer_reuse() {
        // Verify buffers reused when possible
    }

    #[test]
    fn test_buffer_pool_growth() {
        // Verify buffer pool grows appropriately
    }

    #[test]
    fn test_out_of_memory_graceful_degradation() {
        // Simulate OOM, verify degradation
    }
}
```

---

## ✅ Phase 4 Completion Checklist

**Weeks 7-8 Deliverables**:
- [ ] 3 new test files created
- [ ] 42+ new tests implemented
- [ ] Error propagation coverage: 70%+
- [ ] Edge case coverage: 60%+
- [ ] Memory management tests passing

**Expected Coverage Improvement**:
- **Error Paths**: ~40% → **70%**
- **Edge Cases**: ~30% → **60%**
- **Overall**: ~35% → **55%**

---

## 🚀 Phase 5: Performance & Regression (Weeks 9+)

### Ongoing: Performance Benchmarks

#### Task 5.1: Performance Benchmarks
**File**: `tests/performance_benchmarks.rs` (NEW)  
**Effort**: 4 hours (initial setup)  
**Maintenance**: Update benchmarks quarterly

**Deliverables** (6 benchmarks):

```rust
mod performance_benchmarks {
    use std::time::Instant;

    #[test]
    fn benchmark_dicom_parsing_512x512() {
        let start = Instant::now();
        // Parse 512x512 DICOM
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 10, "Too slow: {:?}", elapsed);
    }

    #[test]
    fn benchmark_volume_creation_512x512x100() {
        let start = Instant::now();
        // Create CTVolume from parsed data
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 50, "Too slow: {:?}", elapsed);
    }

    #[test]
    fn benchmark_volume_rendering_512x512() {
        let start = Instant::now();
        // Render single frame
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 16, "Below 60 FPS: {:?}", elapsed);
    }

    #[test]
    fn benchmark_mpr_slice_extraction() {
        let start = Instant::now();
        // Extract MPR slice
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 1, "Too slow: {:?}", elapsed);
    }

    #[test]
    fn benchmark_mesh_generation() {
        let start = Instant::now();
        // Generate mesh from volume
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 100, "Too slow: {:?}", elapsed);
    }

    #[test]
    fn test_memory_usage_large_volume() {
        // Measure memory usage
        // Verify within limits (< 2GB for 512^3 volume)
    }
}
```

---

#### Task 5.2: Regression Test Suite
**File**: `tests/regression_tests.rs` (NEW)  
**Effort**: 2 hours per regression  
**Trigger**: Add test for each bug fix

**Template**:
```rust
mod regression_tests {
    /// Regression test for GitHub Issue #001
    /// Bug: Incorrect pixel rescaling when rescale_slope = 0
    /// Fix: Added check for zero rescale_slope in CTImage::get_pixel_data()
    #[test]
    fn regression_issue_001_pixel_rescaling_zero_slope() {
        // Setup DICOM with rescale_slope = 0
        // Verify error returned or handled correctly
    }

    /// Regression test for GitHub Issue #002
    /// Bug: Sagittal matrix had wrong orientation
    /// Fix: Corrected sagittal orientation matrix
    #[test]
    fn regression_issue_002_sagittal_matrix_orientation() {
        // Verify sagittal matrix is correct
        let sagittal = GeometryBuilder::sagittal_base();
        assert!(sagittal.matrix.determinant().abs() - 1.0 < 0.01);
    }

    /// ADD NEW REGRESSION TESTS HERE FOR EACH BUG FIX
}
```

---

## 📊 Overall Progress Tracking

### Test Count Targets

| Phase | New Tests | Test Files | Test Lines Est. |
|--------|-----------|-------------|-----------------|
| **Phase 1** | 40 | 3 | ~600 |
| **Phase 2** | 54 | 3 | ~900 |
| **Phase 3** | 48 | 2 | ~800 |
| **Phase 4** | 42 | 3 | ~700 |
| **Phase 5** | 6+ (ongoing) | 2 | ~200 |
| **TOTAL** | **190+** | **13** | **~3200** |

### Coverage Targets

| Module | Current | Phase 1 | Phase 2 | Phase 3 | Phase 4 | Phase 5 |
|--------|----------|----------|----------|----------|----------|----------|
| **DICOM Data** | 20% | **60%** | - | - | - | **75%** |
| **Patient Metadata** | 16% | **70%** | - | - | - | **80%** |
| **Medical Formats** | 25% | - | **70%** | - | - | **85%** |
| **Volume Data** | 30% | - | **70%** | - | - | **80%** |
| **Coordinates/MPR** | 8% | **50%** | - | - | - | **70%** |
| **Rendering Core** | 5% | - | - | **45%** | - | **60%** |
| **GPU Pipeline** | 0% | - | - | **40%** | - | **55%** |
| **Error Handling** | ~40% | - | - | - | **70%** | **80%** |
| **OVERALL** | **24%** | **45%** | **60%** | **55%** | **70%** |

---

## 🔧 Implementation Guide

### Getting Started

**Step 1: Setup Test Infrastructure**
```bash
# Create common test utilities
mkdir -p tests/common
touch tests/common/mod.rs

# Create first test file
touch tests/dicom_validation_critical_tests.rs
```

**Step 2: Update Cargo.toml**
```toml
[dev-dependencies]
proptest = "1.4"
quickcheck = "1.0"
```

**Step 3: Run Tests**
```bash
# Run specific test file
cargo test --test dicom_validation_critical_tests

# Run all tests
cargo test

# Generate coverage report
cargo llvm-cov --html --output-dir coverage
```

### Test Naming Convention

```rust
// Format: test_<module>_<behavior>_<scenario>

#[test]
fn test_dicom_from_bytes_missing_uid_rejected() {
    // Module: dicom
    // Behavior: from_bytes
    // Scenario: missing uid
    // Expected: rejected
}

#[test]
fn test_mpr_slice_position_negative_clamped() {
    // Module: mpr
    // Behavior: slice position
    // Scenario: negative
    // Expected: clamped
}
```

### Test Structure Template

```rust
#[cfg(test)]
mod some_module_tests {
    use super::*;

    #[test]
    fn test_something() {
        // ARRANGE
        let input = create_test_input();
        
        // ACT
        let result = function_under_test(input);
        
        // ASSERT
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value, expected);
    }

    #[test]
    #[should_panic(expected = "specific message")]
    fn test_invalid_input_panics() {
        // Test that invalid input panics with expected message
    }

    #[test]
    fn test_edge_case_boundary_value() {
        // Test boundary conditions
        let result = function(100); // MAX_VALUE
        assert_eq!(result, expected_boundary_result);
    }
}
```

---

## 📋 Weekly Checklist Template

Use this to track progress each week:

```
Week X: [Phase Name]
─────────────────────

Tasks Completed:
- [ ] Task X.Y: [description] (est. X hours)
  - Actual: X hours
  - Tests added: N
  - Coverage improved: A% → B%

Issues Encountered:
- [ ] Issue 1: [description]
  - Resolution: [how fixed]

Coverage Report:
- Overall: XX% (was XX%)
- [Module]: XX% (was XX%)

Tests Summary:
- Total tests: N
- Tests added: N
- Tests failing: N
- Tests skipped: N

Next Week Focus:
- [ ] Priority tasks for next week
```

---

## 🎯 Success Criteria

### Phase 1 Complete (Week 2)
- [ ] 40+ new tests added
- [ ] All DICOM mandatory fields validated
- [ ] Patient identity covered (70%+)
- [ ] Coordinate transformations covered (50%+)
- [ ] Overall coverage: 45%+
- [ ] Zero critical gaps in medical paths

### Phase 2 Complete (Week 4)
- [ ] 54+ new tests added
- [ ] MHA/MHD formats covered (60%+)
- [ ] Volume integrity covered (70%+)
- [ ] Property-based testing working
- [ ] Overall coverage: 60%+

### Phase 3 Complete (Week 6)
- [ ] 48+ new tests added
- [ ] GPU initialization covered (40%+)
- [ ] Rendering covered (45%+)
- [ ] Overall coverage: 55%+

### Phase 4 Complete (Week 8)
- [ ] 42+ new tests added
- [ ] Error paths covered (70%+)
- [ ] Edge cases covered (60%+)
- [ ] Memory tests passing
- [ ] Overall coverage: 70%+

### Phase 5 Ongoing (Week 9+)
- [ ] Performance benchmarks in place
- [ ] Regression tests for all bug fixes
- [ ] CI coverage gates active
- [ ] Overall coverage maintained: 80%+ on critical paths

---

## 📞 Support & Resources

### Documentation
- **Test Strategy**: `doc/agents/test_strategy_comprehensive.md`
- **Architecture**: `doc/agents/ARCHITECTURE.md`
- **Conventions**: `doc/agents/CONVENTIONS.md`
- **Common Pitfalls**: `doc/agents/PITFALLS.md`

### Tools
- **Coverage**: `cargo llvm-cov --html`
- **Property testing**: `proptest` crate
- **Fuzz testing**: `cargo fuzz`
- **Benchmarking**: `criterion` crate (optional)

### When Stuck
1. Check existing test patterns in `tests/`
2. Review similar tests in open-source medical imaging projects
3. Consult test strategy document for guidance
4. File GitHub issue with tag `testing-help`

---

**Last Updated**: 2026-01-18  
**Next Review**: After Phase 1 completion (Week 2)
**Maintained By**: Development Team
