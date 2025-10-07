# Unified Architecture Design

## Overview

This document consolidates the design specifications for the kepler-wgpu rendering system, integrating 3D mesh rendering, render pass architecture, pipeline management, and mesh view implementation into a unified design. The architecture supports both 2D medical imaging (MPR views) and 3D mesh visualization with a modular, feature-gated approach.

## System Architecture

### Core Design Principles

1. **Feature-Gated Modularity**: 3D mesh functionality is conditionally compiled via the `mesh` Cargo feature
2. **Memory Safety**: Leverages Rust's ownership system and WGPU's resource management
3. **Cross-Platform Compatibility**: Supports native (Windows, macOS, Linux) and WebAssembly targets
4. **Performance Optimization**: Implements pipeline caching, texture pooling, and efficient render pass organization
5. **Separation of Concerns**: Clear boundaries between 2D slice rendering and 3D mesh rendering

### High-Level Components

```
┌─────────────────────────────────────────────────────────────────┐
│                        Application Layer                        │
├─────────────────────────────────────────────────────────────────┤
│  RenderApp  │  State Management  │  Layout System  │  Views     │
├─────────────────────────────────────────────────────────────────┤
│                      Render Architecture                        │
│  PassExecutor │ PassPlan │ PipelineManager │ PassRegistry*      │
├─────────────────────────────────────────────────────────────────┤
│                    Resource Management                          │
│  TexturePool │ RenderContent │ BufferArena* │ BindGroupCache*   │
├─────────────────────────────────────────────────────────────────┤
│                      View System                                │
│  GenericMPRView (2D) │ MeshView (3D) │ RenderContext           │
├─────────────────────────────────────────────────────────────────┤
│                      WGPU Foundation                            │
│  Device │ Queue │ Surface │ CommandEncoder │ RenderPass         │
└─────────────────────────────────────────────────────────────────┘

* Components marked with asterisk are partially implemented or planned
```

## Render Pass Architecture

### Pass Types and Execution Order

The system implements a multi-pass rendering architecture with the following execution order:

1. **MeshPass** (3D rendering with depth testing)
   - Renders 3D mesh geometry with depth buffering
   - Uses offscreen color and depth attachments
   - Supports camera transformations, lighting, and materials
   - Only executed when mesh feature is enabled and MeshView is present

2. **SlicePass** (2D MPR rendering)
   - Renders medical imaging slices (axial, coronal, sagittal)
   - Uses texture sampling for volume data visualization
   - Operates without depth testing for maximum throughput
   - Renders to offscreen targets or directly to swapchain

3. **OverlayPass** (UI and annotations)
   - Renders overlays, annotations, and UI elements
   - Can optionally read depth buffer from MeshPass
   - Typically renders directly to swapchain

### PassExecutor Implementation

```rust
// Current implementation in src/render_pass.rs
pub enum PassId {
    MeshPass,
    SlicePass,
}

impl PassExecutor {
    pub fn execute_frame(&mut self, 
                        encoder: &mut wgpu::CommandEncoder,
                        target_view: &wgpu::TextureView,
                        texture_pool: &mut TexturePool) -> Result<(), wgpu::SurfaceError> {
        // Execute passes based on available views and features
        if cfg!(feature = "mesh") && has_mesh_view {
            self.execute_mesh_pass(encoder, texture_pool)?;
        }
        self.execute_slice_pass(encoder, target_view, texture_pool)?;
        Ok(())
    }
}
```

### Resource Management

#### TexturePool

Manages offscreen render targets and depth attachments:

```rust
// Implementation in src/mesh/texture_pool.rs
pub struct TexturePool {
    depth_texture: Option<wgpu::Texture>,
    depth_view: Option<wgpu::TextureView>,
    color_views: HashMap<String, wgpu::TextureView>,
    color_textures: HashMap<String, wgpu::Texture>,
}
```

**Key Features:**
- Lazy allocation of depth textures (Depth32Float format)
- Caching of offscreen color targets by pass ID
- Automatic resource cleanup and reuse
- Size-based texture pooling for different viewport dimensions

#### PipelineManager

Centralizes render pipeline creation and caching:

```rust
// Implementation in src/pipeline.rs
pub struct PipelineManager {
    cache: HashMap<PipelineKey, Arc<wgpu::RenderPipeline>>,
    device: Arc<wgpu::Device>,
}
```

**Pipeline Types:**
- **TextureQuad Pipeline**: For 2D slice rendering with texture sampling
- **MeshBasic Pipeline**: For 3D mesh rendering with vertex/fragment shaders
- **MeshDepth Pipeline**: For depth-only rendering passes

**Caching Strategy:**
- Pipelines keyed by shader, vertex layout, and render state
- Arc-wrapped pipelines for safe sharing across render contexts
- Automatic cache invalidation on device loss

## 3D Mesh System

### Mesh Data Structures

#### MeshVertex

```rust
// Implementation in src/mesh/mesh.rs
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}
```

**Vertex Attributes:**
- Position: 3D world-space coordinates
- Normal: Surface normal for lighting calculations
- UV: Texture coordinates for material mapping

#### Mesh

```rust
#[derive(Default, Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
}
```

**Features:**
- Indexed triangle rendering for efficiency
- Support for arbitrary mesh topology
- Built-in unit cube generation for testing
- Optimized for GPU upload via bytemuck

### Rendering Components

#### Camera System

```rust
// Implementation in src/mesh/camera.rs
#[derive(Default, Debug, Clone)]
pub struct Camera {
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}
```

**Capabilities:**
- Perspective projection with configurable FOV
- View matrix generation from position/target/up
- Aspect ratio handling for viewport changes
- Near/far plane configuration for depth precision

#### Lighting System

```rust
// Implementation in src/mesh/lighting.rs
#[derive(Default, Debug, Clone)]
pub struct Lighting {
    pub ambient_color: [f32; 3],
    pub ambient_strength: f32,
    pub light_position: [f32; 3],
    pub light_color: [f32; 3],
}
```

**Lighting Model:**
- Ambient lighting with configurable strength
- Single directional/point light source
- Phong-style lighting calculations in fragment shader
- Extensible for multiple light sources

#### Material System

```rust
// Implementation in src/mesh/material.rs
#[derive(Default, Debug, Clone)]
pub struct Material {
    pub diffuse_color: [f32; 3],
    pub specular_color: [f32; 3],
    pub shininess: f32,
}
```

**Material Properties:**
- Diffuse color for base surface appearance
- Specular highlights with configurable shininess
- Support for texture-based materials (future extension)

### MeshView Integration

#### View Implementation

```rust
// Implementation in src/mesh/mesh_view.rs
pub struct MeshView {
    pub mesh: Option<Mesh>,
    pub material: Option<Material>,
    pub camera: Option<Camera>,
    pub lighting: Option<Lighting>,
    ctx: Option<Arc<MeshRenderContext>>,
    pos: (i32, i32),
    dim: (u32, u32),
}
```

**Integration Points:**
- Implements `Renderable` trait for uniform view handling
- Manages viewport positioning and sizing
- Shares `MeshRenderContext` via Arc for efficient resource usage
- Conditional compilation via `cfg(feature = "mesh")`

#### MeshRenderContext

```rust
// Implementation in src/mesh/mesh_render_context.rs
pub struct MeshRenderContext {
    pub pipeline: Arc<wgpu::RenderPipeline>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_vertices: u32,
    pub num_indices: u32,
}
```

**Responsibilities:**
- GPU resource management for mesh rendering
- Pipeline acquisition from PipelineManager
- Vertex/index buffer creation and upload
- Render command encoding

## Shader System

### Shader Architecture

The system uses WGSL shaders with a modular approach:

#### 2D Slice Shaders
- **shader_tex.wgsl**: Vertex/fragment shaders for texture quad rendering
- Supports volume texture sampling and MPR slice generation
- Optimized for medical imaging workflows

#### 3D Mesh Shaders
- **mesh.wgsl**: Primary vertex/fragment shaders for mesh rendering
- **mesh_depth.wgsl**: Depth-only variant for shadow mapping (future)
- Implements Phong lighting model with configurable materials

### Shader Compilation and Caching

```rust
// Pipeline creation with shader compilation
let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("Mesh Shader"),
    source: wgpu::ShaderSource::Wgsl(include_str!("../shader/mesh.wgsl").into()),
});
```

**Features:**
- Compile-time shader inclusion via `include_str!`
- Automatic shader validation during pipeline creation
- Cross-platform WGSL compilation (native and WASM)

## View System Integration

### Layout Management

The system supports flexible view layouts with the following slot configuration:

```
┌─────────────┬─────────────┐
│   Slot 0    │   Slot 1    │
│  (Axial)    │ (Coronal)   │
├─────────────┼─────────────┤
│   Slot 2    │   Slot 3    │
│ (Sagittal/  │ (Reserved)  │
│  MeshView)  │             │
└─────────────┴─────────────┘
```

**Slot Assignment:**
- **Slot 0**: Axial MPR view (always present)
- **Slot 1**: Coronal MPR view (always present)
- **Slot 2**: Sagittal MPR view OR MeshView (feature-dependent)
- **Slot 3**: Reserved for future use

### View Selection Logic

```rust
// Implementation in src/state.rs
if cfg!(feature = "mesh") && enable_mesh_view {
    // Create MeshView for slot 2
    let mut mesh_view = MeshView::new();
    mesh_view.attach_context(mesh_render_context);
    layout.views[2] = Box::new(mesh_view);
} else {
    // Create Sagittal MPR view for slot 2
    let sagittal_view = GenericMPRView::new(Orientation::Sagittal);
    layout.views[2] = Box::new(sagittal_view);
}
```

## Performance Optimization

### Rendering Optimizations

1. **Pipeline Caching**: Expensive pipeline creation is cached and reused
2. **Texture Pooling**: Offscreen textures are pooled to avoid allocation overhead
3. **Batch Rendering**: Multiple objects can share the same pipeline and resources
4. **Viewport Culling**: Only render views that are visible in the current layout

### Memory Management

1. **Resource Lifetime**: Clear ownership of GPU resources via Rust's type system
2. **Buffer Reuse**: Vertex and index buffers are reused across frames
3. **Texture Compression**: Support for compressed texture formats where available
4. **Garbage Collection**: Automatic cleanup of unused resources

### Cross-Platform Considerations

1. **WASM Compatibility**: All shaders and pipelines work in WebAssembly
2. **Feature Detection**: Graceful fallback for unsupported GPU features
3. **Format Support**: Automatic selection of supported texture formats
4. **Performance Scaling**: Adaptive quality based on platform capabilities

## Implementation Status

### Completed Components

✅ **Core Infrastructure**
- Feature-gated mesh module compilation (`cfg(feature = "mesh")`)
- Basic mesh data structures (Mesh, MeshVertex with bytemuck support)
- Camera, Lighting, and Material systems (data structures only)
- MeshView and MeshRenderContext scaffolding
- Cross-platform build support (native and WASM)

✅ **Render Architecture**
- PassExecutor with MeshPass and SlicePass support
- PassPlan and PassDescriptor for render pass organization
- TexturePool for depth and color attachment management
- PipelineManager with pipeline caching and Arc-wrapped sharing
- Render pass creation and execution with proper borrowing

✅ **Pipeline Management**
- Centralized pipeline creation via PipelineManager
- Pipeline caching with string-based keys
- Global swapchain format management
- Mesh pipeline creation with depth support
- Texture quad pipeline for 2D rendering

✅ **Integration**
- View system integration with layout management
- Conditional MeshView creation in slot 2 based on feature flag
- Borrowing conflict resolution in render passes
- Proper separation of mutable and immutable texture pool access

### Partially Implemented

🔄 **Mesh Rendering Pipeline**
- Basic mesh pipeline creation (implemented but not actively used)
- Vertex buffer and index buffer management in MeshRenderContext
- Shader compilation and validation (mesh.wgsl and mesh_depth.wgsl exist)
- MeshView rendering integration (scaffolded but not fully connected)

🔄 **Advanced Render Architecture**
- PassRegistry (basic implementation exists but not fully utilized)
- Resource management beyond TexturePool (planned BufferArena, BindGroupCache)
- DrawItem abstraction for render commands (planned)

### Planned Features

📋 **Advanced Rendering**
- Active mesh rendering in MeshPass (currently disabled in state.rs)
- Multiple light sources and shadow mapping
- Texture-based materials and normal mapping
- Instanced rendering for multiple objects
- Level-of-detail (LOD) system for large meshes

📋 **User Interaction**
- Camera controls (orbit, pan, zoom)
- Mesh selection and highlighting
- Real-time parameter adjustment
- Animation and keyframe system

📋 **Performance Enhancements**
- BufferArena for efficient buffer suballocation
- BindGroupCache for reducing allocation overhead
- Frustum culling and occlusion culling
- GPU-driven rendering pipelines
- Compute shader integration
- Multi-threaded command buffer generation

### Current Limitations and Known Issues

⚠️ **Mesh Rendering Status**
- MeshView creation is currently disabled in `src/state.rs` (line 805) for debugging purposes
- Mesh rendering pipeline exists but is not actively used in the render loop
- Camera, lighting, and material systems are data-only structures without active rendering logic

⚠️ **Architecture Gaps**
- PassRegistry exists but is not fully integrated into the render loop
- BufferArena and BindGroupCache are designed but not implemented
- DrawItem abstraction for render commands is planned but not implemented
- Resource lifetime management relies on manual coordination rather than automated pooling

⚠️ **Performance Considerations**
- Pipeline caching uses string-based keys which may impact performance at scale
- No automatic cleanup policy for cached resources
- Limited resource pooling beyond basic texture management
- No multi-threaded command buffer generation

⚠️ **Testing and Validation Gaps**
- Mesh rendering functionality is not covered by automated tests
- Visual validation of 3D rendering requires manual testing
- Cross-platform compatibility of mesh features needs verification
- Performance benchmarks for mesh rendering are not established

## Error Handling and Resilience

### Error Handling Patterns

The system implements comprehensive error handling to ensure stability across different platforms and configurations:

**Pipeline Creation Resilience**
```rust
// Safe fallback on pipeline creation failures
match pipeline_manager.get_or_create_texture_quad_pipeline() {
    Ok(pipeline) => pipeline,
    Err(e) => {
        log::error!("Failed to create texture quad pipeline: {}", e);
        return; // Skip rendering for this frame
    }
}
```

**Surface Error Recovery**
- Automatic swapchain reconfiguration on `SurfaceError::Lost` or `SurfaceError::Outdated`
- Graceful degradation when GPU resources become unavailable
- Proper cleanup and reinitialization of render targets

**Mesh Rendering Error Isolation**
- Mesh rendering failures do not affect 2D slice rendering
- Feature-gated error paths prevent compilation issues when mesh feature is disabled
- Fallback to 2D-only rendering when mesh initialization fails

**Resource Management Safety**
- RAII patterns ensure automatic cleanup of GPU resources
- Reference counting (`Arc`) for shared resources prevents use-after-free
- Explicit lifetime management for temporary resources

### Logging and Diagnostics

**Structured Logging**
```rust
// Platform-specific logger initialization
#[cfg(not(target_arch = "wasm32"))]
pub fn init_logger() -> Result<(), log::SetLoggerError> {
    let mut builder = env_logger::Builder::new();
    builder
        .filter_level(LevelFilter::Info)
        .filter_module("wgpu", LevelFilter::Warn)
        .filter_module("wgpu_core", LevelFilter::Warn)
        .filter_module("wgpu_hal", LevelFilter::Warn)
        .filter_module("naga", LevelFilter::Warn);
    builder.try_init()
}
```

**Debug Features**
- Detailed pipeline creation logging always enabled at trace level
- Cache hit/miss statistics for performance monitoring
- Validation of pipeline descriptors and attachment compatibility

## Configuration Management

### Feature Flags

The system uses Cargo feature flags for conditional compilation:

```toml
[features]
# Opt-in 3D mesh feature gate
mesh = []
```

**Feature-Gated Architecture**
- Core 2D functionality always available
- 3D mesh rendering conditionally compiled with `mesh` feature
- Debug instrumentation always available at trace level

### Runtime Configuration

**Backend Selection**
- Automatic backend selection based on platform capabilities
- Preference for `PRIMARY` backend on native platforms
- Fallback to `GL` backend on WebAssembly

**Surface Format Negotiation**
- Automatic selection of optimal surface format
- Preference for sRGB formats when available
- Global format storage for consistent pipeline creation

**Quality Settings**
- Configurable window/level parameters for medical imaging
- Adaptive quality based on platform performance
- Optional MSAA support (planned)

### Initialization Patterns

**Application Startup**
1. Logger initialization with platform-specific configuration
2. WGPU instance creation with appropriate backends
3. Surface creation and format negotiation
4. Device and queue initialization
5. Pipeline manager and resource pool setup
6. View system initialization based on enabled features

**Resource Lifecycle**
- Lazy initialization of expensive resources (mesh pipelines, depth textures)
- Deterministic cleanup when features are disabled
- Automatic resource pooling for frequently used objects

## Testing and Validation

### Build Validation

The system supports multiple build configurations:

```bash
# Native build without mesh feature
cargo build

# Native build with mesh feature
cargo build --features mesh

# WASM build with mesh feature
wasm-pack build --target web --features mesh
```

### Test Coverage

1. **Unit Tests**: Core data structures and algorithms
2. **Integration Tests**: Render pass execution and resource management
3. **Visual Tests**: Manual verification of rendering output
4. **Performance Tests**: Frame time and memory usage benchmarks

## Migration and Compatibility

### Backward Compatibility

- Default build (without mesh feature) maintains existing functionality
- Existing 2D MPR views are unaffected by mesh system additions
- API changes are additive and do not break existing code

### Migration Path

1. **Phase 1**: Enable mesh feature and verify basic functionality
2. **Phase 2**: Integrate mesh rendering into existing workflows
3. **Phase 3**: Add advanced features and optimizations
4. **Phase 4**: Deprecate legacy rendering paths where appropriate

## Conclusion

This unified architecture provides a solid foundation for both 2D medical imaging and 3D mesh visualization. The modular design allows for incremental adoption of 3D features while maintaining the stability and performance of existing 2D workflows. The feature-gated approach ensures that applications can choose their complexity level based on requirements.

The architecture leverages modern Rust and WGPU best practices to deliver high-performance, memory-safe rendering across multiple platforms. Future enhancements can be added incrementally without disrupting the core system design.