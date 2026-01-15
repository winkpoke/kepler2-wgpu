# Basic Mesh Rendering Implementation

## Overview

This document describes the implementation of basic mesh rendering functionality in the kepler-wgpu medical imaging framework. The implementation is based on the analysis of temporary reference files and provides fundamental 3D triangle rendering capabilities.

## WGPU Code Organization (Current)

The codebase organizes WGPU code in three layers:

### 1) Device/Surface Bootstrap (Graphics)

- `src/rendering/core/graphics.rs` owns the global WGPU handles:
  - `Graphics` holds `wgpu::Surface`, `wgpu::SurfaceConfiguration`, `wgpu::Adapter`, plus `Arc<wgpu::Device>` and `Arc<wgpu::Queue>`.
  - `GraphicsContext` wraps `Graphics` together with the render-pass executor.
- Swapchain format is set after `surface.configure` and also exposed through `src/rendering/core/pipeline.rs`:
  - `set_swapchain_format` stores the surface format for pipeline creation helpers.

### 2) Frame Orchestration (Render Passes)

- `src/application/app.rs` drives the frame:
  - acquires the surface frame,
  - creates a `wgpu::CommandEncoder`,
  - delegates pass execution to `PassExecutor`,
  - submits and presents.
- `src/rendering/core/render_pass.rs` defines:
  - `PassRegistry` to decide which passes run (Mesh / MIP / Slice),
  - `PassExecutor` to begin `wgpu::RenderPass` objects and dispatch rendering to views.

### 3) Feature Rendering (Views)

Each view type typically splits WGPU resources into:

- **RenderContext**: shared GPU resources for that feature (pipeline, bind group layouts, shared geometry buffers).
- **RenderContent**: shared textures and samplers (e.g., the 3D CT volume texture).
- **Per-view WGPU impl**: per-view bind groups and uniform buffers.
- **View**: CPU-side state, per-frame uniform updates, and issuing draw calls.

#### MPR (slice views)

- Shared context: `src/rendering/view/mpr/mpr_render_context.rs`
  - owns the texture-quad pipeline, bind group layouts, and a shared quad vertex/index buffer.
- Per-view GPU state: `src/rendering/view/mpr/mpr_view_wgpu_impl.rs`
  - creates per-view bind groups using the shared layouts,
  - owns per-view uniform buffers (vertex + fragment).
- Draw call: `src/rendering/view/mpr/mpr_view.rs`
  - binds pipeline,
  - sets bind groups (0: volume texture, 1: vertex uniforms, 2: fragment uniforms),
  - binds shared quad buffers,
  - `draw_indexed`.

#### MIP (volume projection)

- Context + per-view impl live in `src/rendering/view/mip/mod.rs`:
  - pipeline bind group order is (0: volume texture, 1: MIP uniforms),
  - draw call uses a fullscreen primitive generated from `vertex_index` (no vertex buffer).

#### Mesh (3D geometry)

- Per-mesh context: `src/rendering/view/mesh/basic_mesh_context.rs`
  - owns pipeline, vertex/index buffers, and uniform bind groups.
- CPU-side view state: `src/rendering/view/mesh/mesh_view.rs`
  - computes matrices, transposes for WGSL, uploads via `queue.write_buffer`, then draws.

## Architecture

### Core Components

1. **BasicMeshContext** (`src/rendering/view/mesh/basic_mesh_context.rs`)
   - Manages vertex and index buffers for simple mesh data
   - Handles uniform buffer for MVP (Model-View-Projection) matrix
   - Uses the simplified `mesh_basic.wgsl` shader

2. **MeshView** (`src/rendering/view/mesh/mesh_view.rs`)
   - Calculates MVP matrices with proper camera positioning
   - Integrates with the existing camera system
   - Handles uniform updates for rendering

3. **Shader Integration** (`src/rendering/shaders/mesh_basic.wgsl`)
   - Simple vertex and fragment shader for basic mesh rendering
   - Single uniform binding for MVP matrix
   - Vertex color pass-through

## Key Implementation Details

### MVP Matrix Calculation

The implementation correctly calculates the Model-View-Projection matrix:

```rust,no_run
// Model matrix with Z-translation to position triangle in front of camera
let model_matrix = Matrix4x4::from_array([
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, -2.0,  // Translate back along Z-axis
    0.0, 0.0, 0.0, 1.0,
]);

// Combine with view-projection matrix
let mvp_matrix = view_proj_matrix.multiply(&model_matrix);
```

### Camera Setup

- Camera positioned at `[0.0, 0.0, 3.0]`
- Looking at origin `[0.0, 0.0, 0.0]`
- Uses orthogonal projection for medical imaging accuracy
- Aspect ratio calculated from viewport dimensions

### Vertex Data Structure

```rust,no_run
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}
```

### Shader Uniforms

```wgsl
struct Uniforms {
    model_view_proj: mat4x4<f32>,
}
```

## Integration Points

### Pipeline Management

The implementation uses `create_simple_mesh_pipeline()` which:
- Loads the `mesh_basic.wgsl` shader
- Creates a single bind group layout for uniforms
- Configures depth testing and back-face culling

### Rendering Process

1. Calculate MVP matrix based on camera and model transformation
2. Update uniform buffer with MVP matrix
3. Set render pipeline and bind groups
4. Bind vertex and index buffers
5. Execute indexed draw call

## Testing and Verification

The implementation has been tested and verified to:
- Successfully build with the `mesh` feature enabled
- Execute the complete rendering pipeline without errors
- Properly calculate and apply MVP transformations
- Render a basic triangle with vertex colors

### Debug Output Example

```
[DEBUG] Camera position: [0.0, 0.0, 3.0], target: [0.0, 0.0, 0.0]
[DEBUG] Model matrix (translated): [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, -2.0], [0.0, 0.0, 0.0, 1.0]]
[DEBUG] Combined MVP matrix: [[0.4, 0.0, 0.0, 0.0], [0.0, 0.4, 0.0, 0.0], [0.0, 0.0, -0.13333334, 0.26666668], [0.0, 0.0, -3.3333333, 7.6666665]]
[DEBUG] Starting render - 3 indices, 3 vertices
[DEBUG] Draw indexed called: indices 0..3, base_vertex 0, instances 0..1
```

## Usage

To enable basic mesh rendering:

1. Build with the mesh feature: `cargo build --features mesh`
2. Run the application: `cargo run --features mesh`
3. The mesh rendering will be automatically enabled in the render loop

## Future Enhancements

- Support for more complex mesh geometries
- Texture mapping capabilities
- Advanced lighting models
- Animation and transformation controls
- Integration with medical imaging data visualization

## Compatibility

- Works with both native and WebAssembly targets
- Compatible with existing camera and projection systems
- Maintains medical imaging accuracy requirements (orthogonal projection)
- Follows the project's incremental development approach
