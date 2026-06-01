# Mesh View Rendering Architecture

## Overview

This document describes the architecture of **`MeshView`** as implemented under `src/rendering/view/mesh/`.

It covers:

- Responsibilities and boundaries of `MeshView`
- Construction from `CTVolume` → `RenderContent` → `MeshView`
- Per-frame rendering integration (MeshPass via `PassExecutor`)
- Key control surfaces: uniforms, ROI, opacity, rotation, mode, depth/offscreen pooling, adaptive quality, and more.
- Resilience mechanisms: fallback handling and quality control
- External control entry points via `GLCanvas` / `UserEvent`

Important note: in this codebase, `MeshView` is primarily a **CT volume ray-marching view**. It also renders a small **orientation cube gizmo** in the bottom-left corner. It is not strictly a “generic triangle mesh renderer”.

---

## Core Components

### 1. `MeshView`

- **Lifetime:** UI/layout-level (per view instance)
- **Responsibility:** Owns the view rectangle and user-facing state (rotation, ROI, opacity, mode), and delegates GPU work to attached render contexts.

The view maintains two rendering contexts:

- `volume_ctx: Option<Arc<MeshRenderContext>>` for volume ray-marching
- `orientation_cube_ctx: Option<Arc<BasicMeshContext>>` for the orientation cube gizmo

#### Contents
- `MeshView` definition and attachment APIs

---

### 2. `MeshRenderContext` (volume path)

- **Lifetime:** GPU/device-level (shareable via `Arc`)
- **Responsibility:** Holds the GPU pipeline, bind groups, and uniform buffer to render the 3D volume texture.

#### Contents
- `MeshRenderContext`
- Uniform uploads:`MeshRenderContext::update_uniforms`

---

### 3. `BasicMeshContext` (orientation cube)

- **Lifetime:** GPU/device-level (shareable via `Arc`)
- **Responsibility:** Renders a small cube (unit cube geometry) with a basic lighting pipeline.

#### Contents
- `BasicMeshContext`
- Decode uploads:`BasicMeshContext::update_uniforms`

---

### 4. `RenderContent` (shared dataset GPU resources)

- **Lifetime:** Dataset-level
- **Responsibility:** Owns the 3D volume texture/view/sampler and provides shader decode parameters.

#### Contents
- `RenderContent`
- Decode parameters:`RenderContent::decode_parameters`

---

### 5. `MeshTexturePool` (depth/offscreen reuse)

- **Lifetime:** Cross-frame cache
- **Responsibility:** Maintains depth and optional offscreen textures with correct dimensions.

#### Contents
- `MeshTexturePool`
- Used from:`execute_mesh_pass`

---

## Example Struct Relationships

```rust
struct MeshRenderContext {
    pipeline: wgpu::RenderPipeline,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_group_layout: wgpu::BindGroupLayout,
}

struct RenderContent {
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

struct MeshViewWgpuImpl {
    render_context: MeshRenderContext,
    render_content: Arc<RenderContent>,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    texture_bind_group: wgpu::BindGroup,
}

struct MeshView {
    wgpu_impl: Arc<MeshViewWgpuImpl>,
    position: (i32, i32),
    dimensions: (u32, u32),
}
```

---

## ROI (Region of Interest)

The external API accepts ROI corners in **world space (millimeters)**:

- Event definition (GLCanvas protocol):`UserEvent` mesh events
- Handler:`App::set_mesh_roi`

Conversion rules:

1. Convert mm → volume space using `CTVolume.base.matrix.inverse()`
2. Normalize using `(nx-1, ny-1, nz-1)` and clamp to `0..1`
3. Store into `MeshView` and upload each frame as uniforms

Therefore:

- The shader always sees ROI as **normalized volume coordinates**
- The input corner ordering is normalized into min/max by the handler

---

## Quality Control and Fallback Behavior

Two resilience mechanisms exist:

1. Render error containment and fallback:
  - `handle_render_error`
  - `FallbackMode`
2. Adaptive quality control:
  - `QualityController`
  - Frame timing hooks: `start_frame_timing/end_frame_timing`

---

## Related Shaders and Pipelines

- Volume ray-marching shader: `mesh.wgsl` via:`create_volume_pipeline`
- Orientation cube shader: `mesh_basic.wgsl` via:`create_basic_mesh_pipeline_with_lighting`

---

## Summary Diagram

```
+------------------------+
| MeshRenderContext       |   (global GPU state)
|  - pipeline            |
|  - bind group layouts  |
+-----------+------------+
            |
            v
+------------------------+
| RenderContent          |   (dataset / texture)
|  - texture/view/sampler|
+-----------+------------+
            |
            v
+------------------------+
| MeshViewWgpuImpl        |   (per-view GPU state)
|  - uniform buffer       |
|  - bind groups          |
+-----------+------------+
            |
            v
+------------------------+
| MeshView                |   (UI-level view)
|  - position, size       |
|  - window/level         |
|  - opacity              |
|  - mode                 |
|  - rotation             |
|  - roi                  |
|  - calls render()       |
+------------------------+
```

---

## Summary

| Layer | Holds | Purpose |
|--------|--------|----------|
| `MeshRenderContext` | Pipelines + Layouts | Defines shader bindings |
| `RenderContent` | Texture + View + Sampler | Holds GPU data |
| `MeshViewWgpuImpl` | Uniforms + BindGroups | Manages GPU bindings per view |
| `MeshView` | Position + State + Arc<MeshViewWgpuImpl> | UI and draw orchestration |

---

*This design keeps rendering components minimal, composable, and scalable for medical imaging applications such as CT mesh visualization.*
