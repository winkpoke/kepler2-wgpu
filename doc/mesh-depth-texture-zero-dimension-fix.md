WASM Mesh Depth Texture Validation Error: "Dimension X is zero" — Cause Analysis and Fix

Context
- Project: Rust + WGPU, compiled for native and WASM (wasm-pack build -t web)
- Feature: mesh rendering enabled via Cargo feature "mesh"
- Error surfaced only in WASM builds with mesh enabled; native builds worked

Error Description
- During mesh rendering initialization on the web, creating the texture labeled "Mesh Depth Texture" triggered a WebGPU validation error: "Dimension X is zero".
- This occurs when attempting to create a depth texture with zero width (and/or height), which is invalid in WebGPU.

Root Cause
- In browsers, a canvas may start with size 0x0 until CSS/JS sets a non-zero size, especially early in initialization.
- The surface_config.width and surface_config.height can therefore be 0 during State initialization or the first render.
- Our code attempted to create the "Mesh Depth Texture" using surface_config.width/height without guarding against zero dimensions.
- WebGPU strictly disallows textures with zero dimensions, causing the validation failure when the mesh feature tries to enable depth.

Fix Implemented
- Added non-zero dimension guards in both paths that create the mesh depth texture:
  1) Initial creation during State initialization: only create if surface_config.width > 0 and surface_config.height > 0; otherwise skip and log a warning.
  2) Lazy creation in the render() path when the depth texture is missing: only create if the current surface configuration has non-zero width/height; otherwise skip and log a warning.
- With these guards, the depth texture is created as soon as a valid (non-zero) surface size is available (e.g., after the first resize or after the canvas is sized via CSS/JS).

Affected Code Locations
- src/state.rs
  - Guard added near the initial creation block for "Mesh Depth Texture" in State initialization.
  - Guard added in the render() logic that lazily creates the depth texture if missing.

Why This Works
- Prevents invalid texture creation when the surface size is 0x0 in WASM.
- Ensures depth testing is available once the canvas has a valid size without impacting native behavior.
- Maintains correct render pass configuration: the depth attachment is only set when a valid depth_view exists.

Validation
- Rebuilt the WASM package with mesh enabled: wasm-pack build -t web --features mesh completed successfully.
- Native builds and unit tests were already passing; this change is platform-agnostic and consistent for Windows, macOS, and Linux.
- Note: Some unrelated doctest snippets in geometry/dicom modules were previously failing; they do not affect this fix.

How to Test
- On the web, ensure the canvas has a non-zero size before enabling mesh rendering.
  - Practical options: set explicit canvas CSS dimensions or trigger a resize event early that sets a non-zero size.
- Reload the page and observe that the "Dimension X is zero" validation error no longer appears.
- Verify that mesh rendering proceeds with depth testing once a valid surface size is available.

Follow-ups (Optional)
- Add a small bootstrap helper to ensure the canvas is sized (non-zero) before initializing State and enabling mesh.
- Gate mesh initialization until the first non-zero resize event is received.
- Clean up build warnings in native/WASM targets to improve overall code health.