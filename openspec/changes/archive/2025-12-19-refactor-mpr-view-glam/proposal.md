# Refactor MprView to use glam

## Context
The `MprView` currently relies on a custom `Matrix4x4` implementation and `Base<f32>` struct for its geometry calculations. We are moving towards using `glam` for all linear algebra operations to improve performance (SIMD) and maintainability (standard library).

## Problem
`MprView` uses `crate::core::coord::{Base, Matrix4x4}` which performs software-based matrix operations. This is less efficient than `glam` and inconsistent with the rest of the rendering pipeline which is adopting `glam`.

## Solution
Refactor `MprView` to use `glam::Mat4` and `glam::Vec3` for all internal geometry state and calculations.
Crucially, **preserve the exact existing logic** regarding coordinate transformations, scaling behavior (inversion), and matrix multiplication order.

## Scope
- `src/rendering/view/mpr/mpr_view.rs`
- `src/rendering/view/mpr/mpr_view_wgpu_impl.rs` (already updated signature)

## Implementation Details
1.  **Storage**: Replace `Base<f32>` fields with `glam::Mat4`. Replace `[f32; 3]` pan with `glam::Vec3`.
2.  **Initialization**: Convert `Base` structs from `GeometryBuilder` to `glam::Mat4` immediately upon construction.
3.  **Transform Logic**:
    *   Replicate `Base::scale` behavior: Apply `Scale(1/s)`.
    *   Replicate `Base::translate` behavior: Apply `M * Translate(t)`.
    *   Maintain transformation order: `Pan -> Center -> Scale -> Uncenter`.
    *   Ensure WGPU matrix upload is column-major (no transpose needed as `glam` is column-major and matches `Matrix4x4` storage).
4.  **Coordinate Conversion**: Update `screen_coord_to_world` and `set_center_at_point_in_mm` to use `glam` types.

## Verification
- Unit tests or logic verification to ensure `glam` operations produce identical results to `Matrix4x4` operations.
- Ensure `update_transform_matrix` sends correct data to GPU.
