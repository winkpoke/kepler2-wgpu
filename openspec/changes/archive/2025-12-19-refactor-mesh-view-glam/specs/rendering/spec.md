## ADDED Requirements
### Requirement: Optimized Math Operations
The rendering subsystem SHALL use the `glam` library for all vector and matrix operations to ensure performance and maintainability.

#### Scenario: MVP Calculation
- **WHEN** calculating Model-View-Projection matrices in `MeshView`
- **THEN** `glam::Mat4` operations MUST be used instead of manual array manipulation
- **AND** the resulting matrices MUST be compatible with the shader's column-major layout.
