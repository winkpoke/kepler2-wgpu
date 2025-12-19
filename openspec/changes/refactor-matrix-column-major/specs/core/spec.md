## ADDED Requirements
### Requirement: Column-Major Matrix Layout
The `Matrix4x4` struct MUST store data in column-major order to align with WGPU and standard graphics libraries. The struct MUST use `#[repr(C)]` to guarantee memory layout.

#### Scenario: Direct Buffer Copy
- **WHEN** a `Matrix4x4` is cast to a byte slice (e.g., via `bytemuck`)
- **THEN** the memory layout MUST match the shader's column-major expectation (vectors are columns) without additional transposition.

### Requirement: Explicit Matrix Construction
Matrix constructors MUST explicitly indicate the expected input layout (row-major vs column-major) to prevent ambiguity.

#### Scenario: Row-Major Input
- **WHEN** `from_rows` is called with a flat array (e.g., visual representation)
- **THEN** the data MUST be transposed into column-major storage.

#### Scenario: Column-Major Input
- **WHEN** `from_cols` is called with a flat array
- **THEN** the data MUST be stored directly into the columns.
