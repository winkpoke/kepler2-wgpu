# Change: Implement Test Coverage Improvement Plan

## Why

Current test coverage is 24% across the codebase, which is **insufficient for medical imaging software** where patient safety is critical. The test strategy analysis revealed:
- 5 files with 0% coverage (application layer, GPU initialization)
- Critical medical imaging paths (DICOM parsing, patient metadata) at 16-20%
- No comprehensive test infrastructure (fixtures, property-based testing)
- Missing validation for mandatory DICOM fields and coordinate transformations

Improving test coverage is necessary to ensure:
- Patient safety through validated DICOM parsing and metadata handling
- Correct medical imaging through tested coordinate transformations and volume reconstruction
- Robustness through error handling and edge case coverage
- Maintainability through regression test suite

## What Changes

Implement comprehensive test coverage across 5 phases over 12 weeks:

**Phase 1: Medical Safety Critical (Weeks 1-2)**
- Create test fixtures infrastructure
- DICOM validation tests (mandatory field rejection, rescaling edge cases)
- Patient identity tests (UID validation, study/series integrity)
- Coordinate transformation safety tests (slice position bounds, matrix orthogonality)

**Phase 2: Data Integrity (Weeks 3-4)**
- MHA/MHD format parsing tests (endianness, compression, header validation)
- Volume integrity tests (dimension validation, voxel spacing checks)
- Property-based testing for mathematical correctness (window/level, clamping, aspect ratios)

**Phase 3: Rendering Correctness (Weeks 5-6)**
- GPU pipeline initialization tests (native-only)
- Texture management tests (format conversion, memory limits)
- View management tests (state consistency, concurrent operations)
- Visual correctness tests (window/level clamping, aspect fit)

**Phase 4: Error Handling & Robustness (Weeks 7-8)**
- Error propagation tests (file I/O, parse errors, GPU errors)
- Edge case tests (single-slice volumes, maximum dimensions, empty data)
- Memory management tests (leak detection, OOM handling, concurrent operations)

**Phase 5: Performance & Regression (Weeks 9+ - Ongoing)**
- Performance benchmarks (DICOM parsing, volume rendering, frame timing)
- Regression test suite (one test per bug fix)
- CI/CD coverage gates

## Impact

- Affected specs:
  - `application` - Add testing requirements for app state management and UI orchestration
  - `core` - Add testing requirements for geometry, coordinate systems, math utilities
  - `rendering` - Add testing requirements for GPU pipelines, views, textures
  - **NEW**: `testing` - Add comprehensive testing capability (fixtures, property-based testing, benchmarks)

- Affected code:
  - `src/data/dicom/` - DICOM parsing, patient metadata, series/Study management
  - `src/data/medical_imaging/` - MHA/MHD format parsers, volume data
  - `src/rendering/core/` - GPU initialization, pipeline management, textures
  - `src/rendering/view/` - MPR, MIP, mesh views, view manager
  - `src/core/` - Geometry, coordinate systems, window/level
  - `tests/` - New test files: dicom_validation_critical_tests.rs, patient_safety_tests.rs, coordinate_safety_tests.rs, file_format_integrity_tests.rs, volume_integrity_tests.rs, property_tests.rs, rendering_correctness_tests.rs, visual_correctness_tests.rs, error_propagation_tests.rs, edge_case_tests.rs, robustness_tests.rs, performance_benchmarks.rs, regression_tests.rs
  - `tests/common/` - Test fixtures infrastructure (test_fixtures.rs)

- New infrastructure:
  - Test fixtures module with reusable DICOM, patient, volume data generators
  - Property-based testing with `proptest` crate
  - Performance benchmarking infrastructure
  - CI/CD coverage reporting and gating
  - Regression test tracking system

**Coverage Targets**:
- Phase 1 (Week 2): 45% overall (from 24%), 80%+ on critical medical paths
- Phase 2 (Week 4): 60% overall, 65% on data layer
- Phase 3 (Week 6): 55% overall, 50% on rendering
- Phase 4 (Week 8): 70% overall, 70% on error paths
- Phase 5 (Ongoing): Maintain 80%+ on critical paths, full regression coverage
