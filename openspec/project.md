# Project Context

## Purpose
Kepler-WGPU is a high-performance, cross-platform medical imaging framework built in Rust. It leverages WebGPU for real-time visualization of CT data, supporting Multi-Planar Reconstruction (MPR), Maximum Intensity Projection (MIP), and 3D mesh rendering on both native desktop and web (WebAssembly) environments.

## Tech Stack
- **Language**: Rust (2021 Edition)
- **Graphics**: WGPU (WebGPU implementation)
- **Web**: WebAssembly (wasm-bindgen, web-sys, js-sys)
- **Async Runtime**: Tokio (native), wasm-bindgen-futures (web)
- **Math**: cgmath, ndarray
- **Data**: dicom-rs (dicom-object, dicom-core), serde
- **Utils**: log, env_logger, anyhow, thiserror

## Project Conventions

### Code Style
- Use `rustfmt` for formatting (4 spaces).
- **Naming**: `PascalCase` for types/traits, `snake_case` for functions/variables/modules.
- **Platform Gates**: Use `cfg(target_arch = "wasm32")` for web-specific code.
- **Async**: Use `async_lock::Mutex` for cross-platform compatibility, `tokio::sync::Mutex` only in native-specific paths.

### Architecture Patterns
- **Module Structure**:
  - `application/`: App lifecycle, event handling, UI orchestration.
  - `core/`: Common utilities, math helpers, error types.
  - `data/`: DICOM parsing, volume generation, file I/O.
  - `rendering/`: WGPU pipelines, view implementations (MPR, MIP, Mesh), render passes.
- **Rendering**: Pipeline-based rendering with efficient resource management.
- **State Management**: Separation of application state (AppModel) from rendering state.

### Testing Strategy
- **Unit/Integration**: `cargo test` for native logic.
- **WASM**: Manual browser testing via `wasm-pack build -t web` (no node-based automated tests).
- **Location**: Integration tests in `tests/` (`*_tests.rs`), unit tests co-located in `src/`.
- **Gating**: Use `#[cfg(not(target_arch = "wasm32"))]` for native-only tests.

### Git Workflow
- **Commits**: Conventional Commits format -> `type(scope): description`.
  - Types: `feat`, `fix`, `refactor`, `docs`, `style`, `test`, `chore`.
- **Branches**: Feature branches merged via PRs.

## Domain Context
- **Medical Imaging**: Focus on CT (Hounsfield Units), DICOM standards.
- **Views**:
  - **MPR**: Multi-Planar Reconstruction (Axial, Sagittal, Coronal, Oblique).
  - **MIP**: Maximum Intensity Projection.
  - **3D**: Isosurface/Mesh visualization.
- **Projections**: Orthogonal projection is preferred for medical accuracy (avoid perspective unless specified).
- **Coordinates**: Careful handling of Rust (row-major) vs Shader (column-major) matrices; transpose before upload.

## Important Constraints
- **Cross-Platform**: Must build/run on Windows, macOS, Linux, and Web (WASM).
- **Performance**: Target 60+ FPS; minimize CPU-GPU synchronization; efficient memory usage.
- **Dependencies**: No `tokio` imports in WASM modules.
- **Logging**: `INFO` default; `TRACE` (gated by `trace-logging`) for high-freq diagnostics.
- **Build**: `cargo build` (native), `wasm-pack build -t web` (web).

## External Dependencies
- **WGPU**: Graphics backend.
- **DICOM**: `dicom-rs` ecosystem for file parsing.
- **Web**: `web-sys` for DOM interaction in WASM.
