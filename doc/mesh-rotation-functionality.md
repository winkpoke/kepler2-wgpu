# Mesh Y-Axis Rotation Functionality

## Overview

The medical imaging framework now supports continuous Y-axis rotation for 3D mesh objects. This feature enables smooth animation of mesh objects around the vertical axis (Y-axis) at configurable speeds, providing enhanced visualization capabilities for medical data.

## Architecture

### Core Components

1. **MeshView Rotation State**
   - `rotation_enabled: bool` - Controls whether rotation is active
   - `rotation_angle: f32` - Current rotation angle in radians
   - `rotation_speed: f32` - Rotation speed in radians per second
   - `last_frame_time: Instant` - Frame timing for smooth animation

2. **Frame-Rate Independent Animation**
   - Uses `Instant::now()` for precise timing calculations
   - Calculates `delta_time` between frames
   - Updates rotation angle based on speed and elapsed time
   - Prevents precision issues by keeping angle within [0, 2π]

3. **Matrix Transformation**
   - Applies Y-axis rotation matrix to the model transformation
   - Combines scale and rotation when rotation is enabled
   - Falls back to scaled identity matrix when rotation is disabled

## API Reference

### MeshView Methods

```rust
// Enable/disable rotation
pub fn set_rotation_enabled(&mut self, enabled: bool)
pub fn is_rotation_enabled(&self) -> bool

// Speed control (radians per second)
pub fn set_rotation_speed(&mut self, speed_rad_per_sec: f32)
pub fn get_rotation_speed(&self) -> f32
pub fn set_rotation_speed_degrees(&mut self, degrees_per_sec: f32)

// Angle control
pub fn reset_rotation(&mut self)
pub fn get_rotation_angle(&self) -> f32
```

### State Methods (External Control)

```rust
// High-level control through State struct
pub fn set_mesh_rotation_enabled(&mut self, enabled: bool)
pub fn set_mesh_rotation_speed(&mut self, speed_rad_per_sec: f32)
pub fn set_mesh_rotation_speed_degrees(&mut self, degrees_per_sec: f32)
pub fn reset_mesh_rotation(&mut self)
pub fn is_mesh_rotation_enabled(&self) -> bool
pub fn get_mesh_rotation_speed(&self) -> f32
```

## Usage Examples

### Basic Rotation Control

```rust
// Rotation is enabled by default at 90°/s
// To disable rotation:
state.set_mesh_rotation_enabled(false);

// To re-enable rotation:
state.set_mesh_rotation_enabled(true);

// Set custom rotation speed (45 degrees per second)
state.set_mesh_rotation_speed_degrees(45.0);

// Reset to initial orientation
state.reset_mesh_rotation();

// Disable rotation
state.set_mesh_rotation_enabled(false);
```

### Direct MeshView Control

```rust
// Access MeshView directly for fine-grained control
if let Some(mesh_view) = get_mesh_view() {
    // Enable rotation at π/4 rad/s (45°/s)
    mesh_view.set_rotation_speed(std::f32::consts::PI / 4.0);
    mesh_view.set_rotation_enabled(true);
    
    // Check current state
    let current_angle = mesh_view.get_rotation_angle();
    let is_rotating = mesh_view.is_rotation_enabled();
}
```

### Common Speed Values

```rust
use std::f32::consts::PI;

// Slow rotation: 30°/s
state.set_mesh_rotation_speed(PI / 6.0);

// Medium rotation: 90°/s (default)
state.set_mesh_rotation_speed(PI / 2.0);

// Fast rotation: 180°/s
state.set_mesh_rotation_speed(PI);

// Full rotation: 360°/s
state.set_mesh_rotation_speed(2.0 * PI);
```

## Implementation Details

### Rotation Matrix Calculation

The Y-axis rotation is implemented using a standard 3D rotation matrix:

```rust
let cos_y = self.rotation_angle.cos();
let sin_y = self.rotation_angle.sin();

let rotation_y = Mat4::from_cols(
    Vec4::new(cos_y, 0.0, sin_y, 0.0),
    Vec4::new(0.0, 1.0, 0.0, 0.0),
    Vec4::new(-sin_y, 0.0, cos_y, 0.0),
    Vec4::new(0.0, 0.0, 0.0, 1.0),
);

let model_matrix = rotation_y * scale_matrix;
```

### Frame Timing

```rust
let current_time = Instant::now();
let delta_time = current_time.duration_since(self.last_frame_time).as_secs_f32();
self.last_frame_time = current_time;

if self.rotation_enabled {
    self.rotation_angle += self.rotation_speed * delta_time;
    self.rotation_angle %= 2.0 * PI; // Keep within [0, 2π]
}
```

### Performance Considerations

1. **Efficient Matrix Operations**: Uses glam library for optimized matrix calculations
2. **Conditional Updates**: Only calculates rotation when enabled
3. **Precision Management**: Keeps rotation angle within bounds to prevent floating-point drift
4. **Frame-Rate Independence**: Smooth animation regardless of frame rate

## Integration with Medical Imaging

### Coordinate System

- **Y-axis**: Vertical axis (up/down in medical imaging)
- **Rotation Direction**: Counter-clockwise when viewed from above (standard Y-up convention)
- **Medical Context**: Suitable for viewing anatomical structures from different angles

### Accuracy Considerations

- **Orthogonal Projection**: Maintains medical imaging accuracy (no perspective distortion)
- **Numerical Stability**: Uses double-precision timing and single-precision graphics
- **Consistent Scaling**: Rotation doesn't affect object scale or positioning

## Logging and Debugging

### Log Levels

- **INFO**: Rotation enable/disable, speed changes
- **DEBUG**: Rotation angle resets
- **TRACE**: Per-frame rotation updates (when trace-logging feature is enabled)

### Example Log Output

```
[INFO] Mesh Y-axis rotation enabled at 90.0°/s
[INFO] Mesh rotation speed set to 1.571 rad/s (90.0°/s)
[DEBUG] Mesh rotation angle reset to 0°
[INFO] Mesh rotation disabled
```

## Future Enhancements

### Potential Extensions

1. **Multi-Axis Rotation**: Support for X and Z axis rotation
2. **Rotation Interpolation**: Smooth transitions between rotation states
3. **Synchronized Rotation**: Multiple objects rotating in coordination
4. **Rotation Presets**: Common medical viewing angles
5. **User Input Integration**: Mouse/keyboard control for rotation speed

### Performance Optimizations

1. **SIMD Operations**: Vectorized matrix calculations
2. **Rotation Caching**: Pre-computed rotation matrices for common angles
3. **Adaptive Quality**: Reduce rotation precision during high load

## Changelog Entry

Added Y-axis rotation functionality for 3D mesh objects:
- Frame-rate independent rotation animation
- Configurable rotation speed (radians or degrees per second)
- External control through State struct methods
- Maintains medical imaging accuracy with orthogonal projection
- Default rotation speed: 90 degrees per second
- Comprehensive logging and error handling