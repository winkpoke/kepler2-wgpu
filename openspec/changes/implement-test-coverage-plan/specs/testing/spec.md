## MODIFIED Requirements

### Requirement: Application Testing
See `openspec/specs/application/spec.md` - Added comprehensive testing requirements for application state management, view orchestration, and error recovery.

#### Scenario: Medical Path Testing in Application Layer
- **WHEN** application-level tests cover patient-critical functionality (patient ID validation, study/series integrity, UID uniqueness)
- **THEN** application code SHALL ensure patient safety through proper validation
- **AND** errors SHALL be caught early before rendering or display
- **AND** tests SHALL validate that patient data matches DICOM metadata

### Requirement: Core Testing
See `openspec/specs/core/spec.md` - Added comprehensive testing requirements for mathematical operations, coordinate systems, geometry, and window/level transformations.

#### Scenario: Medical Math Testing in Core Module
- **WHEN** core math functions are used for patient-critical calculations (coordinate transformations, rescaling)
- **THEN** operations SHALL be tested with property-based tests
- **AND** numerical precision SHALL be verified across all Hounsfield unit ranges
- **AND** edge cases SHALL be covered (zero slope, overflow, NaN)
- **AND** tests SHALL use proptest for comprehensive input space exploration

#### Scenario: Coordinate System Safety Testing
- **WHEN** coordinate transformations are used for medical display
- **THEN** correctness SHALL be verified through roundtrip tests
- **AND** bounds checking SHALL prevent out-of-range slice positions
- **AND** orientation matrices SHALL be tested for orthogonality
- **AND** precision SHALL be preserved (error < 0.001 mm)

### Requirement: Rendering Testing
See `openspec/specs/rendering/spec.md` - Added comprehensive testing requirements for GPU pipeline creation, texture management, view manager state consistency, visual correctness, GPU error handling, and performance benchmarks.

#### Scenario: Medical Visualization Testing
- **WHEN** rendering tests cover medical imaging display (window/level, aspect ratios, coordinate accuracy)
- **THEN** visual fidelity SHALL be preserved across all transformations
- **AND** clamping SHALL prevent invalid Hounsfield display values
- **AND** aspect ratios SHALL be preserved (no anatomical distortion)
- **AND** extreme values SHALL be handled gracefully without crashes

#### Scenario: GPU Resource Management Testing
- **WHEN** GPU resources (textures, buffers, pipelines) are created or destroyed
- **THEN** operations SHALL be tested for proper memory management
- **AND** no memory leaks SHALL occur under normal operation
- **AND** resource cleanup SHALL be verified in error paths
- **AND** concurrent operations SHALL be thread-safe

### Requirement: Testing
See `openspec/specs/testing/spec.md` - Added new comprehensive testing capability defining test fixtures, DICOM validation, patient safety, coordinate transformations, format parsing, volume integrity, property-based testing, error handling, edge cases, performance benchmarks, and regression testing.

### Requirement: Test Coverage Targets
See `openspec/specs/testing/spec.md` - Added coverage targets for medical paths (80%+), overall coverage by phase, and CI/CD gating requirements.

#### Scenario: Medical Path Coverage Enforcement
- **WHEN** measuring code coverage for critical medical paths
- **THEN** coverage SHALL be ≥ 80% for patient safety
- **AND** CI pipeline SHALL fail if coverage drops below 80%
- **AND** affected modules SHALL include DICOM parsing, patient metadata, coordinate transformations

#### Scenario: Phase-Based Coverage Tracking
- **WHEN** completing each phase of test implementation (Weeks 1-5)
- **THEN** coverage SHALL meet defined targets (45%, 60%, 55%, 70%)
- **AND** test counts SHALL match implementation plan (40, 54, 48, 42 tests)
- **AND** all critical gaps SHALL be addressed before phase completion

#### Scenario: CI/CD Coverage Gating
- **WHEN** pull requests are submitted to codebase
- **THEN** coverage SHALL be measured and compared against thresholds
- **AND** PRs SHALL be blocked if coverage decreases significantly
- **AND** coverage trend analysis SHALL detect regressions early
- **AND** overall project coverage SHALL be tracked across all PRs

#### Scenario: Medical Path Coverage Enforcement
- **WHEN** measuring code coverage for critical medical paths
- **THEN** coverage SHALL be ≥ 80% for DICOM parsing, patient metadata, and coordinate transformations
- **AND** CI pipeline SHALL fail if medical path coverage drops below 80%

#### Scenario: Phase Coverage Targets
- **WHEN** Phase 1 is complete (Week 2)
- **THEN** overall coverage SHALL be ≥ 45% (from 24%)
- **AND** 40+ new tests SHALL be implemented
- **WHEN** Phase 2 is complete (Week 4)
- **THEN** overall coverage SHALL be ≥ 60%
- **AND** 54+ new tests SHALL be implemented
- **WHEN** Phase 3 is complete (Week 6)
- **THEN** overall coverage SHALL be ≥ 55%
- **AND** 48+ new tests SHALL be implemented
- **WHEN** Phase 4 is complete (Week 8)
- **THEN** overall coverage SHALL be ≥ 70%
- **AND** 42+ new tests SHALL be implemented
- **WHEN** Phase 5 is ongoing (Week 9+)
- **THEN** medical path coverage SHALL be maintained ≥ 80%
- **AND** regression test SHALL exist for each bug fix
