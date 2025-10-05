# 3D Mesh Rendering Design Proposal

This document summarizes the proposed design to add 3D mesh rendering support to the kepler-wgpu project while preserving the existing 2D MPR workflows.

## Goals
- Integrate 3D mesh visualization alongside 2D slice-based MPR views
- Maintain medical imaging accuracy and performance
- Keep architecture extensible and cross-platform (native + WASM)

## Architecture Overview
Current: CTVolume → MPRView (2D slices)

Proposed: CTVolume → MPRView (2D slices) + MeshView (3D meshes)

This extends the existing view system rather than replacing it, enabling mixed layouts combining 2D and 3D.

## Key Components

### Mesh Data Structures
- Mesh: vertices, indices, material, transform
- MeshVertex: position, normal, tex_coords, optional color
- Material: diffuse/specular/shininess, optional texture

### New View Type: MeshView
- A 3D view implementing the View trait, with camera, lighting, and mesh collection management
- Methods: add/remove meshes, camera configuration, lighting configuration

### Camera System
- Position/target/up vectors
- FOV, aspect ratio, near/far
- Provides view and projection matrices

### Lighting System
- Ambient light
- Directional lights
- Point lights (optional)

## Rendering Pipeline Strategy

### Option 1: Separate Render Passes (Recommended default)
- 3D Mesh Pass: depth testing, lighting shaders
- 2D Slice Pass: texture sampling, no depth
- UI/Overlay Pass: alpha blending

Benefits:
- Optimized for each content type
- Different shader pipelines without cross-interference
- Easier performance tuning per pass

### Option 2: Single Unified Render Pass
- One pass with multiple pipelines (mesh + slice) sharing a depth buffer

Benefits:
- Fewer render pass transitions
- Natural depth interaction between 2D overlays and 3D meshes

### Recommended: Hybrid Approach
- Use separate passes for layouts with distinct 2D and 3D views
- Use a unified pass only for mixed content views where 2D overlays must interact with 3D depth

## Depth Buffer Strategy
- 3D Meshes: Depth32Float, depth write enabled, compare Less
- Pure 2D: No depth attachment
- Mixed Views: Shared depth texture for both pipelines; 3D first, then 2D overlays (depth read, no write)

## Pipeline Setup
- Mesh pipeline: position/normal/UV; backface culling; lighting in shader; depth enabled
- Slice pipeline: position/UV; texture sampling; no depth; optimized for CT volume slices
- Overlay pipeline: alpha blending, optional depth test

## Integration Points
- View system: add MeshView implementing the View trait and Renderable
- RenderContext: keep current 2D context as-is; introduce MeshRenderContext for 3D (depth, camera, lighting buffers)
- GeometryBuilder: add helpers like isosurface extraction (marching cubes) and volume bounds mesh
- Shaders: new mesh.wgsl for 3D; keep shader_tex.wgsl for 2D volume sampling

References in codebase:
- 2D view composition: <mcfile name="view.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\view.rs"></mcfile>
- 2D render context and pipeline: <mcfile name="render_context.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\render_context.rs"></mcfile>
- Geometry helpers: <mcfile name="geometry.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\geometry.rs"></mcfile>
- Existing shaders: <mcfile name="shader_tex.wgsl" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\shader\\shader_tex.wgsl"></mcfile>

## File Structure Additions
```
src/
├── mesh/
│   ├── mod.rs
│   ├── mesh.rs           // Mesh data structures
│   ├── material.rs       // Material system
│   ├── camera.rs         // Camera implementation
│   ├── lighting.rs       // Lighting system
│   ├── marching_cubes.rs // Isosurface extraction
│   └── mesh_view.rs      // 3D mesh view implementation
├── shader/
│   ├── mesh.wgsl         // 3D mesh shaders
│   └── mesh_depth.wgsl   // Depth-only pass for shadows (optional)
```

## Performance Considerations
- Level of Detail (LOD) for distant meshes
- Frustum culling for off-screen meshes
- Instancing support for repeated geometry
- Async mesh generation (e.g., marching cubes) off the main thread
- MSAA for smooth geometry edges where needed

## Phased Implementation Plan
1. Foundations
   - Create mesh data structures, camera, lighting modules
   - Implement MeshRenderContext and mesh.wgsl shaders
2. Basic 3D Rendering
   - Render a simple test mesh (box/sphere)
   - Add camera controls (orbit/pan/zoom)
3. Integration with Layouts
   - Add MeshView and integrate into existing layout system
   - Support separate render passes per view
4. Mixed Views
   - Implement unified pass for overlays on 3D
   - Depth testing configuration for 2D overlays
5. Mesh Generation
   - Add GeometryBuilder helpers (volume bounds, isosurface via marching cubes)
6. Optimization
   - Add LOD, frustum culling, instancing, async meshing

## Summary
This design introduces a robust 3D mesh rendering path alongside existing 2D MPR views, using a hybrid render pass strategy for performance and visual consistency. It keeps the architecture modular, adds a dedicated MeshView and MeshRenderContext, and outlines a clear path to add isosurface extraction and high-quality rendering in both native and WASM environments.