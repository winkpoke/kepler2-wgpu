# MIP View Rendering Architecture

## Overview

This document describes the modular design for the **MIP rendering system**, organized around clear separation between rendering context, content, and view state.  
The design aims for **minimal viable structure** — avoiding over-engineering while maintaining scalability for multiple concurrent views and flexible render modes (MIP, MinIP, AIP, etc.).

---

## Core Components

### 1. `MipRenderContext`
- **Lifetime:** Global (per GPU device)
- **Responsibility:** Shared rendering setup and shader definitions.

#### Contents
- `wgpu::RenderPipeline`
- `wgpu::BindGroupLayout`s (for uniforms and textures)
- Shader modules

#### Purpose
Defines *how* rendering is done — pipeline state, shaders, and layout schemas.
It never holds per-view or per-dataset data.

---

### 2. `RenderContent`
- **Lifetime:** Dataset-level
- **Responsibility:** Holds GPU-resident image or volume data.

#### Contents
- `wgpu::Texture`
- `wgpu::TextureView`
- `wgpu::Sampler`

#### Purpose
Represents *what* is rendered — e.g., a CT volume, overlay map, or segmentation texture.
Multiple views can share the same `RenderContent`.

---

### 3. `MipViewWgpuImpl` (ViewImpl)
- **Lifetime:** View-level (per rendering instance)
- **Responsibility:** Centralizes GPU resources and bindings for a single view.
- **Implementation Note:** This is the concrete implementation of the `ViewImpl` concept in the design.

#### Contents
- Reference to `RenderContent` (as `Arc<RenderContent>`)
- Reference to `MipRenderContext`
- `wgpu::Buffer` (uniform buffer)
- `wgpu::BindGroup` (uniform bind group)
- `wgpu::BindGroup` (texture bind group)

#### Purpose
Owns all GPU-side state required to render a view:
- Creates uniform and texture bind groups based on the layouts from `MipRenderContext`.
- References GPU data from `RenderContent`.
- Uploads per-view uniforms (camera, window/level, projection).

---

### 4. `MipView`
- **Lifetime:** Screen/UI-level
- **Responsibility:** Encapsulates per-view display state and rendering logic.

#### Contents
- `Arc<ViewImpl>`
- Viewport position and dimensions
- Interaction or window/level parameters

#### Purpose
Defines *where and how* the view is presented on screen.
Delegates GPU rendering to its `ViewImpl`.

---

## Rendering Flow

```text
MipRenderContext  → defines pipelines and layouts
RenderContent     → holds GPU textures and samplers
MipViewWgpuImpl   → creates bind groups & uniform buffers
MipView           → handles screen state and rendering calls
```

During rendering:
1. `MipView` calls into its `MipViewWgpuImpl`.
2. `MipViewWgpuImpl` binds uniforms and textures.
3. The GPU pipeline from `MipRenderContext` executes.
4. The volume from `RenderContent` is sampled and drawn.

---

## Example Struct Relationships

```rust
struct MipRenderContext {
    pipeline: wgpu::RenderPipeline,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_group_layout: wgpu::BindGroupLayout,
}

struct RenderContent {
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

struct MipViewWgpuImpl {
    render_context: MipRenderContext,
    render_content: Arc<RenderContent>,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    texture_bind_group: wgpu::BindGroup,
}

struct MipView {
    wgpu_impl: Arc<MipViewWgpuImpl>,
    position: (i32, i32),
    dimensions: (u32, u32),
    config: MipConfig,
}
```

---

## Key Design Principles

| Principle | Description |
|------------|--------------|
| **Separation of concerns** | Each struct has a distinct lifetime and responsibility. |
| **Minimal viable structure** | Avoid unnecessary abstractions — only `ViewImpl` encapsulates GPU-specific logic. |
| **Scalability** | Supports multiple concurrent views or render modes with minimal duplication. |
| **Thread-safe** | Use `Arc` for shared references to context and content. |
| **Composable** | `MipView` and `ViewImpl` can be reused or replaced independently. |

---

## Summary Diagram

```
+------------------------+
| MipRenderContext       |   (global GPU state)
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
| MipViewWgpuImpl        |   (per-view GPU state)
|  - uniform buffer       |
|  - bind groups          |
+-----------+------------+
            |
            v
+------------------------+
| MipView                |   (UI-level view)
|  - position, size       |
|  - calls render()       |
+------------------------+
```

---

## Summary

| Layer | Holds | Purpose |
|--------|--------|----------|
| `MipRenderContext` | Pipelines + Layouts | Defines shader bindings |
| `RenderContent` | Texture + View + Sampler | Holds GPU data |
| `MipViewWgpuImpl` | Uniforms + BindGroups | Manages GPU bindings per view |
| `MipView` | Position + State + Arc<MipViewWgpuImpl> | UI and draw orchestration |

---

*This design keeps rendering components minimal, composable, and scalable for medical imaging applications such as CT MIP/MPR visualization.*
