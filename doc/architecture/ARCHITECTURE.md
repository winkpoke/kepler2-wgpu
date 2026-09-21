# Kepler WGPU Architecture

**Last Updated:** 2026-09-14
**Version:** 2.1
**Status:** Active

## 1. Overview

Kepler WGPU is a comprehensive medical imaging framework built with Rust and WGPU. It is designed to run natively (Windows, macOS, Linux) and in the browser (WebAssembly). The system focuses on CT reconstruction, MPR (Multi-Planar Reconstruction), MIP (Maximum Intensity Projection), 3D visualization, and medical AI integration, with strict requirements for medical accuracy (e.g., Hounsfield unit preservation).

### Key Architectural Principles
- **Cross-Platform**: Unified codebase for Native and WASM targets with feature-gated components
- **Modular Design**: Clear separation between Core, Data, Rendering, Application, and Acquisition layers
- **Data-Driven**: `AppModel` serves as the single source of truth for application state
- **Performance**: Zero-cost abstractions, efficient GPU resource management, and minimized CPU-GPU synchronization
- **Extensible**: Plugin architecture for AI/ML integration and specialized processing workflows

## 2. System Layering

The codebase is organized into five distinct layers with strict dependency rules:

```
src/
├── core/           # Fundamental types, math, and error handling (No dependencies)
├── data/           # Domain models, DICOM parsing, Volume data (Depends on Core)
├── rendering/      # WGPU rendering engine, Views, Shaders (Depends on Core, Data)
├── acquisition/    # Medical data acquisition and processing (Depends on Core, Data)
└── application/    # UI orchestration, Event handling, App lifecycle (Depends on all)
├── server/         # Native-only server components (Not available in WASM) (Depends on Core, Data, Acquisition)
```

### 2.1 Core Layer (`src/core/`)
Provides foundational utilities used across the entire application.
- **`coord/`**: Coordinate systems and transformations (3D matrices, orientations)
- **`error.rs`**: Centralized error handling (`KeplerError`) using `thiserror` and `anyhow`
- **`geometry.rs`**: Basic geometric primitives and spatial calculations
- **`timing.rs`**: Performance timing utilities and profiling support
- **`window_level.rs`**: Window/Level adjustment algorithms for medical imaging

### 2.2 Data Layer (`src/data/`)
Handles medical imaging domain logic and data structures.
- **`ct_volume.rs`**: Core data structure for CT volumes with Hounsfield unit preservation
- **`dicom/`**: Comprehensive DICOM parsing (Series, Study, Patient modules)
- **`medical_imaging/`**: Formats (MHD/MHA) and metadata validation
- **Design**: Immutable data structures where possible; clear separation from presentation

### 2.3 Rendering Layer (`src/rendering/`)
The heart of the visualization engine, built on top of `wgpu`.

#### Core Infrastructure (`rendering/core/`)
- **`graphics.rs`**: Manages `wgpu::Device`, `Queue`, and `Surface`
- **`pipeline.rs`**: Pipeline state management and caching
- **`texture.rs`**: GPU texture abstractions and format validation

#### View System (`rendering/view/`)
- **`View` Trait**: Abstract interface for renderable components (MPR, MIP, Mesh)
- **`RenderContent`**: Manages resources (buffers, bind groups) for a specific view
- **`Layout`**: Dynamic layout management for multi-view arrangements
- **`mesh/`**: 3D Mesh rendering subsystem (feature-gated logic)

#### Shaders (`rendering/shaders/`)
- WGSL and GLSL shaders for volume casting, MIP, and mesh rendering

### 2.4 Acquisition Layer (`src/acquisition/`)
Handles medical data acquisition, processing, and specialized workflows.
- **`process.rs`**: Medical image processing pipeline and algorithms
- **`remedy.rs`**: REMEDY serial workflow implementation for medical device integration
- **`error.rs`**: Acquisition-specific error handling and validation

### 2.5 Application Layer (`src/application/`)
Orchestrates the application lifecycle and user interaction.
- **`App`**: Main entry point and orchestrator
- **`AppModel`**: Application state container (holds `CTVolume`, configuration)
- **`AppView`**: Manages the UI layout and view composition
- **`RenderApp`**: Winit event loop integration and WASM support
- **`GLCanvas`**: Cross-platform canvas abstraction for rendering

### 2.6 Server Layer (`src/server/`)
Native-only server components for advanced workflows and AI integration (Not available in WASM).
- **`handlers.rs`**: Request handlers for medical imaging workflows
- **`state.rs`**: Server state management and session handling
- **`ai_handler.rs`**: AI/ML integration handlers
- **`ai_model.rs`**: AI model abstractions and management
- **`ai_task.rs`**: AI task orchestration and execution
- **`navcomputer.rs`**: Navigation and guidance system
- **`ws.rs`**: WebSocket support for real-time communication
- **`routes.rs`**: REST API routes for medical imaging endpoints

## 3. Application Model

The application follows a Model-View-Controller (MVC) inspired pattern:

### Model (`AppModel`)
- Holds the authoritative state of the loaded data (e.g., `CTVolume`)
- Manages data-centric settings (e.g., `enable_float_volume_texture`)
- Provides methods to extract render-ready data (e.g., `get_volume_render_data`)
- Coordinates with acquisition layer for data processing

### View (`AppView`)
- Defines the visual layout (Grid, Split)
- Manages the collection of active `View` instances (e.g., Transverse, Sagittal, Coronal)
- Handles window resizing and layout updates

### Controller (`App` / `RenderApp`)
- `RenderApp` handles the `winit` event loop
- `App` coordinates between input events, the `AppModel`, and the rendering system
- Bridges native and WASM platforms through feature gates

## 4. Rendering Pipeline

1.  **Initialization**: `GraphicsContext` sets up the WGPU device and surface
2.  **Resource Loading**: `AppModel` loads data; `ViewFactory` creates GPU resources (`RenderContent`)
3.  **Update Loop**: Input events modify `AppModel` or View state (e.g., window/level, slice position)
4.  **Render Loop**:
    - `PassExecutor` begins a frame
    - Active Views record render passes via the `View` trait
    - Command buffers are submitted to the queue

## 5. Acquisition Integration

### Medical Data Processing Pipeline
1. **Data Ingestion**: DICOM files loaded through `data/dicom/`
2. **Validation**: Medical imaging standards compliance checked
3. **Processing**: `acquisition/process.rs` applies medical algorithms
4. **Storage**: Processed data stored in optimized GPU formats
5. **Rendering**: Results visualized through the rendering pipeline

### REMEDY Integration
- **`acquisition/remedy.rs`**: Implementation of REMEDY serial workflow
- Device integration for medical imaging equipment
- Real-time data streaming and processing
- Quality control and validation

### AI/ML Integration (Native-only)
- **`server/ai_handler.rs`**: AI model inference and processing
- **`server/navcomputer.rs`**: Advanced medical image analysis
- **Python Integration**: Seamless integration with Python ML libraries
- **WebSocket Support**: Real-time AI processing results

## 6. Cross-Platform Architecture

### Native Platform (Windows, macOS, Linux)
- Full server functionality with AI integration
- Advanced medical device connectivity
- Local file system access
- Multi-threaded processing capabilities

### WebAssembly Platform
- Core medical imaging functionality
- Web-based DICOM viewing
- GPU-accelerated rendering in browser
- Limited server functionality (file loading only)

### Feature Gate Pattern
```rust
#[cfg(not(target_arch = "wasm32"))]
pub mod server;  // Native-only components

#[cfg(feature = "mesh")]
pub mod mesh;     // Optional 3D mesh capabilities
```

## 7. Roadmap & Future Architecture

### 7.1 WGPU 27 & Winit 0.30 Upgrade (Pending)
The system is currently on `wgpu 23.0` and `winit 0.29`. A migration is planned to:
- Upgrade to **wgpu 27.0+** and **winit 0.30+**
- Adopt `raw-window-handle` 0.6
- Refactor the event loop to use the new `winit` trait-based API
- Update surface creation to use `wgpu::SurfaceTarget`

### 7.2 Enhanced AI Integration
- **Model Management**: Advanced AI model lifecycle management
- **Real-time Processing**: Stream-based AI inference for medical imaging
- **Integration Framework**: Standardized interfaces for AI/ML components

### 7.3 Performance & Reliability
- **Trace Logging**: Implement `trace-logging` feature flag for detailed diagnostics
- **Error Recovery**: Enhance GPU context loss recovery and shader compilation error handling
- **Testing**: Expand test coverage for WASM targets and property-based testing for medical algorithms

### 7.4 Feature Enhancements
- **Async Loading**: Move data loading to a fully asynchronous pipeline to prevent UI blocking
- **Advanced Visualization**: Implement advanced transfer functions and overlays for medical analysis
- **Collaboration**: Multi-user collaboration features for medical image review

## 8. Development Guidelines

### Module Organization
- **Feature Flags**: Use `mesh` for 3D mesh capabilities; `trace-logging` for debugging
- **WASM Compatibility**: All core logic must be WASM-compatible. Avoid blocking threads
- **Documentation**: Keep architecture docs updated in `doc/architecture/`

### Error Handling
- **Centralized Error Types**: Use `KeplerError` for consistent error handling
- **Medical Safety**: Critical medical operations must validate inputs and provide clear error messages
- **Graceful Degradation**: System should continue operating with reduced functionality on errors

### Performance Considerations
- **GPU Resource Management**: Efficient texture and buffer lifecycle management
- **Memory Optimization**: Minimize CPU-GPU synchronization and memory allocations
- **Medical Accuracy**: Ensure all processing preserves Hounsfield units and medical data integrity

### Testing Strategy
- **Unit Tests**: Comprehensive testing for core functionality
- **Integration Tests**: End-to-end testing for medical imaging workflows
- **Regression Tests**: Prevent regression in medical accuracy and functionality
- **Cross-Platform Testing**: Ensure consistent behavior across native and WASM platforms