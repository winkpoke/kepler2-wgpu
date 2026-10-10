# Rendering & GPU Patterns

**Status:** Current reference, reviewed against `947b4fc` on 2026-10-01.
For signed storage/filtering precision, use [the encoding contract](../rendering/volume-encoding.md).

## GPU Resource Management

### WebGPU Resources (device, queue, surfaces)

- **Device and Queue**: Wrapped in `Arc` for cheap cloning
- **Surface and TextureView**: Borrow from their owner or create the needed view; do not assume these wgpu handles implement Clone
- **Buffers and Textures**: Reuse when possible to minimize allocations
- **Bind Groups**: Reuse to avoid redundant GPU state changes

### Resource Lifecycle

Retain resources in the owning graphics/view context and release unused caches.
There is no project-wide textures/buffers/bind-groups/pipelines drop sequence.
wgpu tracks resource references; follow the concrete owner's lifetime and avoid
cross-device reuse or unbounded retention.

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

The complete bulk operation is `AppView::rebind_gpu_resources`: it receives a borrowed
device/queue/attachment-format configuration, selects a compatible factory internally,
and rebuilds existing views using its shared MPR context. `App` prepares replacement
volume uploads before this operation; `None` retains same-device volume content.
Standalone per-view rebuilding methods retain their existing interfaces. See the
[production bulk-rebinding regression](../../src/application/appview_tests.rs) for
context/texture reuse, CPU state, pending presentation, and headless pixel evidence.

Headless regressions in `tests/rendering_correctness_tests.rs` validate independent
factory formats and existing-view rebinding across BGRA/RGBA formats and two devices.

## Coordinate Systems

### Four Coordinate Systems

1. **World**: 3D world coordinates (millimeters from volume origin)
2. **Screen**: 2D screen coordinates (pixels, origin at top-left)
3. **Voxel**: 3D voxel indices (integer indices into volume array)
4. **Base**: Named basis/transform stored in `glam::Mat4`

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
- Voxel ↔ Base: Use the volume's basis and spacing; do not assume an identity or one-to-one physical mapping

## Volume Rendering

The authoritative [signed voxel encoding contract](../rendering/volume-encoding.md)
describes exact packed-i16 RG8 storage, half-float rounding, affine decode/filtering,
and the separation from window/level. `src/data/volume_encoding.rs` owns the byte representation and format mapping;
`Graphics` snapshots capabilities for its selected device. A volume format is
usable only when the enabled device supports `TEXTURE_BINDING | COPY_DST` and
`FILTERABLE`. R16Float is preferred when usable; otherwise exact packed RG8 is
selected. A device without either usable format is rejected. Do not infer support
from backend identity or one feature bit. See the encoding guide for per-axis
3D texture limits and pre-allocation upload validation.

## View System

### View Trait

The current [`View` trait](../../src/rendering/view/view.rs) includes rendering,
layout, GPU rebinding, state, and invalidation hooks. Read that interface instead
of copying a simplified trait declaration into new code.

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
- Uses device-owned filtered-volume capability and texture-limit decisions; validate uploads before texture allocation

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
    └─ Present (wgpu::Queue::present after submission)
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

`RenderApp`'s owned `ApplicationHandler` translates events into the private
[viewer lifecycle operation](../../src/application/viewer_lifecycle.rs), which orders
complete frames and scheduling effects in `about_to_wait`.
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

### Browser backend selection

The WASM build enables wgpu 30's `webgpu` and `webgl` backends. With no query
selector, initialization prefers browser WebGPU and makes one WebGL2 attempt if
WebGPU detection or pre-canvas adapter/device/capability negotiation fails. A
fresh page load with `?kepler_wgpu_backend=gl` selects WebGL2 directly for
compatibility diagnosis. Unknown selector values warn and use automatic selection.

Selection and capability checks use a disposable candidate canvas before the
application canvas is bound. Once the real Winit canvas has acquired a context,
surface/device/pipeline errors are reported with backend/stage context; do not
attempt another API on that canvas. A forced-GL diagnostic requires a fresh page
load. The selected device owns the volume format and 3D-dimension capability
snapshot used for upload validation. See the [manual browser procedure](../testing/browser_gpu.md);
a successful WASM build does not prove runtime selection or rendering parity.

## Rendering-error and lifecycle migration

See [wgpu 30 / Winit 0.30 caller migration](../rendering/wgpu30-migration.md) for
`KeplerResult<()>`, owned surface failure classifications, separate pass-context
lifetimes, queue presentation, validation guards, surface/device recovery limits,
and the single GPU-ready bootstrap bridge. Known hidden/suspended/zero-size viewers
wait; an acquisition-only occlusion uses a bounded retry probe. Fatal GPU callbacks
wake the idle owner through the existing quit transport.

## Debugging Rendering

### Common Issues

**Problem**: Surface errors on startup
```bash
# Check backend selection
KEPLER_WGPU_BACKEND=vulkan cargo run --locked

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
- **Current architecture**: [Ownership map](../architecture/ARCHITECTURE.md)
- **Verification**: [Headless rendering checks](../testing/headless_gpu.md)
- **Historical designs**: `doc/rendering/unified-render-architecture.md` (not current policy)

## Release paths retained during replay

`MeshView` renders DVR and retains separate spine/needle MultiMeshContext owners.
MPR retains segmentation and mesh overlays, needle/ball markers, aliasing, and
oblique controls. Dirty tracking includes their visual state. Legacy format and
texture-pool helpers retain their public signatures; production owners pass
explicit formats and use the bounded persistent depth cache.
