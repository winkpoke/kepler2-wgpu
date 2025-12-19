# Change: Refactor MeshView to use Glam

## Why
The current `MeshView` implementation uses manual matrix multiplication and array manipulation for MVP (Model-View-Projection) calculations. This is error-prone, verbose, and less efficient than using a dedicated SIMD-optimized math library. `glam` is already a project dependency and used elsewhere.

## What Changes
- Refactor `MeshView::update_uniforms` to use `glam::Mat4` for all matrix operations.
- Remove manual matrix construction and multiplication logic.
- Ensure consistent coordinate system handling (preserving existing orthographic projection parameters).

## Impact
- Affected specs: rendering
- Affected code: `src/rendering/view/mesh/mesh_view.rs`
