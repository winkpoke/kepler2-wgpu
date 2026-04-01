# MeshView Aspect Ratio Distortion Fix

**Date**: 2026-04-01T17:26:15  
**Status**: Implemented  
**Priority**: High  

## Problem Statement

When resizing the application window, the 3D volume view (`MeshView`) would distort and deform. The objects within the 3D view would appear stretched horizontally or vertically instead of maintaining their natural aspect ratio.

## Root Cause Analysis

In the previous implementation of `MeshView::update_uniforms`, the aspect ratio of the viewport (`self.dim`) was not factored into the projection scaling.

Specifically:
- The fragment shader `mesh_volume.wgsl` uses normalized texture coordinates `in.tex_coords` spanning `[0, 1]`.
- It calculates `uv_centered = in.tex_coords - vec2<f32>(0.5, 0.5)` without accounting for the actual screen aspect ratio.
- Because `uv_centered` maps equally to the X and Y axes of the ray origin, an unequal screen dimension stretches the volume in the longer axis.

## Solution

We introduced aspect ratio compensation directly into the shader by adding the `aspect_ratio` value to the `MeshUniforms` struct.

### 1. Updated `MeshUniforms` struct
We added `aspect_ratio` and 3 floats of padding (to satisfy the WGSL 16-byte alignment requirement for `mat4x4<f32>`).

```rust
pub struct MeshUniforms {
    // ...
    pub aspect_ratio: f32,
    pub rotation: [f32; 16],
}
```

### 2. Calculated Viewport Aspect Ratio in `MeshView`
In `update_uniforms`, we calculated the `aspect_ratio` from `self.dim.0` and `self.dim.1` (width / height) and populated the struct:

```rust
let aspect_ratio = if self.dim.1 > 0 && self.dim.0 > 0 {
    self.dim.0 as f32 / self.dim.1 as f32
} else {
    1.0
};
```

### 3. Modified `mesh_volume.wgsl` Shader
In the fragment shader, we conditionally scale `uv_centered.x` or `uv_centered.y` based on `aspect_ratio` to normalize the projection bounds.

```wgsl
var uv_centered = in.tex_coords - vec2<f32>(0.5, 0.5);

// Apply aspect ratio compensation
if (u_vol.aspect_ratio > 1.0) {
    uv_centered.x *= u_vol.aspect_ratio;
} else if (u_vol.aspect_ratio < 1.0 && u_vol.aspect_ratio > 0.0) {
    uv_centered.y /= u_vol.aspect_ratio;
}
```

This effectively scales the virtual coordinate system to cover the proportional width or height, preventing the object from appearing stretched on the physical screen.