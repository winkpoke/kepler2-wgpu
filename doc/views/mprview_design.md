# MPR View Rendering Architecture

## Overview

This document describes the modular design for the **MPR (Multi-Planar Reconstruction) rendering system**, organized around clear separation between rendering context, content, and view state.  
The design aims for **minimal viable structure** — avoiding over-engineering while maintaining scalability for multiple concurrent views and flexible anatomical orientations (Transverse, Coronal, Sagittal, Oblique).

This architecture follows the same design principles as the MIP rendering system, ensuring consistency and maintainability across the medical imaging framework.

---

## Core Components

### 1. `MprRenderContext`
- **Lifetime:** Global (per GPU device)
- **Responsibility:** Shared rendering setup and shader definitions for MPR visualization.

#### Contents
- `wgpu::RenderPipeline`
- `wgpu::BindGroupLayout`s (for uniforms and textures)
- Shader modules for slice-based rendering
- Vertex and index buffers for quad geometry

#### Purpose
Defines *how* MPR rendering is done — pipeline state, shaders, and layout schemas.
It never holds per-view or per-dataset data. Manages the common GPU resources needed for all MPR views.

---

### 2. `RenderContent`
- **Lifetime:** Dataset-level
- **Responsibility:** Holds GPU-resident image or volume data.

#### Contents
- `wgpu::Texture` (3D volume texture)
- `wgpu::TextureView`
- `wgpu::Sampler`
- `wgpu::TextureFormat`

#### Purpose
Represents *what* is rendered — e.g., a CT volume, overlay map, or segmentation texture.
Multiple MPR views can share the same `RenderContent` for different anatomical orientations.

---

### 3. `MprViewWgpuImpl` (ViewImpl)
- **Lifetime:** View-level (per rendering instance)
- **Responsibility:** Centralizes GPU resources and bindings for a single MPR view.
- **Implementation Note:** This is the concrete implementation of the `ViewImpl` concept in the design.

#### Contents
- Reference to `RenderContent` (as `Arc<RenderContent>`)
- Reference to `MprRenderContext`
- `wgpu::Buffer` (vertex uniform buffer)
- `wgpu::Buffer` (fragment uniform buffer)
- `wgpu::BindGroup` (texture bind group)
- `wgpu::BindGroup` (vertex uniform bind group)
- `wgpu::BindGroup` (fragment uniform bind group)
- Medical coordinate system bases (`Base<f32>` for screen and UV coordinates)

#### Purpose
Owns all GPU-side state required to render an MPR view:
- Creates uniform and texture bind groups based on the layouts from `MprRenderContext`.
- References GPU data from `RenderContent`.
- Uploads per-view uniforms (transform matrices, window/level, slice position).
- Manages coordinate system transformations for medical imaging accuracy.

---

### 4. `MprView`
- **Lifetime:** Screen/UI-level
- **Responsibility:** Encapsulates per-view display state and medical imaging parameters.

#### Contents
- `Arc<MprViewWgpuImpl>`
- Viewport position and dimensions
- Medical imaging parameters (slice position, window/level, scale, translation)
- Anatomical orientation (Transverse, Coronal, Sagittal, Oblique)
- Interaction state (pan, zoom)

#### Purpose
Defines *where and how* the MPR view is presented on screen.
Delegates GPU rendering to its `MprViewWgpuImpl` while managing medical imaging-specific state.

---

## Rendering Flow

```text
MprRenderContext  → defines pipelines and layouts for slice rendering
RenderContent     → holds GPU textures and samplers (3D volume)
MprViewWgpuImpl   → creates bind groups & uniform buffers for orientation
MprView           → handles medical state and rendering calls
```

During rendering:
1. `MprView` updates medical parameters (slice, window/level, transforms).
2. `MprView` calls into its `MprViewWgpuImpl`.
3. `MprViewWgpuImpl` uploads uniforms and binds GPU resources.
4. The GPU pipeline from `MprRenderContext` executes slice rendering.
5. The volume from `RenderContent` is sampled at the current slice plane.

---

## Example Struct Relationships

```rust
struct MprRenderContext {
    pipeline: wgpu::RenderPipeline,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    vertex_uniform_bind_group_layout: wgpu::BindGroupLayout,
    fragment_uniform_bind_group_layout: wgpu::BindGroupLayout,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
}

struct RenderContent {
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    texture_format: wgpu::TextureFormat,
}

struct MprViewWgpuImpl {
    render_context: MprRenderContext,
    render_content: Arc<RenderContent>,
    vertex_uniform_buffer: wgpu::Buffer,
    fragment_uniform_buffer: wgpu::Buffer,
    texture_bind_group: wgpu::BindGroup,
    vertex_uniform_bind_group: wgpu::BindGroup,
    fragment_uniform_bind_group: wgpu::BindGroup,
    base_screen: Base<f32>,
    base_uv: Base<f32>,
}

struct MprView {
    wgpu_impl: Arc<MprViewWgpuImpl>,
    position: (i32, i32),
    dimensions: (u32, u32),
    orientation: Orientation,
    slice: f32,
    scale: f32,
    translate: [f32; 3],
    pan: [f32; 3],
}
```

---

## Medical Imaging Considerations

### Coordinate Systems
MPR views maintain multiple coordinate systems for medical accuracy:
- **Screen coordinates**: Pixel positions on the display
- **UV coordinates**: Texture sampling coordinates (0-1 range)  
- **Medical coordinates**: Real-world millimeter measurements
- **Volume coordinates**: Voxel indices in the 3D dataset

### Anatomical Orientations
The system supports standard medical imaging orientations:
- **Transverse (Axial)**: Horizontal slices through the body
- **Coronal**: Vertical slices from front to back
- **Sagittal**: Vertical slices from side to side
- **Oblique**: Custom angled slices for specialized views

### Window/Level Processing
Critical for medical imaging visualization:
- **Window Level**: Controls image brightness (tissue-specific)
- **Window Width**: Controls image contrast (tissue-specific)
- Applied in fragment shader for optimal performance

---

## Key Design Principles

| Principle | Description |
|------------|--------------|
| **Separation of concerns** | Each struct has a distinct lifetime and responsibility. |
| **Medical imaging accuracy** | Maintains precise coordinate systems and measurements. |
| **Scalability** | Supports multiple concurrent views with different orientations. |
| **Thread-safe** | Use `Arc` for shared references to context and content. |
| **Composable** | `MprView` and `MprViewWgpuImpl` can be reused independently. |
| **Memory efficient** | Share `RenderContent` between multiple MPR views. |

---

## Summary Diagram

```
+------------------------+
| MprRenderContext       |   (global GPU state)
|  - pipeline            |
|  - bind group layouts  |
|  - geometry buffers    |
+-----------+------------+
            |
            v
+------------------------+
| RenderContent          |   (dataset / 3D texture)
|  - texture/view/sampler|
|  - format info         |
+-----------+------------+
            |
            v
+------------------------+
| MprViewWgpuImpl        |   (per-view GPU state)
|  - uniform buffers     |
|  - bind groups         |
|  - coordinate bases    |
+-----------+------------+
            |
            v
+------------------------+
| MprView                |   (medical imaging view)
|  - position, size      |
|  - orientation, slice  |
|  - window/level        |
|  - calls render()      |
+------------------------+
```

---

## Summary

| Layer | Holds | Purpose |
|--------|--------|----------|
| `MprRenderContext` | Pipelines + Layouts + Geometry | Defines slice rendering pipeline |
| `RenderContent` | 3D Texture + View + Sampler | Holds volume data |
| `MprViewWgpuImpl` | Uniforms + BindGroups + Coordinates | Manages GPU bindings per orientation |
| `MprView` | Medical State + Position + Arc<MprViewWgpuImpl> | Medical imaging orchestration |

---

## Migration Benefits

Adopting this architecture for MPR views provides:

1. **Consistency**: Same design patterns as MIP rendering system
2. **Resource Sharing**: Multiple MPR views can share `RenderContent` via `Arc`
3. **Memory Efficiency**: Reduced GPU memory usage through texture sharing
4. **Maintainability**: Clear separation of concerns and responsibilities
5. **Scalability**: Easy to add new anatomical orientations or view types
6. **Thread Safety**: Arc-based sharing enables multi-threaded access

---

*This design keeps MPR rendering components minimal, composable, and scalable for medical imaging applications while maintaining the precision and accuracy required for clinical visualization.*