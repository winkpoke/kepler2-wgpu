# Change: Overlay Dual Orthogonal MPR Slices in a Single Canvas

## Why
The current dual orthogonal MPR behavior renders planes in separate regions, which reduces direct anatomical correlation at the same pixel location. Clinical workflows for cross-plane interpretation benefit from seeing axial context as the base image while simultaneously overlaying sagittal information in the same canvas.

## What Changes
- Modify dual orthogonal MPR rendering to use a single full-canvas composition path.
- Render axial slice as the primary base layer.
- Blend sagittal slice on top of axial using transparency (fixed default and optional user-defined opacity control).
- Keep existing transfer function and window/level application active for both planes.
- Avoid introducing multi-viewport or split-region rendering for this mode.
- Preserve existing pipeline architecture by prioritizing minimal shader and uniform changes.
- Add performance guardrails to avoid unnecessary extra passes or CPU-GPU synchronization.

## Impact
- Affected specs: `rendering`
- Affected code: `src/rendering/view/mpr/mpr_view.rs`, `src/rendering/view/mpr/mpr_view_wgpu_impl.rs`, `src/rendering/shaders/shader_tex.wgsl`, `src/application/appview.rs`, `src/application/app.rs`

