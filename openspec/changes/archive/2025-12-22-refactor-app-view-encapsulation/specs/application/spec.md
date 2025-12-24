## ADDED Requirements

### Requirement: View State Encapsulation
The `AppView` SHALL be responsible for serializing and deserializing transient view state (such as orientation, scale, pan, and slice position) to support layout transitions without data loss.

#### Scenario: Layout Reconfiguration
- **WHEN** the layout changes (e.g., from grid to single view and back)
- **THEN** `AppView` captures the state of active views and restores it to the new views where applicable, without the `App` controller needing to iterate or inspect view types.

### Requirement: Centralized View Interaction
The `AppView` SHALL provide type-safe interfaces for modifying view properties, abstracting away the specific view implementation details (downcasting, error handling) from the `App` controller.

#### Scenario: Updating Window Level
- **WHEN** the `App` receives a request to update window level for a specific view index
- **THEN** it calls a single method on `AppView` (e.g., `set_window_level`), which handles the lookup, type check, and error logging internally.
