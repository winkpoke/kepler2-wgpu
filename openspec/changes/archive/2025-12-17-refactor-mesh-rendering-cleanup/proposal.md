# Refactor Mesh Rendering Cleanup

## Summary
Clean up the `src/rendering/view/mesh` directory by removing unused code, dead files, and refactoring redundant logic to improve maintainability and performance clarity.

## Why
The current mesh rendering codebase contains "zombie code" (unused contexts, materials) and hardcoded values that hinder maintainability and confuse the architecture. Specifically:
- `MeshRenderContext` is a complex unused struct.
- `Material` system is defined but not connected to the rendering pipeline.
- `Lighting` conversion uses hardcoded values instead of the actual struct fields.
- `BasicMeshContext` has an unnecessary `Drop` implementation.
- Domain-specific geometry (`spine_vertebra`) is coupled with the generic `Mesh` struct.

## Proposed Changes

### 1. Remove Unused Components
- **Rename** `src/rendering/view/mesh/mesh_render_context.rs` to `_mesh_render_context.rs`: Completely unused complex context.
- **Rename** `src/rendering/view/mesh/material.rs` to `_material.rs`: Unused material system.
- **Update** `MeshView`: Remove the `material` field and related imports.

### 2. Refactor Lighting Logic
- **Update** `Lighting::to_basic_uniforms` in `src/rendering/view/mesh/mesh.rs`:
  - Replace hardcoded colors and intensities with values from the `Lighting` struct.
  - Add necessary fields to `Lighting` struct (e.g., `ambient_color`, `ambient_intensity`, `light_color`) with the current hardcoded values as defaults.

### 3. Simplify Context
- **Update** `BasicMeshContext`: Remove the explicit `Drop` implementation. `wgpu` resources are automatically managed.

## Verification Plan
- Run `cargo build` to ensure no broken references.
- Run `cargo test` to ensure no regressions.
- Manual verification: Check that the basic cube/mesh rendering still works (lighting should look the same or better due to correct config).
