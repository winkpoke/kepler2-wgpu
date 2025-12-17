# Kepler WGPU Architecture

**Last Updated:** 2025-12-17
**Version:** 2.0
**Status:** Active

## 1. Overview

Kepler WGPU is a high-performance medical imaging framework built with Rust and WGPU. It is designed to run natively (Windows, macOS, Linux) and in the browser (WebAssembly). The system focuses on CT reconstruction, MPR (Multi-Planar Reconstruction), MIP (Maximum Intensity Projection), and 3D visualization with strict requirements for medical accuracy (e.g., Hounsfield unit preservation).

### Key Architectural Principles
- **Cross-Platform**: Unified codebase for Native and WASM targets.
- **Modular Design**: Clear separation between Core, Data, Rendering, and Application layers.
- **Data-Driven**: `AppModel` serves as the single source of truth for application state.
- **Performance**: Zero-cost abstractions, efficient GPU resource management, and minimized CPU-GPU synchronization.

## 2. System Layering

The codebase is organized into four distinct layers with strict dependency rules:

```
src/
├── core/           # Fundamental types, math, and error handling (No dependencies)
├── data/           # Domain models, DICOM parsing, Volume data (Depends on Core)
├── rendering/      # WGPU rendering engine, Views, Shaders (Depends on Core, Data)
└── application/    # UI orchestration, Event handling, App lifecycle (Depends on all)
```

### 2.1 Core Layer (`src/core/`)
Provides foundational utilities used across the entire application.
- **`coord.rs`**: Coordinate systems and transformations.
- **`error.rs`**: Centralized error handling (`KeplerError`) using `thiserror` and `anyhow`.
- **`geometry.rs`**: Basic geometric primitives.
- **`timing.rs`**: Performance timing utilities.

### 2.2 Data Layer (`src/data/`)
Handles medical imaging domain logic and data structures.
- **`ct_volume.rs`**: Core data structure for CT volumes.
- **`dicom/`**: Comprehensive DICOM parsing (Series, Study, Patient modules).
- **`medical_imaging/`**: Formats (MHD/MHA) and metadata validation.
- **Design**: Immutable data structures where possible; clear separation from presentation.

### 2.3 Rendering Layer (`src/rendering/`)
The heart of the visualization engine, built on top of `wgpu`.

#### Core Infrastructure (`rendering/core/`)
- **`graphics.rs`**: Manages `wgpu::Device`, `Queue`, and `Surface`.
- **`pipeline.rs`**: Pipeline state management and caching.
- **`texture.rs`**: GPU texture abstractions.

#### View System (`rendering/view/`)
- **`View` Trait**: Abstract interface for renderable components (MPR, MIP, Mesh).
- **`RenderContent`**: Manages resources (buffers, bind groups) for a specific view.
- **`Layout`**: Dynamic layout management for multi-view arrangements.
- **`mesh/`**: 3D Mesh rendering subsystem (feature-gated logic).

#### Shaders (`rendering/shaders/`)
- WGSL and GLSL shaders for volume casting, MIP, and mesh rendering.

### 2.4 Application Layer (`src/application/`)
Orchestrates the application lifecycle and user interaction.
- **`App`**: Main entry point and orchestrator.
- **`AppModel`**: Application state container (holds `CTVolume`, configuration).
- **`AppView`**: Manages the UI layout and view composition.
- **`RenderApp`**: Winit event loop integration.

## 3. Application Model

The application follows a Model-View-Controller (MVC) inspired pattern:

- **Model (`AppModel`)**:
  - Holds the authoritative state of the loaded data (e.g., `CTVolume`).
  - Manages data-centric settings (e.g., `enable_float_volume_texture`).
  - Provides methods to extract render-ready data (e.g., `get_volume_render_data`).
  
- **View (`AppView`)**:
  - Defines the visual layout (Grid, Split).
  - Manages the collection of active `View` instances (e.g., Transverse, Sagittal, Coronal).
  - Handles window resizing and layout updates.

- **Controller (`App` / `RenderApp`)**:
  - `RenderApp` handles the `winit` event loop.
  - `App` coordinates between input events, the `AppModel`, and the rendering system.

## 4. Rendering Pipeline

1.  **Initialization**: `GraphicsContext` sets up the WGPU device and surface.
2.  **Resource Loading**: `AppModel` loads data; `ViewFactory` creates GPU resources (`RenderContent`).
3.  **Update Loop**: Input events modify `AppModel` or View state (e.g., window/level, slice position).
4.  **Render Loop**:
    - `PassExecutor` begins a frame.
    - Active Views record render passes via the `View` trait.
    - Command buffers are submitted to the queue.

## 5. Roadmap & Future Architecture

### 5.1 WGPU 27 & Winit 0.30 Upgrade (Pending)
The system is currently on `wgpu 23.0` and `winit 0.29`. A migration is planned to:
- Upgrade to **wgpu 27.0+** and **winit 0.30+**.
- Adopt `raw-window-handle` 0.6.
- Refactor the event loop to use the new `winit` trait-based API.
- Update surface creation to use `wgpu::SurfaceTarget`.

### 5.2 Performance & Reliability
- **Trace Logging**: Implement `trace-logging` feature flag for detailed diagnostics (partially planned).
- **Error Recovery**: Enhance GPU context loss recovery and shader compilation error handling.
- **Testing**: Expand test coverage for WASM targets and property-based testing for medical algorithms.

### 5.3 Feature Enhancements
- **Async Loading**: Move data loading to a fully asynchronous pipeline to prevent UI blocking.
- **Advanced Visualization**: Implement advanced transfer functions and overlays for medical analysis.

## 6. Development Guidelines

- **Feature Flags**: Use `mesh` for 3D mesh capabilities; `trace-logging` for debugging.
- **WASM Compatibility**: All core logic must be WASM-compatible. Avoid blocking threads.
- **Documentation**: Keep architecture docs updated in `doc/architecture/`.
