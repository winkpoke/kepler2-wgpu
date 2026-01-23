## ADDED Requirements
### Requirement: MIP Rotation Control Interface
The application layer SHALL expose a type-safe interface to update MIP rotation for a specific view index without requiring callers to downcast view types.

#### Scenario: Set MIP Rotation From UI
- **WHEN** the UI issues a “set MIP rotation” request for a view index
- **THEN** the application routes the request through a single AppView method
- **AND** only MIP views apply the rotation update while other view types are unaffected
