# Changelog

All notable changes to the Kepler WGPU Medical Imaging Framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **MHD Dual-File Processing in WASM**: Implemented comprehensive support for processing MHD (MetaImage) files consisting of separate header (.mhd) and data (.raw/.zraw) files in WASM environments
  - Rewrote `process_single_mhd_file()` to use `MhdParser::parse_by_bytes(&header_bytes, &data_bytes)` for proper dual-file handling
  - Enhanced `parse_common_files_wasm()` with intelligent file matching by name and extension (case-insensitive)
  - Added comprehensive file validation (extensions, sizes, content) with reasonable limits (1MB for headers, 2GB for data)
  - Implemented fallback matching strategy when exact name matches aren't found
  - Added robust error handling for missing data files, mismatched pairs, and processing failures
  - Used modern async/await patterns with `gloo_timers` for improved readability and timeout handling
  - Created detailed documentation in `doc/mhd-dual-file-wasm.md`

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