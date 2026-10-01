# Proposal

## Why

Upgrade the requested graphics/window stack from locked wgpu 23.0.1 and winit 0.29.15 to wgpu 30.0.1 and winit 0.30.13 without changing image interpretation or existing browser integrations. The intervening APIs remove the surface error type used throughout Kepler and change surface presentation and window construction, so a compatibility migration is needed rather than a manifest-only update.

## What Changes

- Update both common and WASM wgpu declarations and the winit declaration; commit a narrowly resolved Cargo lockfile with the requested versions. Retain native backends and both browser WebGPU/WebGL2 features; use raw-window-handle 0.6 compatibility rather than the unused explicit `rwh_05` feature.
- Migrate instance/device requests, surface capabilities/configuration, render and pipeline descriptors, texture copies, polling, validation scopes, and native GPU test helpers to the target APIs. Keep existing surface format preference and historical SDR presentation behavior.
- **BREAKING:** Replace Rust rendering signatures exposing removed `wgpu::SurfaceError` with `KeplerResult` and an application-owned surface failure payload. Retain the unit success shape of public `App::render`; distinguish presentation and skipped/recoverable acquisition internally. Document the migration for custom `Renderable` implementations and callers matching `KeplerError::Surface`. Also separate public `PassContext` borrow/encoder lifetimes (add a second lifetime argument), approved during apply on 2026-10-01 to retain wgpu 30's encoder drop guard.
- Handle all target surface acquisition outcomes deliberately, including suboptimal, occluded, timeout, outdated, lost, and validation cases. Present through the queue; retain dirty state on skipped/failed frames and preserve bounded redraw/recovery scheduling.
- Replace the closure event loop with an internal `ApplicationHandler<UserEvent>`, native `run_app`, and browser `spawn_app`. Preserve the exported async constructor, `get_glcanvas`, `run`, control names/result shapes, pre-run event delivery, and GPU-ready constructor behavior. The design documents a single scoped pre-run window-construction compatibility bridge; active-loop replacement windows use the nondeprecated API.
- Add focused regression coverage for error propagation, acquisition/recovery decisions, and lifecycle/control dispatch; migrate existing tests without weakening signed-value or framebuffer tolerances. Update current migration guidance and the changelog during implementation.

**Non-goals:** new GPU features, renderer/compositor redesign, shader algorithms or uniform-layout changes, new encoding/precision policy, new mobile targets, automatic device-loss recovery, post-binding browser API switching, browser automation infrastructure, unrelated control-handler fixes, toolchain upgrades, or live acquisition/device commands. No performance improvement is claimed.

## Capabilities

### New Capabilities

None; this change extends the existing rendering and application boundaries.

### Modified Capabilities

- `rendering`: add application-owned rendering errors and explicit surface frame outcome handling without weakening existing pixel, matrix, or resource contracts.
- `application`: add startup/control compatibility and lifecycle scheduling requirements for the window event-loop migration.

## Impact

- Dependencies: [Cargo.toml](../../../Cargo.toml), [Cargo.lock](../../../Cargo.lock); review only required transitive changes. The current [compiler pin](../../../rust-toolchain.toml) remains unchanged.
- Rust: [errors](../../../src/core/error.rs), [App rendering](../../../src/application/app.rs), [event loop](../../../src/application/render_app.rs), [library bootstrap](../../../src/lib.rs), graphics initialization, pipeline/pass constructors, view/layout render signatures, texture upload paths, and GPU-backed tests.
- Browser: retain the caller sequence already used by [index_image.html](../../../static/index_image.html) and [index.html](../../../static/index.html), including replacement canvases through `set_window_by_div_id`. No caller migration or generated package files are part of this planning step.
- Compatibility: the user accepted documenting the necessary Rust rendering-error API break on 2026-10-01. This authorizes planning, not implementation; the bootstrap bridge and detailed design still require review before apply.
- Coordination: preserve the implemented behavior of `prefer-browser-webgpu` and `optimize-render-resource-lifetimes`, whose manual acceptance remains open. Do not rewrite their artifacts, mark their tasks complete, or treat their pending deltas as synced baseline requirements.

### Acceptance and rollback

Acceptance requires the locked native/WASM-library build matrix, focused frame/lifecycle regressions, required-adapter rendering/encoding/resource/redraw suites, and independent synthetic-data native/browser smoke checks. Keep manual browser results, unavailable environments, and baseline quality-gate failures separate from automated success; follow [BUILD.md](../../../doc/agents/BUILD.md).

Keep the dependency/API migration and event-loop migration reviewable as separate commits. Roll back the migration commits and Cargo lockfile together to the original stack if acceptance fails; do not roll back the preceding encoding, backend-selection, or resource-lifetime work. No data migration is needed.
