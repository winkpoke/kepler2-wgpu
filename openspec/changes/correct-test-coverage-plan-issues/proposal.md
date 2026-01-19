# Change: Correct Test Coverage Plan Issues

## Why

The test coverage implementation plan (`implement-test-coverage-plan`) contains **27 critical bugs and design flaws** identified through comprehensive codebase analysis:

### Critical Implementation Gaps
- MHA/MHD format tests assume functionality that doesn't exist
- GPU test coverage targets are unachievable (native-only code)
- Property testing strategy is generic without specific property definitions

### Logical Inconsistencies
- Duplicate "Medical Path Coverage Enforcement" requirement
- Phase 3 coverage target (55%) is lower than Phase 2 (60%)—impossible
- Test counts in success criteria don't match task breakdown
- Coverage targets inconsistent across proposal and specs

### Technical Flaws
- WASM testing completely unaddressed despite `#[cfg(target_arch = "wasm32")]` guards
- Test fixture architecture undefined (duplication will explode)
- Coordinate transformation tests assume clamping that doesn't exist
- Memory leak testing strategy absent (no detection method)
- Performance benchmark targets are unrealistic (hard timing limits will fail in CI)
- Regression test naming convention is ambiguous

### Missing Requirements
- DICOM tag validation (VR types, transfer syntax)
- Pixel data edge cases (overflow, truncation, precision loss)
- Coordinate system roundtrip precision tests
- Matrix orthogonality tests
- UID format validation
- Study/series relationship validation
- Voxel spacing validation
- Texture upload bounds tests
- Concurrent view state tests
- Shader compilation error tests
- Coverage calculation methodology undefined
- CI/CD coverage gates undefined

**Medical Safety Risk**: These gaps could allow malformed DICOM data, corrupted pixel data, or incorrect coordinate transformations to reach production, violating patient safety requirements.

## What Changes

Fix all identified issues in test coverage plan to make it **implementable, measurable, and medically safe**:

### Immediate Fixes (Blocking Implementation)
- **FIX**: Remove duplicate "Medical Path Coverage Enforcement" requirement in `specs/testing/spec.md`
- **FIX**: Correct Phase 3 coverage target from 55% to 65% (must increase from Phase 2's 60%)
- **FIX**: Define property-based testing strategy with specific properties (monotonicity, invertibility, bounds, orthogonality)
- **ADD**: WASM testing strategy for browser rendering paths
- **ADD**: Test fixture architecture definition (location: `tests/common/mod.rs`, export policy, versioning strategy)
- **ADD**: Memory leak detection strategy (valgrind in CI, custom allocator tracking, or remove from plan)
- **ADD**: Realistic performance targets with ranges + statistical thresholds instead of hard time limits
- **FIX**: Align test counts in success criteria with actual task breakdown
- **ADD**: Regression test naming convention: `regression_issue_NNN_description` or `regression_module_symptom`

### Medium-Term Fixes (During Implementation)
- **ADD**: DICOM tag validation tests (VR types, transfer syntax, private tags)
- **ADD**: Pixel data edge case tests (odd sizes, overflow in rescaling, precision loss)
- **ADD**: Coordinate roundtrip precision tests with error bounds (< 0.001 mm tolerance)
- **ADD**: Matrix orthogonality tests (dot products, determinants, unit vectors)
- **ADD**: UID format validation (DICOM 2.25 + ISO OID format, max 64 chars, 0-9 and period only)
- **ADD**: Study/series relationship integrity tests
- **ADD**: Voxel spacing validation tests (realistic bounds, anisotropy detection)
- **ADD**: Texture upload bounds tests (max GPU texture size, format mismatches, partial updates)
- **ADD**: Concurrent view state tests (race conditions, dirty flag updates)
- **ADD**: Shader compilation error tests (syntax errors, resource limit exceeded, missing uniforms)
- **ADD**: Coverage calculation methodology (line vs branch coverage, medical path definition, exclusion policy)

### Long-Term Fixes (Architectural)
- **ADD**: GPU testing strategy (mock implementations, offline testing approach)
- **ADD**: WASM integration test suite (Puppeteer/Playwright for browser rendering)
- **ADD**: Medical test dataset library (not just synthetic data)
- **ADD**: CI/CD coverage gates definition (tool selection, upload location, threshold values)

## Impact

- Affected specs:
  - `testing` - Fix duplicate requirement, add missing test strategies, define coverage methodology
  - `application` - Add WASM testing requirements for browser paths
  - `core` - Add property testing definitions, coordinate precision requirements
  - `rendering` - Add GPU testing strategy, shader validation requirements

- Affected code:
  - `openspec/changes/implement-test-coverage-plan/` - Correct all documents with identified flaws
  - `tests/common/mod.rs` - Create test fixture module with architecture
  - `tests/` - Add missing test coverage for DICOM validation, coordinate precision, GPU safety
  - `.github/workflows/` - Add CI/CD configuration for coverage reporting and gating

- New infrastructure:
  - Test fixture architecture (centralized, versioned, re-exportable)
  - Property testing strategy document (specific properties, input ranges, expected invariants)
  - Coverage calculation methodology (tool config, metrics definition, thresholds)
  - Memory leak detection strategy (valgrind integration, custom allocator, or removal)
  - CI/CD coverage pipeline (cargo-llvm-cov, upload to codecov, threshold enforcement)

**Blocking Issues**: All 27 issues must be resolved before implementation to prevent:
- Test implementation failures (unreachable coverage targets)
- Test flakiness (unrealistic performance targets)
- Medical safety violations (missing DICOM validation)
- Maintainability problems (fixture duplication)
