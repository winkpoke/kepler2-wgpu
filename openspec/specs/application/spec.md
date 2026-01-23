## ADDED Requirements

### Requirement: Application Testing
The application layer SHALL have comprehensive test coverage for state management, view orchestration, and user interactions to prevent regressions.

#### Scenario: Test Fixture Usage
- **WHEN** tests are written that require valid application state or configuration
- **THEN** reusable test fixtures SHALL be available via `tests/common/test_fixtures.rs`
- **AND** fixtures SHALL generate valid `AppModel`, `AppView`, and view configurations
- **AND** fixtures SHALL provide malformed data for validation tests

#### Scenario: Application State Transitions
- **WHEN** user actions modify application state (e.g., load volume, toggle mesh mode, change view layout)
- **THEN** state transitions SHALL be tested for correctness
- **AND** state SHALL remain consistent across concurrent operations
- **AND** view state SHALL be preserved during layout changes where applicable
- **AND** no data loss SHALL occur

#### Scenario: View Orchestration Testing
- **WHEN** multiple views are active (MPR, MIP, mesh)
- **THEN** view manager SHALL handle concurrent add/remove operations correctly
- **AND** view manager SHALL handle active view switching without state loss
- **AND** each view SHALL be independently testable in isolation
- **AND** memory SHALL be cleaned up when views are removed

### Requirement: MIP Rotation Control Interface
The application layer SHALL expose a type-safe interface to update MIP rotation for a specific view index without requiring callers to downcast view types.

#### Scenario: Set MIP Rotation From UI
- **WHEN** the UI issues a “set MIP rotation” request for a view index
- **THEN** the application routes the request through a single AppView method
- **AND** only MIP views apply the rotation update while other view types are unaffected

#### Scenario: Error Recovery in Application Layer
- **WHEN** application-level errors occur (e.g., invalid file load, state corruption)
- **THEN** errors SHALL be handled gracefully without crashing
- **AND** user SHALL receive clear, actionable error messages
- **AND** system SHALL maintain valid state despite errors
