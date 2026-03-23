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
- Use `PipelineManager` in `rendering/core/pipeline.rs` for pipeline lifecycle
- Always check device capabilities before creating pipelines
- Use `pollster` to block on async operations in native code

### Pipeline Recreation

Recreate pipelines when:
- Surface format changes (after window resize)
- Shader code changes
- Bind group layouts change

```rust
// Example: Update surface format after resize
PassExecutor::update_surface_format(&new_format);

// Recreate DefaultViewFactory after graphics swap
view_factory = DefaultViewFactory::new(graphics.clone());
```

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

Use per-frame texture pools to avoid fragmentation:
- `MeshTexturePool` for 3D mesh rendering
- Clear pools between frames or when swapping contexts

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
- Resize events trigger pipeline recreation

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
