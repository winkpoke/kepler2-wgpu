# Changelog

All notable changes to the Kepler WGPU Medical Imaging Framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- **Uniform Color Cube for Better Lighting Visualization**: Updated `uniform_color_cube()` function to use the same neutral gray color for all faces instead of different colors per face
  - **Improved Lighting Isolation**: All faces now use uniform gray color `[0.7, 0.7, 0.7]` to better isolate lighting effects
  - **Enhanced Debugging**: Easier to distinguish between lighting-induced brightness variations and base color differences
  - **Medical Imaging Accuracy**: More appropriate for medical contexts where uniform material properties are common
  - **Consistent Base Color**: All 24 vertices use the same color, making lighting effects more apparent
  - Documented in `doc/redering/2025-01-12T11-45-00Z-uniform-color-cube-same-color-update.md`

- **MIP View Blank Output**: Initialize and upload MIP uniforms each frame in `MipView::update`, providing valid camera vectors, volume parameters, and window/level defaults for MVP. This resolves the blank MIP view by ensuring the fragment shader receives non-zero parameters.
  - Camera set in normalized volume space (pos (0.5,0.5,-0.5), front (0,0,1), up (0,1,0), right (1,0,0))
  - Format-aware window/level defaults (RG8: window=4096, level=2048; Float: window=1.0, level=0.5)
  - Identity view_matrix to avoid unintended perspective transforms
  - Documented in `doc/2025-10-11T00-00-00Z-mip-uniforms-initialization-fix.md`

### Added
- **Minimal Lighting Integration for Basic Mesh Rendering**: Implemented basic lighting support for 3D mesh visualization with Lambert diffuse and ambient lighting
  - **Extended Vertex Structure**: Added normal vectors to `MeshVertex` struct for per-vertex lighting calculations
  - **Basic Lighting Uniforms**: Created `BasicLightingUniforms` structure with directional light support:
    - Configurable light direction, color, and intensity
    - Ambient lighting with color and intensity controls
    - Default lighting setup with white directional light from top-left-front
  - **Enhanced Shader System**: Updated `mesh_basic.wgsl` with lighting calculations:
    - Lambert diffuse lighting model for realistic surface shading
    - Ambient lighting for base illumination in shadowed areas
    - Per-vertex normal interpolation for smooth lighting transitions
  - **Pipeline Integration**: Added `create_basic_mesh_pipeline_with_lighting()` function:
    - Dual bind group support for transform and lighting uniforms
    - Dedicated lighting bind group layout for fragment shader visibility
    - Maintains compatibility with existing basic mesh pipeline
  - **Normal Vector Generation**: Enhanced cube mesh generation with proper face normals:
    - Accurate normal vectors for each cube face (front, back, top, bottom, left, right)
    - Consistent winding order for proper lighting calculations
  - **Cross-Platform Compatibility**: Verified functionality for both native and WebAssembly builds
  - **Performance Optimized**: Minimal overhead with efficient GPU-based lighting calculations
  - Created comprehensive documentation in `doc/2025-10-12T10-08-29-minimal-lighting-integration-plan.md`

- **MIP View Bottom-Right Integration**: Integrated MIP (Maximum Intensity Projection) view in the bottom-right corner of the 2x2 grid layout
  - **Automatic Positioning**: MIP view positioned at grid index 3 (bottom-right) using existing GridLayout strategy
  - **Responsive Design**: Automatic resizing and positioning across different screen sizes and platforms
  - **Dual Mode Support**: MIP view available in both mesh-enabled and mesh-disabled modes
  - **Resource Efficiency**: Shares texture resources with MPR views for optimal memory usage
  - **Cross-Platform Compatibility**: Verified functionality for both native and WebAssembly builds
  - **Minimal Code Changes**: Implementation achieved with focused modifications to `load_data_from_ct_volume()` method
  - Created comprehensive documentation in `doc/2025-01-11T16-30-00Z-mip-view-bottom-right-integration.md`

### Added
- **MIP (Maximum Intensity Projection) MVP Implementation**: Complete foundation for 3D volume visualization using maximum intensity projection
  - **Core Data Structures**: `MipConfig`, `MipView`, and `MipRenderContext` with essential fields for ray marching
  - **Essential Shaders**: Vertex shader for full-screen quad and fragment shader with ray marching implementation
  - **Pipeline Integration**: Basic integration with existing render pass system and `PassExecutor` framework
  - **View System Integration**: Full implementation of `View` and `Renderable` traits for seamless integration
  - **Memory Efficiency**: Reuses existing `RenderContent` through Arc sharing to avoid texture duplication
  - **Comprehensive Testing**: 6 test cases covering configuration, view creation, trait implementation, and positioning
  - **Architecture Compliance**: Follows established patterns from MPR and Mesh rendering systems
  - Created detailed documentation in `doc/2025-01-11T20-30-00Z-mip-mvp-completion-report.md`

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