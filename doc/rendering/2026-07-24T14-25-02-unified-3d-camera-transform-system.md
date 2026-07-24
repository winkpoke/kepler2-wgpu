# Unified 3D Camera Transform System

## Summary

The 3D view now uses one camera-driven transform pipeline for both spine meshes and ray-marched volume rendering. Rotation, pan, zoom, projection selection, and reset all flow through the same right-handed camera state and the same `view_proj` / `inv_view_proj` matrices.

## What Changed

- Added `ProjectionMode::{Orthographic, Perspective}` to the shared mesh camera in `src/rendering/view/mesh/camera.rs`.
- Changed camera zoom to work as camera-distance scaling so orthographic and perspective modes stay visually stable when switching.
- Added a locked `SceneTransformState` snapshot in `src/rendering/view/mesh/mesh_view.rs` to keep mesh and volume uniforms synchronized within the same frame.
- Replaced the old volume `rotation` matrix uniform with shared `inv_view_proj` and `view_proj` matrices in:
  - `src/rendering/view/mesh/mesh.rs`
  - `src/rendering/shaders/mesh.wgsl`
- Updated mesh normalization so segmentation meshes and imported OBJ meshes live in the shared `[0,1]^3` world cube consumed by the volume renderer.
- Preserved reset behavior through `MeshView::reset_transform_state()` and `App::reset_mesh()`.

## Validation

- `cargo test --lib mesh_view -- --nocapture`
- `cargo check`

## Notes

- This change unifies the math and state path, but real-world frame-rate validation still needs interactive runtime profiling on representative CT + mesh datasets.
- Perspective mode is available through the runtime API (`MeshView::set_projection_mode`, `App::set_mesh_projection_mode`) and is preserved across layout state restore.
