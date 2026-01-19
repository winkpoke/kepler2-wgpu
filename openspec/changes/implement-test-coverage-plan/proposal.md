# Change: Implement Test Coverage Improvement Plan

## Why

Current test coverage is 24% across the codebase, which is **insufficient for medical imaging software** where patient safety is critical. The test strategy analysis revealed:
- 5 files with 0% coverage (application layer, GPU initialization)
- Critical medical imaging paths (DICOM parsing, patient metadata) at 16-20%
- No comprehensive test infrastructure (fixtures, property-based testing, WASM testing)
- Missing validation for mandatory DICOM fields, coordinate transformations, GPU safety, and memory management

**Critical Issues Identified** (27 total):
- Property testing strategy undefined (no specific property definitions)
- WASM testing strategy absent (browser rendering paths unaddressed)
- Test fixture architecture undefined (duplication risk)
- Memory leak detection strategy absent (no implementation method)
- Performance targets unrealistic (hard limits will fail in CI)
- GPU coverage targets unachievable (native-only code)
- DICOM validation tests missing (tag VR, transfer syntax, pixel data edge cases)
- Coordinate precision tests assume clamping that doesn't exist
- Regression test naming convention ambiguous
- Coverage calculation methodology undefined
- CI/CD coverage gates undefined

Improving test coverage is necessary to ensure:
- Patient safety through validated DICOM parsing and metadata handling
- Correct medical imaging through tested coordinate transformations and volume reconstruction
- Robustness through error handling and edge case coverage
- Maintainability through regression test suite

## What Changes

Implement comprehensive test coverage across 5 phases over 12 weeks:

**Phase 1: Medical Safety Critical (Weeks 1-2)**
- Create test fixtures infrastructure (centralized at `tests/common/mod.rs`, see `doc/agents/test_fixture_architecture.md`)
- DICOM validation tests (mandatory field rejection, VR type validation, transfer syntax validation, rescaling edge cases)
- Patient identity tests (UID validation, format validation, study/series integrity, patient metadata validation)
- Coordinate transformation safety tests (slice position validation, roundtrip precision, matrix orthogonality)

**Phase 2: Data Integrity (Weeks 3-4)**
- MHA/MHD format parsing tests (endianness, compression, header validation, external file resolution)
- Volume integrity tests (dimension validation, voxel spacing checks, anisotropy detection)
- Property-based testing for mathematical correctness (window/level, coordinate transformations, see `doc/agents/property_testing_strategy.md`)
- DICOM pixel data edge case tests (odd sizes, overflow, precision loss, negative slope)

**Phase 3: Rendering Correctness (Weeks 5-6)**
- GPU pipeline initialization tests (native-only, offline testing approach, see `doc/agents/gpu_testing_strategy.md`)
- Texture management tests (format conversion, memory limits, upload bounds, mipmap validation)
- View management tests (state consistency, concurrent operations, race conditions)
- Visual correctness tests (window/level, aspect fit, shader compilation error handling)
- WASM testing unit tests (browser-specific code paths, wasm-bindgen bridge, see `doc/agents/wasm_testing_strategy.md`)

**Phase 4: Error Handling & Robustness (Weeks 7-8)**
- Error propagation tests (file I/O, parse errors, GPU errors with context)
- Edge case tests (single-slice volumes, maximum dimensions, empty data)
- Memory management tests (valgrind leak detection in CI, memory cleanup, see `doc/agents/memory_testing_strategy.md`)
- Regression test infrastructure (naming convention: `regression_issue_NNN_symptom`, see `doc/agents/regression_testing_guide.md`)

**Phase 5: Performance & Regression (Weeks 9+ - Ongoing)**
- Performance benchmarks (DICOM parsing, volume rendering, frame timing) with statistical thresholds (mean ± 3 std dev, hardware variability allowance)
- Regression test suite (one test per bug fix, naming: `regression_issue_NNN_symptom`)
- CI/CD coverage reporting and gating (cargo-llvm-cov, branch coverage, medical path ≥ 80%, see `doc/agents/coverage_methodology.md`)
- WASM integration tests (Playwright browser automation, cross-browser compatibility)

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
  - Test fixtures module with reusable DICOM, patient, volume data generators (see `doc/agents/test_fixture_architecture.md`)
  - Property-based testing with `proptest` crate and defined property invariants (see `doc/agents/property_testing_strategy.md`)
  - WASM testing strategy with unit and browser integration tests (see `doc/agents/wasm_testing_strategy.md`)
  - Memory leak detection with Valgrind integration in CI (see `doc/agents/memory_testing_strategy.md`)
  - Performance benchmarking with statistical thresholds and hardware variability (see tasks.md task 6.1)
  - Coverage calculation methodology with branch coverage and medical path definition (see `doc/agents/coverage_methodology.md`)
  - GPU testing strategy with offline testing and adjusted targets (see `doc/agents/gpu_testing_strategy.md`)
  - Regression test tracking system with naming convention `regression_issue_NNN_symptom` (see `doc/agents/regression_testing_guide.md`)
  - CI/CD coverage reporting and gating with trend analysis

**Coverage Targets**:
- Phase 1 (Week 2): 45% overall (from 24%), 80%+ on critical medical paths, 37+ tests
- Phase 2 (Week 4): 60% overall, 65% on data layer, 36+ tests
- Phase 3 (Week 6): 65% overall, 50% on rendering (adjusted for GPU testability), 36+ tests
- Phase 4 (Week 8): 70% overall, 70% on error paths, 29+ tests
- Phase 5 (Ongoing): Maintain 80%+ on critical paths, full regression coverage, CI gates active
