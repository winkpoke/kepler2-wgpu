# Copilot / AI Agent Instructions for Kepler2-WGPU

Target audience: automated coding assistants and contributors who will modify or extend the kepler-wgpu Rust codebase.

Keep edits concise and only modify the files listed in the PR description unless asked otherwise.

What this project is
- A Rust crate that provides a cross-platform (native + WASM) DICOM processing and WebGPU rendering application.
- Native entry: binary `kepler` (`src/main.rs`). Library re-exports main subsystems from `src/lib.rs` and exposes `get_render_app()` for embedding or WASM.
- Major subsystems: `application` (UI and app orchestration), `data` (DICOM parsing, CT volume generation), `rendering` (WebGPU pipelines, views, mesh), `core` (utilities, error types, math).

Quick build & run hints
- Native build: `cargo build --release` and run with `cargo run` (or `cargo run --bin kepler`).
- WASM build (minimal): install `wasm-pack` then `wasm-pack build` and serve `./static` with a static server (e.g., `npx live-server ./static`).
- Tests: unit and integration tests live in `tests/` and can be run with `cargo test`.

Project-specific patterns & conventions
- Cross-platform conditional compilation: many modules use `cfg(target_arch = "wasm32")` to separate WASM vs native code paths. Preserve feature-specific imports (e.g., `tokio` only on native targets).
- Logging: `init_logger()` is implemented in `src/lib.rs` with different implementations for native (env_logger) and WASM (console logging). Prefer using crate-level `log` macros (info/warn/error) so output is consistent across targets.
- Singleton/Global helpers: `once_cell` and `async_lock`/`tokio::sync::Mutex` are used selectively. Match the existing pattern: `#[cfg(target_arch = "wasm32")]` -> `async_lock::Mutex`, else `tokio::sync::Mutex`.
- Re-exports: `src/lib.rs` re-exports commonly used types (e.g., `RenderApp`, `GLCanvas`, `CTVolume`). When changing public API, update `lib.rs` re-exports accordingly.
- Window/canvas handling: `get_render_app()` builds a Winit `EventLoop` and `Window` and on WASM appends the canvas into the DOM element with id `wasm-example`. When modifying DOM interaction ensure the DOM id and CSS sizing behavior are respected.

Key files to reference while coding
- `src/lib.rs` — central re-exports, logging init, `get_render_app()` and WASM start hooks.
- `src/application/` — app lifecycle, `render_app.rs`, `gl_canvas.rs` (exposes `GLCanvas` and `UserEvent`), event handling.
- `src/data/` — DICOM parsing (`dicom`), CT volume generation (`ct_volume.rs`), file io helpers in `application` call into here.
- `src/rendering/` — pipeline management, mesh, view system. Look at `rendering/core/state.rs` for GPU initialization and `pipeline.rs` for bind groups and layout conventions.
- `pkg/` and `static/` — pre-built JS/WASM artifacts and static web assets used by the Web demo.
- `tests/` — examples of intended behavior and edge cases; follow the patterns used here for new tests.

Design & data flows (big picture)
- File IO (native): `application` calls into `data::dicom::fileio` which returns a repository of studies/series. `ct_volume::CTVolumeGenerator` converts series into a `CTVolume`.
- Rendering: `RenderApp` owns `State` (GPU device, queue, surface) and creates `GLCanvas` views. `GLCanvas::load_data_from_ct_volume(&CTVolume)` converts CPU volumes to GPU buffers/textures using `wgpu` buffers and textures.
- WASM considerations: GPU initialization and logging differ on web — watch for `web_sys` and `wasm-bindgen` usage; canvas sizing is controlled by CSS so the code forces an initial size on startup.

Common pitfalls to avoid
- Don't import `tokio` in WASM-targeted modules; follow target-gated dependencies in `Cargo.toml`.
- When changing any GPU pipeline shader or bind group layout, update `PipelineManager` and layout creation sites to avoid mismatches.
- Tests often assume native-only behavior; mark tests with `#[cfg(not(target_arch = "wasm32"))]` if they rely on `tokio` or native filesystem paths.

Examples from the codebase
- Logger init (native vs wasm): see `init_logger()` in `src/lib.rs`.
- WASM canvas injection: DOM element id `wasm-example` is referenced in `get_render_app()` (keep this id in `static/index.html`).
- Re-export pattern: `pub use application::{render_app::RenderApp, gl_canvas::GLCanvas};` in `src/lib.rs` — keep public API stable.

When editing or adding code
- Keep changes minimal and well-scoped. Follow existing file/module grouping (core, data, rendering, application).
- Add unit tests in `tests/` or alongside the module. Use `cargo test` and ensure platform guards for WASM/native differences.
- Run `cargo build` (or `cargo check`) and `cargo test` locally before proposing large PRs.

If you need more info
- Inspect `doc/` for design notes and recent implementation summaries (e.g., `redering/mip-*.md`).
- Ask for the preferred CI commands or target matrix (native/dev/release/wasm) if you need to change build scripts.

Keep this file short. If more project-specific guidance is needed (e.g., shader hot-reload procedure, CI matrix), request specific sections from a maintainer.
