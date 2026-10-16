# Mesh 3-Axis Rotation Functionality

## Overview

The medical imaging framework now supports full 3-axis rotation for 3D mesh objects using quaternions. This feature enables smooth animation and intuitive mouse-based rotation (yaw/pitch) without gimbal lock, providing enhanced visualization capabilities for medical data.

## Architecture

### Core Components

1. **MeshView Rotation State**
   - `rotation_enabled: bool` - Controls whether auto-rotation is active
   - `rotation_quat: Quat` - Current rotation state as a quaternion
   - `rotation_speed: f32` - Auto-rotation speed in radians per second
   - `last_frame_time: Instant` - Frame timing for smooth animation

2. **Quaternion-Based Rotation**
   - Uses `glam::Quat` for robust 3D rotation representation
   - Avoids gimbal lock issues associated with Euler angles
   - Allows accumulating rotations from multiple sources (mouse, auto-rotation)

3. **Frame-Rate Independent Animation**
   - Uses `Instant::now()` for precise timing calculations
   - Calculates `delta_time` between frames
   - Updates rotation quaternion based on speed and elapsed time (around Y axis)

4. **Matrix Transformation**
   - Converts quaternion to rotation matrix (`Mat4::from_quat`)
   - Combines scale and rotation in the model transformation

## API Reference

### MeshView Methods

```rust
// Enable/disable auto-rotation
pub fn set_rotation_enabled(&mut self, enabled: bool)
pub fn is_rotation_enabled(&self) -> bool

// Speed control (radians per second)
pub fn set_rotation_speed(&mut self, speed_rad_per_sec: f32)
pub fn get_rotation_speed(&self) -> f32
pub fn set_rotation_speed_degrees(&mut self, degrees_per_sec: f32)

// Rotation control
pub fn reset_rotation(&mut self) // Resets to identity
pub fn get_rotation_quat(&self) -> Quat
pub fn rotate_by_mouse(&mut self, dx: f32, dy: f32) // Accumulate 2D mouse input
pub fn set_rotation_angle_degrees(&mut self, degrees: [f32; 3]) // Set specific Euler angles
```

### State Methods (External Control)

```rust
// High-level control through State struct
pub fn set_mesh_rotation_enabled(&mut self, enabled: bool)
pub fn set_mesh_rotation_speed(&mut self, speed_rad_per_sec: f32)
pub fn set_mesh_rotation_speed_degrees(&mut self, degrees_per_sec: f32)
pub fn reset_mesh_rotation(&mut self)
pub fn set_mesh_rotation_delta(&mut self, dx: f32, dy: f32)
```

## Usage Examples

### Mouse Rotation Control

```rust
// Rotate mesh based on mouse drag
// dx: horizontal movement (yaw), dy: vertical movement (pitch)
state.set_mesh_rotation_delta(delta_x, delta_y);
```

### Basic Auto-Rotation Control

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
```

## Implementation Details

### Mouse Rotation Logic

The mouse rotation uses an arcball-like approach to map 2D screen movement to 3D rotation:

```rust
pub fn rotate_by_mouse(&mut self, dx: f32, dy: f32) {
    let sensitivity = 0.005; 
    
    // Horizontal move -> Yaw around Global Y
    let rot_y = Quat::from_rotation_y(dx * sensitivity);
    
    // Vertical move -> Pitch around Local X
    let rot_x = Quat::from_rotation_x(dy * sensitivity);
    
    // Apply rotations: Global Y * Current * Local X
    self.rotation_quat = rot_y * self.rotation_quat * rot_x;
    self.rotation_quat = self.rotation_quat.normalize();
}
```

### Auto-Rotation Logic

Auto-rotation is applied around the global Y-axis:

```rust
if self.rotation_enabled {
    let angle_delta = self.rotation_speed * delta_time;
    let rot_delta = Quat::from_rotation_y(angle_delta);
    self.rotation_quat = rot_delta * self.rotation_quat;
    self.rotation_quat = self.rotation_quat.normalize();
}
```

## Integration with Medical Imaging

### Coordinate System

- **Y-axis**: Vertical axis (up/down in medical imaging)
- **Rotation Direction**: Counter-clockwise when viewed from above (standard Y-up convention)
- **Medical Context**: Suitable for viewing anatomical structures from different angles

### Accuracy Considerations

- **Orthogonal Projection**: Maintains medical imaging accuracy (no perspective distortion)
- **Numerical Stability**: Uses normalized quaternions to prevent drift
- **Consistent Scaling**: Rotation doesn't affect object scale or positioning

## Logging and Debugging

### Log Levels

- **INFO**: Rotation enable/disable, speed changes
- **DEBUG**: Rotation angle resets, mouse rotation deltas
- **TRACE**: Per-frame rotation updates (when trace-logging feature is enabled)
