# Rendering Specification

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
