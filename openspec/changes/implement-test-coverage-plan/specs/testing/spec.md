## MODIFIED Requirements

### Requirement: Test Coverage Targets

The system SHALL maintain minimum coverage percentages across different modules to support risk reduction and code quality. Percentages alone SHALL NOT be treated as proof of patient safety.

#### Scenario: Medical Path Coverage Target
- **WHEN** measuring code coverage for critical medical paths
- **THEN** coverage SHALL be ≥ 80% for DICOM parsing, patient metadata, and coordinate transformations
- **AND** tests SHALL explicitly validate these paths
- **AND** CI SHALL fail if coverage drops below 80%
- **AND** the reported metric, included files, denominator, toolchain, and tool version SHALL be recorded according to `doc/testing/coverage.md`

#### Scenario: Overall Coverage Targets by Phase
- **WHEN** Phase 1 is complete (Week 2)
- **THEN** overall coverage SHALL be ≥ 45%
- **WHEN** Phase 2 is complete (Week 4)
- **THEN** overall coverage SHALL be ≥ 60%
- **WHEN** Phase 3 is complete (Week 6)
- **THEN** overall coverage SHALL be ≥ 65%
- **WHEN** Phase 4 is complete (Week 8)
- **THEN** overall coverage SHALL be ≥ 70%
- **AND** progress SHALL be tracked via reproducible coverage reports rather than inferred from test counts

### Requirement: CI/CD Coverage Gating

CI/CD pipeline SHALL enforce coverage targets and prevent regressions.

#### Scenario: Coverage Gate Failure
- **WHEN** a pull request is submitted and critical medical path coverage < 80%
- **THEN** the CI pipeline SHALL fail
- **AND** the failure SHALL indicate which scope is below threshold
- **AND** the PR SHALL NOT be mergeable until coverage improves
- **AND** missing or invalid measurement data SHALL fail rather than be treated as zero or successful execution

#### Scenario: Coverage Trend Detection
- **WHEN** comparable coverage is measured on successive builds
- **THEN** significant decreases (> 5% absolute) SHALL trigger warnings
- **AND** the CI SHALL report which files/lines lost coverage
- **AND** this SHALL enable early detection of coverage regressions
- **AND** missing or differently scoped baseline evidence SHALL be reported explicitly rather than silently compared
