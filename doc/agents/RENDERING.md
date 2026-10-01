# Rendering & GPU Patterns

**Last Updated**: 2025-01-15

## GPU Resource Management

### WebGPU Resources (device, queue, surfaces)

- **Device and Queue**: Wrapped in `Arc` for cheap cloning
- **Surface and TextureView**: Never clone these (single-use resources)
- **Buffers and Textures**: Reuse when possible to minimize allocations
- **Bind Groups**: Reuse to avoid redundant GPU state changes

### Resource Lifecycle

Drop order matters:
1. Textures
2. Buffers
3. Bind Groups
4. Pipelines

**Rationale**: Dependent resources must be dropped after resources they reference.

### Pipeline Management

- Pipelines are cached in render contexts
- Pass color attachment formats explicitly to pipeline/context constructors; never use process-wide surface format state
- Always check device capabilities before creating pipelines
- Use `pollster` to block on async operations in native code

### Pipeline Recreation

`GraphicsContext::surface_format()` reads the authoritative `SurfaceConfiguration.format`.
`DefaultViewFactory` passes that format to MPR, MIP, mesh, and orientation-cube contexts.

| Event | Resources updated |
|---|---|
| Ordinary resize, unchanged format | Surface configuration and layout; the persistent depth cache replaces size-dependent targets on the next mesh frame. No pipeline rebuild. |
| Same-device surface format change | Pass descriptors, view factory, and existing view GPU contexts; volume textures are reused. |
| Device replacement via `App::swap_graphics` | Volume textures re-uploaded and all existing view GPU resources rebuilt, including orientation gizmos. |

`App` synchronizes format-dependent resources before rendering and after resize.
`swap_graphics` returns a `Result`; upload failures leave the previous context intact.
GPU rebinding preserves CPU view state (slice/oblique basis, window/level, pan/zoom,
projection settings, and mesh rotation/opacity). Rebuild pipelines also when shaders
or bind group layouts change.

Headless regressions in `tests/rendering_correctness_tests.rs` validate independent
factory formats and existing-view rebinding across BGRA/RGBA formats and two devices.

## Coordinate Systems

### Four Coordinate Systems

1. **World**: 3D world coordinates (millimeters from volume origin)
2. **Screen**: 2D screen coordinates (pixels, origin at top-left)
3. **Voxel**: 3D voxel indices (integer indices into volume array)
4. **Base**: Base coordinate type (uses `glam::Vec3`)

### Transformations

Use `glam::Mat4` (column-major) for all 3D transformations:

```rust
use glam::Mat4, Vec3;

// Translation
let translate = Mat4::from_translation(Vec3::new(x, y, z));

// Scale
let scale = Mat4::from_scale(Vec3::new(sx, sy, sz));

// Rotation
let rotate = Mat4::from_quat(quat);

// Composition
let transform = translate * scale * rotate;
```

### Coordinate Transformations

Transformations between coordinate systems are managed in `src/core/coord/`:
- World ↔ Screen: View/projection matrices
- World ↔ Voxel: Volume spacing and dimensions
- Voxel ↔ Base: Direct mapping (1:1 for CT volumes)

## Volume Rendering

### Volume Texture Formats

Two texture format options for CT volumes:

#### R16Float
- **Pros**: Full Hounsfield Unit precision, direct float conversion
- **Cons**: Not all GPUs support filtering on R16Float
- **Use when**: GPU supports it, need maximum precision

#### Rg8Unorm
- **Pros**: Broad GPU support, hardware filtering
- **Cons**: Packed Hounsfield Units (8-bit integer), precision loss
- **Use when**: GPU doesn't support R16Float, performance critical

### Format Selection

Check GPU capability before choosing format:

```rust
fn device_supports_r16float(adapter: &wgpu::Adapter) -> bool {
    let features = adapter.features();
    features.contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES)
        // Additional checks may be needed
}
```

### Volume Encoding

See `src/data/volume_encoding.rs` for format selection logic:
- Automatic fallback from R16Float to Rg8Unorm
- Conversion utilities for Hounsfield Units

## View System

### View Trait

All renderable views implement the `View` trait:

```rust
pub trait View {
    fn render(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView);
    fn resize(&mut self, width: u32, height: u32);
}
```

### View Types

- **MprView**: Multi-Planar Reconstruction (transverse, sagittal, coronal)
- **MipView**: Maximum Intensity Projection
- **MeshView**: 3D mesh rendering

### View Factory

`DefaultViewFactory` creates GPU resources for views:
- Manages `RenderContent` for each view
- Handles texture loading and buffer creation
- Coordinates with `AppModel` for data
- Lazily owns one `Arc<MprRenderContext>` per device/attachment-format configuration
- Shares immutable MPR pipeline/layout/quad buffers across creation and bulk GPU rebinding
- Keeps uniforms and bind groups per view; the volume sampler remains shared in `RenderContent`

## Shader Management

### WGSL Shaders

Shaders are embedded as strings in `src/rendering/shaders/`:
- Volume casting shaders
- MIP shaders
- Mesh rendering shaders
- Utility shaders (coordinate transforms, etc.)

### Shader Compilation

- Compiled at pipeline creation time
- Errors reported at initialization
- Use `naga` for WGSL validation if needed

### Shader Uniforms

Common uniform structures:
- View transforms (model, view, projection matrices)
- Volume metadata (dimensions, spacing, window/level)
- Lighting parameters (for mesh views)

## Memory Management

### Texture Pools

`GraphicsContext` owns a persistent, depth-only `MeshTexturePool`:
- Reuse the texture **and cached view** for `(width, height, sample_count, depth_format)`.
- Keep at most one attachment; replace it on key changes and never allocate for zero sizes.
- Release the attachment when mesh views are removed; drop the pool with its graphics context.
- Never share a pool across devices. Graphics replacement constructs an empty pool.
- Mesh color attaches directly to the surface; no temporary color target is allocated.

The obsolete offscreen helpers `get_mesh_views`, `get_color_view`, and
`ensure_mesh_offscreen_texture` have been removed. Use `ensure_depth`/`depth_view`;
`ensure_textures`, `set_depth`, and `clear_depth_view` remain for compatibility.

### Buffer Management

- Reuse vertex/index buffers when possible
- Use `BufferUsages` flags correctly for GPU/CPU access
- Map/unmap buffers efficiently for dynamic updates

### Memory Leak Prevention

- Explicitly drop unused GPU resources
- Use weak references where circular dependencies exist
- Profile GPU memory usage with tools like `wgpu-profiler`

## Rendering Pipeline

### Frame Rendering Flow

```
App::render()
    ↓
PassExecutor::execute_frame()
    ↓
    ├─ Begin new frame (wgpu::Surface::get_current_texture)
    ├─ Begin render pass (wgpu::CommandEncoder::begin_render_pass)
    ├─ Render each view (View::render)
    ├─ Submit commands (wgpu::Queue::submit)
    └─ Present (wgpu::Surface::present)
```

### Multi-Pass Rendering

The system uses multiple render passes for different view types:
- **MeshPass**: 3D mesh rendering (depth buffer required)
- **MipPass**: Maximum Intensity Projection
- **SlicePass**: MPR slice rendering

### Command Buffers

- Use single command encoder per frame when possible
- Batch draw calls to minimize state changes
- Use `render_bundle` for repeated rendering operations

## Performance Optimization

### Event-Driven Frames

Built-in views compare small CPU state snapshots to their last uniform upload and
successful presentation. This catches setters and direct configuration edits without
invalidating on getters or no-op controls. Uploading does not consume pending presentation.
`View::mark_presented` is called only after the complete frame succeeds.

`RenderApp` schedules work in `AboutToWait`, not by recursively requesting redraws.
Static idle viewers use `ControlFlow::Wait`; animation, surface retries, and deferred
mesh recovery use bounded deadlines. Initial display, visual changes, layout clearing,
resize, unocclusion, and graphics replacement trigger frames. Minimized/occluded windows
wait without acquisition, retaining pending work for restoration.

Every acquired surface image is cleared once and **all visible views are drawn**, even
when only one view's uniforms changed. Surface images are not a retained backing store;
per-view draw skipping requires a separate cached-output/compositing design.
Custom `View` implementations retain continuous rendering by default; implement
`needs_redraw`/`mark_presented` to opt into invalidation-aware scheduling.

### GPU Synchronization

- Minimize CPU-GPU synchronization points
- Use asynchronous operations where possible
- Profile with `wgpu-profiler` to identify bottlenecks

### Texture Filtering

- Use appropriate filter modes (linear vs nearest) based on use case
- Mipmap for textures viewed at multiple scales
- Anisotropic filtering for oblique viewing angles

### Vertex Buffer Optimization

- Use `BufferUsages::VERTEX | BufferUsages::COPY_DST` for dynamic meshes
- Consider using `StorageBuffer` for large dynamic data
- Use `InstanceBuffer` pattern for repeated geometry

## Canvas and Window Handling

### Native (Winit)

- Window size controlled by Winit
- Canvas ID: `wasm-example` (see `src/lib.rs`)
- Resize events reconfigure the surface; pipelines are rebuilt only if the attachment format changes

### WASM (Web)

- Canvas size controlled by CSS, not Winit
- Must use `request_inner_size()` to set style width/height
- Canvas element ID must be `wasm-example` for embedding

### WASM Canvas Injection

```rust
// From src/lib.rs - get_render_app()
#[cfg(target_arch = "wasm32")]
{
    // Append canvas to DOM element with id "wasm-example"
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("wasm-example"))
        .expect("Canvas element not found");
}
```

## Debugging Rendering

### Common Issues

**Problem**: Surface errors on startup
```bash
# Check backend selection
KEPLER_WGPU_BACKEND=vulkan cargo run

# Enable validation layers
KEPLER_WGPU_VALIDATION=true cargo run
```

**Problem**: Pipeline mismatches
- Check that bind group layouts match shader expectations
- Ensure texture formats are compatible
- Verify buffer sizes match shader requirements

### Validation Layers

Enable WGPU validation for debugging:
```bash
KEPLER_WGPU_VALIDATION=true cargo run
```

### Profiling

Use `wgpu-profiler` or platform-specific tools:
- **RenderDoc**: Frame capture and analysis (native)
- **Chrome DevTools**: WebGPU inspector (WASM)
- **PIX**: Windows GPU debugging
- **Xcode GPU Debugger**: macOS

## Related Documentation

- **Quick Reference**: `QUICK_REFERENCE.md` - Common rendering tasks
- **Architecture**: `ARCHITECTURE.md` - View system and modules
- **Conventions**: `CONVENTIONS.md` - Coding patterns
- **Pitfalls**: `PITFALLS.md` - Common rendering mistakes
- **Render Architecture**: `doc/rendering/unified-render-architecture.md` - Deep dive
