# Changelog

All notable changes to the Kepler WGPU Medical Imaging Framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- 2025-11-08T22-27-07: Resolved build failure by switching `Graphics` to store `Arc<wgpu::Device>` and `Arc<wgpu::Queue>` and updating `State::new()` to initialize `DefaultViewFactory` via `Arc::clone`.
  - Eliminates invalid `clone()` calls on `wgpu` handles.
  - Preserves existing behavior; public API unchanged (fields are `pub(crate)`).
  - All existing call sites continue to work via auto-deref to `&Device`/`&Queue`.
  - Verified native build succeeds; tests mostly pass (2 integration tests fail due to missing external files).
  - Documentation: `doc/views/2025-11-08T22-27-07-default-view-factory-init-and-arc-device-queue.md`.

### Added
- 2025-11-08T22-28-40: Declared `trace-logging` feature in `Cargo.toml` to gate heavy TRACE logs as required.
  - Usage: `cargo run --features trace-logging` (native) or `wasm-pack build -t web -- --features trace-logging` (wasm).
  - Documentation: `doc/views/2025-11-08T22-28-40-trace-logging-feature-declaration.md`.

### Changed
- 2025-11-08T22-15-41: Modified `ViewFactory::create_mesh_view` to accept a mesh parameter `(&Mesh)` for caller-provided geometry.
  - New trait signature: `fn create_mesh_view(&self, mesh: &Mesh, pos: (i32, i32), size: (u32, u32)) -> Result<Box<dyn View>, Box<dyn std::error::Error>>`.
  - `DefaultViewFactory::create_mesh_view` now builds `BasicMeshContext` from the provided mesh and enables depth testing by default. Rotation remains enabled.
  - `ViewManager::create_mesh_view` preserves its existing API by constructing a default `Mesh::spine_vertebra()` internally and forwarding to the factory.
  - Test `MockViewFactory` implementations updated accordingly; targeted tests (`cargo test --test view_transition_integration_tests`) pass.
  - Rationale: aligns mesh creation with the additive design adopted for volume views (MPR/MIP), improves flexibility and performance by avoiding redundant mesh construction.

### Changed
- 2025-11-08T21-26-05: Extracted ViewFactory trait from `src/rendering/view/view.rs` into `src/rendering/view/view_factory.rs` to improve module organization and decouple factory responsibilities from core view types.
  - Public re-export added in `src/rendering/view/mod.rs` (`pub use view_factory::ViewFactory`) so existing imports continue to work.
  - Verified native build (`cargo build`), targeted view transition tests (`cargo test --test view_transition_integration_tests`), and WASM build (`wasm-pack build -t web`) succeed.
  - No functional changes to trait signatures; this is a structural refactor to support future platform-specific factories and cleaner testing.
  - Documentation added in `doc/views/2025-11-08T21-26-05-view-factory-extraction.md`.

### Added
- 2025-11-08T21-32-39: Added `create_mip_view` to `ViewFactory` and `ViewManager` to support MIP (Maximum Intensity Projection) view creation.
  - Trait method signature: `fn create_mip_view(&self, volume: &CTVolume, viewport_pos: (i32, i32), viewport_size: (u32, u32)) -> Result<Box<dyn View>, Box<dyn std::error::Error>>`.
  - Forwarding implementation in `ViewManager` with INFO/DEBUG logging and error propagation.
  - Updated test mocks to implement the new method; all `view_transition_integration_tests` pass.
  - Verified native build (`cargo build`) and WASM build (`wasm-pack build -t web`) succeed.
  - Documentation added in `doc/views/2025-11-08T21-32-39-mip-view-factory.md`.

### Changed
- 2025-11-08T21-58-55: Moved `DefaultViewFactory` into `src/rendering/view/view_factory.rs` and removed separate `default_factory` module.
  - Import path remains accessible via re-exports: `kepler_wgpu::rendering::DefaultViewFactory` (through `rendering::view::mod.rs` → `pub use view_factory::*`).
  - Simplified module structure; tests and existing imports remain functional.
  - No API changes; purely organizational refactor.

### Added
- 2025-11-08T22-09-22: Added `with_content` variants to `ViewFactory` for MPR and MIP views to enable GPU texture reuse.
  - New trait methods:
    - `create_mpr_view_with_content(&self, render_content: Arc<RenderContent>, vol: &CTVolume, orientation: Orientation, pos: (i32, i32), size: (u32, u32))`
    - `create_mip_view_with_content(&self, render_content: Arc<RenderContent>, pos: (i32, i32), size: (u32, u32))`
  - Implemented in `DefaultViewFactory` and `MockViewFactory`.
  - Benefits: Avoids repeated volume uploads and conversions; allows sharing one 3D texture across views.
  - Verified native build (`cargo build`) and targeted tests (`cargo test --test view_transition_integration_tests`).

### Added
- **Spine Vertebra Mesh**: Added anatomically-inspired spine vertebra mesh generation for medical imaging visualization
  - **Anatomical Structure**: Simplified thoracic vertebra representation with body, arch, and processes
  - **Medical Accuracy**: Bone-colored mesh (light beige/cream) for realistic anatomical reference
  - **Component-Based**: Vertebra body, vertebral arch, spinous process, transverse processes, and articular processes
  - **Lighting Ready**: Proper vertex normals for accurate lighting calculations in medical contexts
  - **Scalable Design**: Modular box-based construction allows for future anatomical refinements
  - **Cross-Platform**: Compatible with both native and WebAssembly builds
  - **Integration Ready**: Accessible via `Mesh::spine_vertebra()` method for immediate use in mesh views
- **Architecture Documentation**: Created comprehensive state structure refactoring plan (`doc/redering/state-structure-refactoring-plan.md`) outlining separation of concerns improvements for the monolithic State struct

### Removed
- **Unused Mesh State Management Code**: Removed unused mesh mode state management functionality from `src/rendering/core/state.rs`
  - **Field Removal**: Eliminated `mpr_state_slot2` field from State struct that was only used for storing MPR state snapshots
  - **Method Cleanup**: Removed `save_mpr_state()` and `restore_mpr_state()` methods that were never called in production code
  - **Mode Toggle Cleanup**: Removed `enable_mesh_mode()` and `disable_mesh_mode()` methods that were only defined but never used
  - **Code Reduction**: Eliminated approximately 50 lines of dead code that provided no functional value
  - **Maintainability**: Improved code clarity by removing unused state management complexity
  - **Memory Efficiency**: Reduced State struct size by removing unnecessary `Option<MPRViewState>` field
- **Unused Shader Validation Module**: Removed `src/rendering/view/mesh/shader_validation.rs` and associated test code
  - **Code Cleanup**: Eliminated 332 lines of unused shader validation functionality that was not used in production code
  - **Test Cleanup**: Removed `ShaderValidationError` tests from `error_handling_tests.rs` and `ShaderValidator` tests from `mesh_integration_tests.rs`
  - **Module Cleanup**: Removed shader validation module declaration and `ShaderValidationError` re-export from `src/rendering/view/mesh/mod.rs`
  - **Maintainability**: Improved codebase maintainability by removing dead code that was only referenced in test files
  - **Build Verification**: Confirmed removal does not affect production functionality or build process

### Changed
- **MPR Architecture Transition Completed**: Successfully migrated MPR (Multi-Planar Reconstruction) views to new shared rendering context architecture
  - **Shared Resource Management**: Implemented `MprRenderContext` for shared rendering resources (pipeline, buffers, bind group layouts)
  - **Per-View Implementation**: Created `MprViewWgpuImpl` for per-view WGPU resources (uniforms, bind groups)
  - **Memory Efficiency**: Eliminated resource duplication across multiple MPR views using Arc-based sharing
  - **Performance Improvement**: Reduced GPU memory usage and faster initialization through shared render pipeline
  - **Code Quality**: Better separation of concerns between shared and per-view resources
  - **Compatibility Maintained**: All existing MPR functionality preserved with unchanged public API
  - **Build Verification**: Both native (`cargo build`, `cargo test`) and WebAssembly (`wasm-pack build -t web`) builds successful
  - Documented in `doc/views/2025-01-12T16-00-00Z-mpr-architecture-transition-completion.md`

### Added
- **MPR View Architecture Design**: Created comprehensive design document for MPR (Multi-Planar Reconstruction) rendering system following the same modular architecture as MIP views
  - **Modular Component Design**: Defined clear separation between `MprRenderContext`, `RenderContent`, `MprViewWgpuImpl`, and `MprView`
  - **Medical Imaging Focus**: Specialized for anatomical orientations (Transverse, Coronal, Sagittal, Oblique) with precise coordinate systems
  - **Arc-based Sharing**: Designed for efficient resource sharing between multiple MPR views using `Arc<MprViewWgpuImpl>`
  - **Coordinate System Management**: Supports multiple coordinate systems (screen, UV, medical, volume) for medical accuracy
  - **Window/Level Processing**: Integrated tissue-specific brightness and contrast controls for clinical visualization
  - **Consistent Architecture**: Follows same design principles as MIP rendering for maintainability and consistency
  - Documented in `doc/views/mprview_design.md`
- **MPR Architecture Transition Plan**: Created detailed step-by-step migration plan to move from current MPR implementation to the designed modular architecture
  - **8-Step Migration Process**: Comprehensive plan with minimal disruption and maintained compilation at each step
  - **Risk Mitigation**: Detailed risk assessment and rollback strategies for each transition step
  - **Resource Sharing Benefits**: Clear path to Arc-based sharing for memory efficiency and thread safety
  - **Functionality Preservation**: Ensures all existing MPR features remain intact during transition
  - **Timeline Estimation**: 8-12 hours of focused development time with clear milestones
  - **Validation Strategy**: Compilation safety, visual verification, and automated testing at each step
  - Documented in `doc/views/mpr_architecture_transition_plan.md`

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