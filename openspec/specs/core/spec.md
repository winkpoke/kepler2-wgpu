# core Specification

## Purpose
Provides fundamental building blocks for the application, including standardized error handling, mathematical primitives, coordinate system definitions, geometry transformations, timing utilities, and medical imaging specific value mapping (Window/Level).
## Requirements
### Requirement: Standardized Math Library
The core geometry subsystem MUST use a standardized, SIMD-optimized linear algebra library (e.g., `glam`) for internal calculations to ensure performance and maintainability. The chosen library MUST be compatible with WebAssembly (WASM) targets.

#### Scenario: Matrix Construction
Given a request to build a geometric base (e.g., Transverse, Coronal)
When the geometry builder computes the transformation matrix
Then it should use `glam::Mat4` for intermediate operations
And the result should be numerically equivalent to the defined physical-to-screen transformation.

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

