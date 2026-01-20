# Project Architecture

**Last Updated**: 2025-01-15

## Overview

Kepler2-WGPU is a Rust-based medical imaging application for processing and visualizing DICOM data using WebGPU. It provides cross-platform support (native + WASM) with high-performance CT volume rendering.

## Four-Layer Architecture

```
src/
├── core/           # Platform-independent utilities and types
├── data/           # Medical imaging data structures and parsing
├── rendering/      # WebGPU rendering system
└── application/    # UI orchestration and app lifecycle
```

### Module Dependencies (Bottom-Up)

#### 1. Core Layer (`src/core/`)
**Purpose**: Foundational utilities with no external dependencies

**Modules**:
- **`coord/`**: Coordinate systems (Base, World, Screen, Voxel)
- **`error.rs`**: `KeplerError` and `MprError` types
- **`geometry.rs`**: Geometric primitives and transformations (uses `glam`)
- **`timing.rs`**: Performance measurement utilities
- **`window_level.rs`**: Medical imaging window/level presets

**Key Types**:
- `KeplerError`: Centralized error handling
- `KeplerResult<T>`: Type alias for `Result<T, KeplerError>`
- Coordinate types: `WorldCoord`, `ScreenCoord`, `VoxelCoord`, `BaseCoord`

---

#### 2. Data Layer (`src/data/`)
**Purpose**: Domain models (depends on `core` only)

**Modules**:
- **`dicom/`**: Patient, StudySet, ImageSeries, CTImage, DicomRepo
- **`ct_volume.rs`**: `CTVolume` struct and `CTVolumeGenerator` trait
- **`medical_imaging/`**: MHA/MHD format support, validation, metadata
- **`volume_encoding.rs`**: Volume texture format selection (R16Float vs Rg8Unorm)
- **`dicom_.rs`**: DICOM file I/O and parsing
- **`export_dicom.rs`**: Export CT volumes to DICOM format

**Key Types**:
- `DicomRepo`: Repository of DICOM data (Patient → StudySet → ImageSeries → CTImage)
- `CTVolume`: Struct containing CT volume data (dimensions, spacing, voxel data, base coordinate)

---

#### 3. Rendering Layer (`src/rendering/`)
**Purpose**: Graphics engine (depends on `core` and `data`)

**Submodules**:
- **`core/`**: Graphics initialization, pipeline management, render passes
- **`view/`**: View system (View trait, MprView, MipView, MeshView)
- **`shaders/`**: WGSL shader code (embedded as strings)
- **`mesh/`**: 3D mesh loading, processing, and rendering

**Key Types**:
- `Graphics`: GPU context wrapper (Device, Queue, Surface)
- `View`: Trait for renderable components
- `RenderContent`: GPU resources for a specific view (buffers, bind groups, textures)
- `PipelineManager`: Caching and lifecycle management for GPU pipelines

---

#### 4. Application Layer (`src/application/`)
**Purpose**: UI layer (depends on all above)

**Modules**:
- **`app.rs`**: Main application state (`App` struct)
- **`render_app.rs`**: `RenderApp` wrapper for native/WASM entry
- **`gl_canvas.rs`**: `GLCanvas` API for external control
- **`app_model.rs`**: Application data model (volume, mesh state, window/level)
- **`appview.rs`**: View layout and orchestration

**Key Types**:
- `App`: Main orchestrator for rendering loop and event handling
- `RenderApp`: Entry point for native and WASM
- `AppModel`: Single source of truth for application data
- `GLCanvas`: External API for controlling the rendering canvas

---

## Entry Points

### Native
- **File**: `src/main.rs`
- **Command**: `cargo run` or `cargo run --bin kepler`
- **Features**: Uses tokio runtime, filesystem access

### WebAssembly
- **File**: `src/lib.rs`
- **Export**: `get_render_app()` for embedding in web
- **Build**: `wasm-pack build --target web`
- **Features**: Uses web-sys, console_log, wasm-bindgen

### Library
- **File**: `src/lib.rs`
- **Purpose**: Re-exports public API for external use

---

## Data Flow Architecture

### DICOM to Volume Pipeline

```
File System (native) / File Upload (WASM)
    ↓
data::dicom::fileio::parse_dcm_directories()
    ↓
DicomRepo (Patient → StudySet → ImageSeries → CTImage)
    ↓
DicomRepo::generate_ct_volume(image_series_uid)
    ↓
CTVolume { dimensions, voxel_spacing, voxel_data, base }
    ↓
AppModel::load_volume(CTVolume)
    ↓
RenderContent (GPU texture + metadata)
    ↓
View system (MprView, MipView, MeshView)
    ↓
WebGPU rendering
```

### Rendering Pipeline

```
App::render()
    ↓
PassExecutor::execute_frame()
    ↓
    ├─ MeshPass (if mesh view present)
    ├─ MipPass (if MIP view present)
    └─ SlicePass (MPR views)
    ↓
wgpu::Queue::submit()
    ↓
wgpu::Surface::present()
```

---

## MVC Pattern

The application follows a Model-View-Controller (MVC) inspired pattern:

### Model (`AppModel`)
- Holds authoritative state of loaded data (e.g., `CTVolume`)
- Manages data-centric settings (e.g., `enable_float_volume_texture`)
- Provides methods to extract render-ready data (e.g., `get_volume_render_data`)

### View (`AppView`)
- Defines visual layout (Grid, Split)
- Manages collection of active `View` instances (e.g., Transverse, Sagittal, Coronal)
- Handles window resizing and layout updates

### Controller (`App` / `RenderApp`)
- `RenderApp` handles `winit` event loop
- `App` coordinates between input events, `AppModel`, and rendering system

---

## Key File Reference

| File | Purpose | Key Concepts |
|------|---------|--------------|
| `src/lib.rs` | Public API re-exports, logger init, WASM entry | Re-exports, logger, `get_render_app()` |
| `src/main.rs` | Native binary entry point | `#[tokio::main]`, parse DICOM, render |
| `src/core/error.rs` | Error types | `KeplerError`, `MprError` |
| `src/data/ct_volume.rs` | CT volume representation | `CTVolume`, `CTVolumeGenerator` |
| `src/application/app.rs` | Main app state | `App`, `render()`, event handling |
| `src/rendering/core/graphics.rs` | GPU initialization | `Graphics`, `GraphicsContext` |
| `src/rendering/view/` | View system | `View`, `MprView`, `MipView`, `MeshView` |
| `tests/dicom_tests.rs` | DICOM test suite | Test patterns, mock data |

---

## External Dependencies

### Key Libraries

- **`wgpu`** (23.0): Cross-platform graphics API
- **`winit`** (0.29): Window and event loop management
- **`glam`** (0.30): Vector and matrix math
- **`dicom-*`** crates: DICOM file parsing
- **`wasm-bindgen`**: WASM/JS interop
- **`pollster`**: Async blocking for native code

### Platform-Specific

**Native only** (`[target.'cfg(not(target_arch = "wasm32"))'.dependencies]`):
- `tokio`: Async runtime
- `rayon`: Parallel processing

**WASM only** (`[target.'cfg(target_arch = "wasm32")'.dependencies]`):
- `console_error_panic_hook`: Panic handling
- `console_log`: Browser logging
- `web-sys`: Web API bindings
- `js-sys`: JavaScript interop

---

## Related Documentation

- **Quick Reference**: `QUICK_REFERENCE.md` - Common tasks and commands
- **Conventions**: `CONVENTIONS.md` - Coding standards and patterns
- **Rendering**: `RENDERING.md` - GPU and rendering details
- **Pitfalls**: `PITFALLS.md` - Common anti-patterns to avoid
- **Project Architecture**: `doc/architecture/ARCHITECTURE.md` - Deeper architectural design
