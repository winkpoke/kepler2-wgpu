# Shader Overview (src/rendering/shaders)

This document describes the current shader inventory and their intended roles.

## Files
- `mesh.wgsl`
- `mesh_basic.wgsl`
- `mip.wgsl`
- `shader_tex.wgsl`

## Responsibilities (conceptual)
- Mesh shaders (`mesh*.wgsl`):
  - Render triangle meshes, optionally with basic lighting or volume-related coloring.
- MIP shader (`mip.wgsl`):
  - Implements projection (e.g., Maximum Intensity Projection) for volume rendering modes supported by the view layer.
- Texture shaders (`shader_tex.wgsl`):
  - Draw textured quads / intermediate outputs where used by views.

## Integration points
- Pipeline creation and bind group layouts are defined in `src/rendering/core/*`.
- View-specific render logic lives under `src/rendering/view/*` and selects the appropriate pipelines/shaders.