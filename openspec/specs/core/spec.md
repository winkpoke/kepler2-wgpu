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

