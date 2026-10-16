# Application Module (src/application)

This folder contains the application orchestration layer: it wires input/events, view management, and rendering together.

## Scope
- Owns high-level app lifecycle and state transitions.
- Bridges UI/event handling to rendering/view logic.
- Avoids owning domain parsing/IO details (those live in `src/data` and `src/acquisition`).

## Main Files (current implementation)
- `src/application/app.rs`
  - App bootstrap and runtime orchestration.
- `src/application/app_model.rs`
  - App state model (what the app “knows” at runtime).
- `src/application/appview.rs`
  - Presentation/controller glue for views, view switching, and interaction.
- `src/application/render_app.rs`
  - Rendering-facing app wrapper / integration for the render loop.
- `src/application/gl_canvas.rs`
  - Canvas / surface integration details (notably relevant for wasm targets).
- `src/application/mod.rs`
  - Module exports.

## Interactions
- Rendering is implemented under `src/rendering/**` and is called/owned by the app runtime.
- Data loading/decoding is handled under `src/data/**`.
- Acquisition/protocol integration (native or wasm) is under `src/acquisition/**`.

## Native vs WASM considerations
- WASM builds must avoid async runtimes that are not wasm-friendly.
- Logging should route to browser console for wasm (see project conventions).