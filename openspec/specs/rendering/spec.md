## ADDED Requirements

### Requirement: Rendering Testing
The rendering subsystem SHALL have comprehensive test coverage for GPU operations, view management, and visual correctness to ensure reliable medical imaging visualization.

#### Scenario: GPU Pipeline Creation Validation
- **WHEN** a WGPU device or pipeline is created
- **THEN** it SHALL be validated for success
- **AND** creation failures SHALL return clear, actionable errors
- **AND** this SHALL only apply to native builds (#[cfg(not(target_arch = "wasm32"))])

#### Scenario: Texture Management Validation
- **WHEN** textures are created, updated, or destroyed
- **THEN** operations SHALL be tested for correct behavior
- **AND** dimension mismatches SHALL be caught
- **AND** GPU memory SHALL be properly cleaned up on destruction
- **AND** format conversions SHALL preserve data integrity

#### Scenario: View Manager State Consistency
- **WHEN** views are added, removed, or switched
- **THEN** view manager SHALL maintain consistent internal state
- **AND** concurrent operations SHALL be thread-safe
- **AND** removed views SHALL be fully cleaned up (no memory leaks)
- **AND** view counts SHALL always be accurate
- **AND** active view switching SHALL work without state loss

#### Scenario: Visual Correctness Validation
- **WHEN** window/level settings are applied
- **THEN** output SHALL preserve visual fidelity
- **AND** clamping SHALL prevent invalid values
- **AND** aspect ratios SHALL be preserved (no stretching)
- **AND** extreme values SHALL be handled gracefully

#### Scenario: GPU Error Handling
- **WHEN** GPU operations fail (allocation, shader compilation, surface creation)
- **THEN** errors SHALL be properly propagated to user
- **AND** error messages SHALL include relevant details (dimensions, memory limits)
- **AND** no GPU resources SHALL be leaked after errors
- **AND** system SHALL NOT crash or enter undefined state

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
#### Scenario: Performance Benchmarks
- **WHEN** critical operations are executed (DICOM parsing, volume rendering, frame timing)
- **THEN** performance SHALL be measured against defined baselines
- **AND** DICOM parsing SHALL complete in < 10ms for 512×512 images
- **AND** volume rendering SHALL achieve ≥ 60 FPS (16ms per frame) for 512×512 views
- **AND** memory usage SHALL be tracked and bounded (< 2GB for typical volumes)
- **AND** regressions SHALL be detected (>50% performance degradation)
