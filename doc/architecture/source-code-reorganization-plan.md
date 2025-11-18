# Source Code Reorganization Plan

## Overview

This document outlines a comprehensive reorganization plan for the `src` directory structure of the Kepler WGPU project. The reorganization follows software architecture best practices to improve maintainability, scalability, and code clarity.

## Current Structure Analysis

### Strengths of Current Organization
- ✅ Feature-gated `mesh` module with clear separation
- ✅ Well-organized `dicom` module with logical grouping
- ✅ Separate `view` module for rendering abstractions
- ✅ Dedicated `shader` directory for WGSL files

### Areas for Improvement
- ❌ Root-level files lack clear categorization
- ❌ Rendering-related files scattered across root
- ❌ Missing clear separation between core, rendering, and application layers
- ❌ Inconsistent module organization patterns

## Recommended New Structure

### 1. Core Architecture Layers

```
src/
├── core/                    # Core utilities and foundational types
│   ├── mod.rs
│   ├── coord.rs            # Moved from root
│   ├── timing.rs           # Moved from root  
│   ├── error.rs            # Moved from root
│   └── geometry.rs         # Moved from root
├── data/                   # Data structures and domain models
│   ├── mod.rs
│   ├── ct_volume.rs        # Moved from root
│   └── dicom/              # Keep existing structure
│       ├── mod.rs
│       ├── ct_image.rs
│       ├── dicom_helper.rs
│       ├── dicom_repo.rs
│       ├── fileio.rs
│       ├── image_series.rs
│       ├── patient.rs
│       └── studyset.rs
├── rendering/              # All rendering-related functionality
│   ├── mod.rs
│   ├── core/               # Core rendering infrastructure
│   │   ├── mod.rs
│   │   ├── pipeline.rs     # Moved from root
│   │   ├── pipeline_builder.rs # Moved from root
│   │   ├── texture.rs      # Moved from root
│   │   └── state.rs        # Moved from root
│   ├── passes/             # Render pass management
│   │   ├── mod.rs
│   │   └── render_pass.rs  # Moved from root
│   ├── content/            # Render content management
│   │   ├── mod.rs
│   │   └── render_content.rs # Moved from root
│   ├── view/               # Keep existing view structure
│   │   ├── mod.rs
│   │   ├── view.rs
│   │   ├── render_context.rs
│   │   ├── renderable.rs
│   │   └── layout.rs
│   ├── mesh/               # Keep existing mesh structure (feature-gated)
│   │   ├── mod.rs
│   │   ├── mesh.rs
│   │   ├── material.rs
│   │   ├── camera.rs
│   │   ├── lighting.rs
│   │   ├── mesh_view.rs
│   │   ├── mesh_render_context.rs
│   │   ├── texture_pool.rs
│   │   ├── shader_validation.rs
│   │   └── performance.rs
│   └── shaders/            # Renamed from shader
│       ├── mesh.wgsl
│       ├── mesh_depth.wgsl
│       ├── shader.wgsl
│       ├── shader2.frag.glsl
│       └── shader_tex.wgsl
├── application/            # Application layer and UI
│   ├── mod.rs
│   ├── render_app.rs       # Moved from root
│   └── gl_canvas.rs        # Moved from root
├── lib.rs                  # Keep as main library entry point
└── main.rs                 # Keep as application entry point
```

## Module Responsibilities

### Core Module (`src/core/`)
**Purpose**: Foundational utilities used across the entire application

**Contents**:
- `coord.rs`: Coordinate systems and transformations
- `timing.rs`: Performance timing utilities
- `error.rs`: Error handling and custom error types
- `geometry.rs`: Basic geometric primitives and operations

**Dependencies**: Minimal external dependencies, no rendering dependencies

**Design Principles**:
- Pure functions where possible
- No side effects
- Reusable across all application layers

### Data Module (`src/data/`)
**Purpose**: Domain models and data structures

**Contents**:
- `ct_volume.rs`: CT volume data structures and operations
- `dicom/`: DICOM parsing, patient data, and medical imaging structures

**Dependencies**: Core utilities only, no rendering dependencies

**Design Principles**:
- Domain-driven design
- Clear separation from presentation logic
- Immutable data structures where appropriate

### Rendering Module (`src/rendering/`)
**Purpose**: All graphics and rendering functionality

**Submodules**:

#### `core/` - Core Rendering Infrastructure
- `pipeline.rs`: Pipeline management and caching
- `pipeline_builder.rs`: Pipeline construction utilities
- `texture.rs`: Texture management and operations
- `state.rs`: Graphics state management

#### `passes/` - Render Pass Management
- `render_pass.rs`: Render pass orchestration and execution

#### `content/` - Render Content Management
- `render_content.rs`: Content organization and management

#### `view/` - View Abstractions
- `view.rs`: View trait and base implementations
- `render_context.rs`: Rendering context management
- `renderable.rs`: Renderable trait and utilities
- `layout.rs`: Layout management and composition

#### `mesh/` - 3D Mesh Rendering (Feature-Gated)
- All existing mesh-related files
- Feature-gated behind `cfg(feature = "mesh")`

#### `shaders/` - Shader Assets
- All WGSL and GLSL shader files
- Renamed from `shader` for clarity

**Dependencies**: Core utilities and data structures

**Design Principles**:
- Separation of concerns between different rendering aspects
- Clear abstraction layers
- Performance-oriented design

### Application Module (`src/application/`)
**Purpose**: Application-level coordination and user interface

**Contents**:
- `render_app.rs`: Main render application and event loop
- `gl_canvas.rs`: Canvas management and user interaction

**Dependencies**: All other modules as needed

**Design Principles**:
- High-level orchestration
- User interface concerns
- Platform-specific adaptations

## Dependency Architecture

### Dependency Graph
```
application/
    ↓
rendering/
    ↓
data/ ← core/
```

### Dependency Rules
1. **`core/`** has no internal dependencies
2. **`data/`** depends only on `core/`
3. **`rendering/`** depends on `core/` and `data/`
4. **`application/`** can depend on all modules
5. **No circular dependencies** allowed
6. **Feature gates** must be respected across all layers

## Updated Library Structure

### New `lib.rs` Organization

```rust
#![feature(duration_millis_float)]

// Core foundational modules
pub mod core;

// Data and domain models  
pub mod data;

// Rendering system
pub mod rendering;

// Application layer
pub mod application;

// Re-export commonly used types for backward compatibility
pub use core::{coord, error::KeplerError, timing};
pub use data::{ct_volume, dicom};
pub use rendering::{
    view::{View, Renderable, Layout},
    core::{pipeline::PipelineManager, state::State},
};
pub use application::{render_app::RenderApp, gl_canvas::GLCanvas};

// Feature-gated exports
#[cfg(feature = "mesh")]
pub use rendering::mesh;

// Main application entry points
pub use application::render_app::{get_render_app, create_graphics};
```

### Backward Compatibility Strategy
- Maintain existing public API through re-exports
- Gradual migration path for external consumers
- Clear deprecation warnings for old import paths

## Migration Implementation Plan

### Phase 1: Create New Module Structure (Week 1)
**Objective**: Establish new directory structure without breaking existing code

**Tasks**:
1. Create new directory structure
2. Add new `mod.rs` files with proper module declarations
3. Update `lib.rs` with new module organization
4. Ensure compilation still works

**Success Criteria**:
- New directories exist
- `cargo check` passes
- No functionality changes

### Phase 2: Move Core Files (Week 1)
**Objective**: Migrate foundational utilities

**Tasks**:
1. Move `coord.rs`, `timing.rs`, `error.rs`, `geometry.rs` to `src/core/`
2. Update import statements in moved files
3. Add re-exports in `core/mod.rs`
4. Update dependent files

**Success Criteria**:
- Core files successfully moved
- All tests pass
- No compilation errors

### Phase 3: Move Data Files (Week 2)
**Objective**: Migrate data structures and domain models

**Tasks**:
1. Move `ct_volume.rs` to `src/data/`
2. Move `dicom/` directory to `src/data/dicom/`
3. Update import statements
4. Update `data/mod.rs` with proper exports

**Success Criteria**:
- Data files successfully moved
- DICOM functionality intact
- All tests pass

### Phase 4: Move Rendering Files (Week 2-3)
**Objective**: Reorganize rendering system

**Tasks**:
1. Move pipeline files to `src/rendering/core/`
2. Move render pass files to `src/rendering/passes/`
3. Move content files to `src/rendering/content/`
4. Move view files to `src/rendering/view/`
5. Move mesh files to `src/rendering/mesh/`
6. Rename `shader/` to `src/rendering/shaders/`
7. Update all import statements

**Success Criteria**:
- All rendering files properly organized
- Feature gates still work
- Mesh functionality intact
- All tests pass

### Phase 5: Move Application Files (Week 3)
**Objective**: Complete the reorganization

**Tasks**:
1. Move `render_app.rs`, `gl_canvas.rs` to `src/application/`
2. Update import statements
3. Update `application/mod.rs`
4. Final cleanup and verification

**Success Criteria**:
- All files moved successfully
- Complete application functionality
- All tests pass

### Phase 6: Update Documentation and Tests (Week 4)
**Objective**: Complete migration with full documentation

**Tasks**:
1. Update all documentation to reflect new structure
2. Update test imports and module references
3. Verify all feature combinations work
4. Update README and other documentation

**Success Criteria**:
- Documentation updated
- All tests pass
- Both native and WASM builds work
- Feature flags work correctly

## Benefits of Reorganization

### Improved Maintainability
- **Clear separation of concerns** between layers
- **Easier to locate** and modify specific functionality
- **Reduced coupling** between unrelated components
- **Consistent organization** patterns throughout codebase

### Better Scalability
- **New rendering features** can be added to appropriate submodules
- **Data processing features** isolated from rendering concerns
- **Application-level features** don't affect core functionality
- **Feature gates** properly organized and maintainable

### Enhanced Testability
- **Core utilities** can be tested independently
- **Rendering components** can be tested without application layer
- **Clear dependency boundaries** enable better unit testing
- **Mock implementations** easier to create and maintain

### Clearer Architecture
- **Dependency flow** is explicit and unidirectional
- **Module responsibilities** are well-defined
- **Feature gates** are properly organized
- **Code navigation** is more intuitive

## Naming Conventions

### Module Naming
- **Modules**: `snake_case` (e.g., `render_pass`, `mesh_view`)
- **Files**: `snake_case.rs` (e.g., `pipeline_builder.rs`)
- **Directories**: `snake_case` (e.g., `rendering/core/`)

### Feature Gates
- Maintain existing `mesh` feature structure
- All mesh-related code behind `cfg(feature = "mesh")`
- Clear feature documentation in module headers

### Import Conventions
- Use absolute paths from crate root
- Group imports by module hierarchy
- Separate external crate imports from internal imports

## Risk Assessment and Mitigation

### High-Risk Areas
1. **Import Statement Updates**: Large number of files to update
   - **Mitigation**: Systematic approach, one module at a time
   - **Verification**: Compile after each phase

2. **Feature Gate Integrity**: Mesh feature must remain properly gated
   - **Mitigation**: Test both with and without mesh feature
   - **Verification**: Automated testing in CI

3. **Backward Compatibility**: External consumers may break
   - **Mitigation**: Maintain re-exports in lib.rs
   - **Verification**: Test existing API usage patterns

### Medium-Risk Areas
1. **Test Updates**: Test imports need updating
   - **Mitigation**: Update tests in parallel with code moves
   - **Verification**: Run full test suite after each phase

2. **Documentation Consistency**: Docs may become outdated
   - **Mitigation**: Update docs as part of migration
   - **Verification**: Review all documentation files

### Low-Risk Areas
1. **Performance Impact**: Reorganization should not affect performance
2. **Functionality**: No functional changes planned
3. **Platform Support**: No platform-specific changes

## Success Metrics

### Compilation Metrics
- ✅ `cargo check` passes
- ✅ `cargo build` passes
- ✅ `cargo build --features mesh` passes
- ✅ `wasm-pack build -t web --features mesh` passes

### Test Metrics
- ✅ All unit tests pass
- ✅ All integration tests pass
- ✅ All feature combinations work
- ✅ Performance tests show no regression

### Code Quality Metrics
- ✅ No increase in compilation warnings
- ✅ Import statements are clean and organized
- ✅ Module boundaries are respected
- ✅ Documentation is up to date

## Future Considerations

### Extensibility
- New rendering backends can be added to `rendering/` module
- Additional data formats can be added to `data/` module
- Platform-specific code can be organized in `application/` module

### Performance Optimizations
- Module organization supports better compilation parallelization
- Clear boundaries enable targeted optimizations
- Feature gates allow minimal builds for specific use cases

### Maintenance
- Clear module responsibilities reduce maintenance burden
- Consistent organization patterns ease onboarding
- Well-defined dependencies prevent architectural drift

---

## Conclusion

This reorganization plan provides a solid foundation for the future development of the Kepler WGPU project. By implementing clear architectural layers, consistent naming conventions, and proper dependency management, the codebase will become more maintainable, scalable, and easier to understand.

The phased implementation approach ensures minimal disruption to ongoing development while providing clear milestones and success criteria for each step of the migration.