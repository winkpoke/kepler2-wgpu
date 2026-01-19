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

### Requirement: MIP View Spin
The MIP view SHALL support 3D rotation via roll, yaw, and pitch inputs.

#### Scenario: User Rotates MIP View
- **WHEN** the user inputs a roll, yaw, or pitch angle
- **THEN** the MIP view updates the internal rotation state
- **AND** the rendered image reflects the rotated volume

### Requirement: MIP Shader Rotated Ray Casting
The MIP shader MUST support rotated orthographic rays by applying a rotation transform to ray generation and MUST compute correct ray-box intersection for non-axis-aligned rays.

#### Scenario: Rotated Ray Casting
- **WHEN** rotation parameters are provided via uniforms
- **THEN** the shader computes ray-box intersection for the rotated ray against the unit volume box
- **AND** the sampling coordinates remain within [0,1]^3 for valid hits
- **AND** the shader returns black output when the ray does not intersect the volume

### Requirement: MIP Uniform Layout Compatibility
The MIP uniform buffer layout MUST remain compatible between Rust and WGSL, and MUST maintain 16-byte alignment requirements on all targets, including wasm.

#### Scenario: Uniform Layout Validation
- **WHEN** the MIP uniforms are updated to include rotation parameters
- **THEN** the uniform struct size remains a multiple of 16 bytes
- **AND** runtime binding uses the full struct size without undefined padding reads

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
