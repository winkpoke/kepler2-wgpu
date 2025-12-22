# core Specification

## REMOVED Requirements

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

## MODIFIED Requirements

### Requirement: Standardized Math Library
The core geometry subsystem MUST use `glam` as the primary linear algebra library for calculations. The `Base<T>` struct MUST be retained as the public interface for coordinate systems, and it MUST directly contain a `glam` matrix (e.g. `glam::Mat4` or `glam::DMat4`) instead of the custom `Matrix4x4` type.

### Requirement: Legacy Matrix Support
The `Matrix4x4` struct MUST be retained in the codebase to support legacy consumers. It MAY be refactored to wrap `glam` or provide conversion methods to/from `glam` types, but its definition MUST NOT be removed.

#### Scenario: Preserved Interface
Given an existing function that takes `&Base<f32>`
When the underlying matrix implementation changes to `glam`
Then the function signature MUST remain valid (or require minimal adaptation)
And the behavior of methods like `to_base()` MUST remain functionally identical.

#### Scenario: Matrix Operations
Given a `Base` object
When performing scaling, translation, or inversion
Then the operation MUST be performed using `glam`'s SIMD-optimized methods instead of custom loops.
