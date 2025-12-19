## ADDED Requirements
### Requirement: MIP View Geometry
The MIP view implementation MUST use `glam` vector types for all internal geometry state and transformations to ensure type safety and performance.

#### Scenario: Pan Operation
- **WHEN** the user pans the MIP view
- **THEN** the pan offset is stored as a `glam::Vec3`
- **AND** vector operations are used for clamping and updates
