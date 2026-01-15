# rendering Specification

## Purpose
Defines the rendering subsystem architecture, ensuring a clean, streamlined mesh rendering pipeline with fully configurable lighting and minimal overhead.
## Requirements
### Requirement: Clean Architecture for Mesh Rendering
The mesh rendering subsystem MUST provide a streamlined, single-context architecture without dead code or unused abstractions.

#### Scenario: Basic Rendering
Given a simple mesh (e.g., a cube)
When the application initializes the `MeshView`
Then it should use `BasicMeshContext` exclusively
And no `MeshRenderContext` should be instantiated or compiled.

### Requirement: Configurable Lighting
The lighting system MUST be fully configurable via the `Lighting` struct without hardcoded fallbacks in the conversion logic.

#### Scenario: Custom Lighting
Given a `Lighting` configuration with red ambient light
When the uniforms are generated via `to_basic_uniforms()`
Then the output `BasicLightingUniforms` should contain the red ambient light value
And not the hardcoded default.

### Requirement: Optimized Math Operations
The rendering subsystem SHALL use the `glam` library for all vector and matrix operations to ensure performance and maintainability.

#### Scenario: MVP Calculation
- **WHEN** calculating Model-View-Projection matrices in `MeshView`
- **THEN** `glam::Mat4` operations MUST be used instead of manual array manipulation
- **AND** the resulting matrices MUST be compatible with the shader's column-major layout.

## MPR View

The `MprView` is responsible for defining the geometry and projection for Multi-Planar Reconstruction.

### State

The view state is maintained using `glam` types for performance and standard compliance.

```rust
pub struct MprView {
    pub base_screen: glam::Mat4, // Screen space transformation
    pub width: u32,
    pub height: u32,
    pub pan: glam::Vec3,
    pub scale: f32,
    pub window_level: WindowLevel,
    // ... other fields
}
```

### Transformation Logic

The transformation pipeline matches the legacy `Base` implementation logic but uses `glam`:

1.  **Scale**: Applied as `1.0 / scale_factor`.
2.  **Translate**: Applied as standard matrix translation.
3.  **Composition Order**: `Pan -> Center -> Scale -> Uncenter`.
    *   `Translate(-pan)`
    *   `Translate(0.5, 0.5, 0.0)`
    *   `Scale(scale, scale, 1.0)` (where `scale` is derived from zoom, etc.)
    *   `Translate(-0.5, -0.5, 0.0)`

### Uniform Buffer

The transform matrix is uploaded to the GPU as a `[f32; 16]` column-major array.

```rust
struct UniformsFrag {
    mat: [f32; 16],
    // ...
}
```
