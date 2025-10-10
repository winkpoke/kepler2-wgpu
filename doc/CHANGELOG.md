# Changelog

All notable changes to the Kepler WGPU Medical Imaging Framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **View Layout Refactoring and ViewManager Module**: Comprehensive refactoring of view management system for improved maintainability and functionality
  - **Enhanced Layout Module**: Added view replacement and management methods to `Layout` struct:
    - `replace_view_at()`: Replace view at specific index with proper bounds checking
    - `get_view_mut()`: Mutable access to views with bounds validation
    - `is_view_type()`: Type checking for views at specific positions
    - `toggle_view_type_at()`: Toggle between different view types at specified index
    - `view_count()`: Get total number of views in layout
  - **ViewManager Module**: New centralized view transition management system:
    - State preservation and restoration for view transitions
    - Factory pattern integration for consistent view creation
    - Comprehensive error handling with detailed logging
    - Support for saving/restoring MPR view states (window level, width, slice position, scale, translation)
    - Utility methods for state management (`clear_states`, `saved_state_count`, `has_saved_state`)
  - **State Refactoring**: Major simplification of `set_mesh_mode_enabled` function:
    - Extracted 10 focused helper methods for improved code organization
    - Enhanced error handling with proper logging and early returns
    - Improved type safety and compilation reliability
    - Better separation of concerns between mesh and MPR view handling
  - **Enhanced View Traits**: Extended view system with new capabilities:
    - `StatefulView` trait for state preservation across view transitions
    - Enhanced `View` trait with downcasting support (`as_any`, `as_any_mut`)
    - Improved factory pattern with consistent error handling

### Added
- **Orthogonal Projection for Medical Visualization**: Implemented orthogonal projection as the default projection type for 3D mesh rendering to ensure accurate dimensional representation without perspective distortion
  - Added `ProjectionType` enum with `Perspective` and `Orthogonal` variants
  - Enhanced `Camera` struct with orthogonal projection parameters (`ortho_left`, `ortho_right`, `ortho_bottom`, `ortho_top`)
  - Implemented orthogonal projection matrix calculation with automatic aspect ratio handling
  - Added `Camera::new_perspective()` method for backward compatibility
  - Updated default uniforms in `MeshRenderContext` to use orthogonal projection
  - Created comprehensive documentation in `doc/orthogonal-projection-implementation.md`

- **Buffer Lifecycle Management**: Implemented comprehensive buffer cleanup to prevent "Buffer does not exist" errors during mesh mode toggling
  - Added `Drop` implementations for `BasicMeshContext` and `MeshView` with debug logging
  - Added `clear_mesh_context_cache()` method to `State` for explicit cache clearing
  - Added `clear_depth_view()` method to `TexturePool` for depth texture cleanup
  - Enhanced graphics context swapping to clear mesh resources and prevent stale references
  - Created detailed documentation in `doc/buffer-lifecycle-fix.md`

- **Mesh Y-Axis Rotation Animation**: Added continuous Y-axis rotation functionality for 3D mesh objects
  - Frame-rate independent rotation animation using precise timing calculations
  - Configurable rotation speed in radians or degrees per second (default: 90°/s)
  - **Enabled by default** for immediate visual feedback
  - External control through `State` struct methods (`set_mesh_rotation_enabled`, `set_mesh_rotation_speed`, etc.)
  - Direct control through `MeshView` methods for fine-grained manipulation
  - Automatic angle normalization to prevent floating-point precision issues
  - Maintains medical imaging accuracy with orthogonal projection
  - Comprehensive logging at INFO, DEBUG, and TRACE levels
  - Created detailed documentation in `doc/mesh-rotation-functionality.md`

### Changed
- **Default Camera Projection**: `Camera::new()` now defaults to orthogonal projection instead of perspective projection for medical accuracy
- **Fallback Rendering**: Default uniform calculations now use orthogonal projection for consistency

### Technical Details
- Orthogonal projection maintains object size regardless of distance from viewer
- Automatic aspect ratio preservation prevents distortion
- Cross-platform compatibility verified (native and WASM builds)
- Backward compatibility maintained for existing perspective projection use cases
- Performance optimized with simpler matrix calculations compared to perspective projection

### Documentation
- Added detailed implementation guide for orthogonal projection
- Updated function-level comments to reflect orthogonal projection usage
- Documented benefits for medical visualization applications

---

## Template for Future Releases

### [Version] - YYYY-MM-DD

#### Added
- New features

#### Changed
- Changes in existing functionality

#### Deprecated
- Soon-to-be removed features

#### Removed
- Now removed features

#### Fixed
- Bug fixes

#### Security
- Vulnerability fixes