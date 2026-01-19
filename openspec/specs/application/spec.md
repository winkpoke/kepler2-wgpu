# application Specification

## Purpose
Defines the application layer architecture, ensuring strict separation of concerns between data state management (`AppModel`) and UI/rendering orchestration (`App`).
## Requirements
### Requirement: Application Model Responsibility
The `AppModel` SHALL be the single source of truth for application configuration state and data preparation logic.

#### Scenario: Data Preparation for Rendering
- **WHEN** the application needs to render a loaded volume
- **THEN** `AppModel` provides the raw byte buffer and format metadata (e.g., is_float) without requiring the caller to perform voxel arithmetic.

#### Scenario: State Management
- **WHEN** the user toggles mesh mode or changes texture precision settings
- **THEN** this state is updated in `AppModel`, not in the UI controller (`App`).

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

### Requirement: MIP Rotation Control Interface
The application layer SHALL expose a type-safe interface to update MIP rotation for a specific view index without requiring callers to downcast view types.

#### Scenario: Set MIP Rotation From UI
- **WHEN** the UI issues a “set MIP rotation” request for a view index
- **THEN** the application routes the request through a single AppView method
- **AND** only MIP views apply the rotation update while other view types are unaffected

