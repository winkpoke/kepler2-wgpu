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

