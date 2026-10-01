# Proposal

## Why

Kepler's WASM initialization forces WebGL even though the locked wgpu 23.0.1 already supports browser WebGPU and adapter-based detection. Prefer WebGPU without a dependency migration, while keeping WebGL as the compatibility path and making capability decisions visible.

## What Changes

- Enable `webgpu` and `webgl` explicitly for WASM, retaining wgpu 23, winit 0.29, and the existing lockfile.
- Use wgpu's asynchronous WebGPU detection; attempt WebGL when WebGPU detection or pre-surface adapter/device negotiation fails.
- Finish backend negotiation before acquiring the application's canvas context. Once that canvas is bound, surface/pipeline failures return actionable errors rather than attempting an invalid API switch on the same canvas.
- Centralize selected-device volume capabilities in graphics initialization, including backend, filtered/uploadable volume formats, and effective maximum 3D texture dimension. Use them for initialization, graphics replacement, and format toggles.
- Request supported browser limits instead of requiring a fixed 1024-voxel 3D texture dimension. Reject oversized volume uploads with dimensions and limits, without changing voxels.
- Log selected backend, adapter, negotiated limits, volume encoding, and fallback reasons using the existing logger.

Non-goals: dependency upgrades, shader algorithms or uniform changes, changes to signed encoding/window-level/orientation, device-loss recovery, post-canvas backend switching, performance claims, new browser automation infrastructure, or live acquisition.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rendering`: add browser backend selection and selected-device volume capability requirements; existing rendering requirements remain unchanged.

## Impact

Affected areas: `Cargo.toml`, `src/rendering/core/graphics.rs` and internal capability helpers, `src/application/app.rs`, volume upload validation in `src/rendering/view/render_content.rs`, focused tests, rendering/build guidance, and changelog. Preserve existing native backend overrides and public constructors/control APIs. No dependency version or lockfile change is intended.

Acceptance: native and WASM-library locked builds; deterministic capability and fallback-policy regressions; existing native volume/pixel/resource regressions; manual synthetic-volume browser checks for WebGPU enabled, unavailable, and forced WebGL. Browser checks remain open until actually performed. No native test can certify browser parity.

Compatibility/rollback: retain an explicit browser WebGL-only diagnostic mode, selected before initialization, without relying on browser environment variables. Reverting the backend preference must not require reverting capability centralization or signed encoding fixes. The user approved the backend-only scope on 2026-10-01; consequential canvas/limit behavior is detailed in the design for review before implementation.
