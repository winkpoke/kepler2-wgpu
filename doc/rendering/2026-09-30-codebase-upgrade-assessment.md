# Kepler2 codebase and dependency upgrade assessment

Date: 2026-09-30. Baseline: commit `603bb96`; initial working tree clean.

## Recommendation

Modernize the rendering stack in separate, reviewable changes: establish useful rendering regressions and a consistent Rust toolchain, migrate winit to **0.30.13**, migrate wgpu to **30.0.1**, then enable browser WebGPU with an explicit WebGL fallback. Keep DICOM/server dependency migrations separate.

wgpu 27 can be a temporary compilation checkpoint if the direct migration becomes difficult. It need not be the shipping destination. The existing architecture roadmap names 27, but current upstream releases and APIs warrant updating that roadmap. No dependency or implementation changes were made in this assessment.

The application already separates graphics initialization, render passes, view creation, and volume preparation. These are useful migration seams; a wholesale renderer rewrite is unnecessary. `Arc<RenderContent>` already permits views to share a volume texture, and the safe `create_surface(window.clone())` ownership pattern is already present.

## Scope and evidence

Inspected the manifest and lockfile, application lifecycle/state, graphics initialization, pipeline helpers, render passes, texture uploads, MPR/MIP shaders and contexts, mesh resource management, volume encoding, DICOM loading, relevant tests, CI, and active OpenSpec changes. Source inventory: 69 Rust files, 18,580 lines, with wgpu referenced in 22 files. This is a targeted source assessment, not an exhaustive correctness audit of every algorithm.

- `cargo check --locked --all-targets`: **passed**, with 83 warnings on installed `rustc 1.100.0-nightly (2026-08-24)`.
- `cargo test --locked --lib`: **38 passed**.
- Initial checks could not unpack `adler2 2.0.0` into the Cargo registry because of filesystem permissions. A rerun with approved cache access succeeded. The initial failure was not a source/build failure.
- Full integration tests, browser tests, GPU image readback, and performance measurements were **not run**.
- `wasm32-unknown-unknown` is not installed on this host, so WASM compilation remains unverified.
- Existing OpenSpec changes: `implement-test-coverage-plan` and `refactor-mip-view-glam`. Coordinate with these instead of duplicating their scope. The old coverage figure in the proposal is historical; coverage was not measured here.

## Dependency targets

| Component | Current lockfile | Suggested action |
|---|---|---|
| wgpu | 23.0.1 | Target 30.0.1; update both manifest declarations together |
| winit | 0.29.15 | Target stable 0.30.13 in its own lifecycle change |
| raw-window-handle | 0.5.2 and 0.6.2 | Remove explicit `rwh_05` after checking consumers; use 0.6 for the new stack |
| wasm-bindgen | 0.2.100 | Resolve compatible web bindings with the wgpu migration; match the wasm-bindgen CLI to the resolved crate |
| glam | 0.30.9 | Keep during renderer migration; no demonstrated reason for a simultaneous major API change |
| DICOM crates | dicom-object 0.8.2 | Assess separately with parsing/rescale/transfer-syntax fixtures |
| Server stack | axum 0.7.9, tower-http 0.5.2; tower 0.4.13 and 0.5.3 | Assess axum/tower/tower-http as a coupled group in a later change |
| Rust | No repository toolchain pin or package `rust-version`; library has nightly feature gate | Prefer stable by using existing timing extension helpers; pin a tested compiler and declare the project MSRV |

Current wgpu 30.0.1 is the latest stable upstream release; it includes Metal and WebGPU fixes relevant to the supported platforms. winit 0.30.13 is the stable target; avoid adding a beta lifecycle migration to this work. Sources: [wgpu 30.0.1 release](https://github.com/gfx-rs/wgpu/releases/tag/v30.0.1), [winit releases](https://github.com/rust-windowing/winit/releases).

These are proposed targets, not proven compatibility results. Full transitive resolution and platform builds are still required. wgpu 30 declares Rust 1.87 and winit 0.30 declares Rust 1.70; the complete application's MSRV must be resolved and tested separately. See the companion [dependency/API research](/Users/philchen/Projects/2026/kepler2-wgpu/doc/research/2026-09-30-graphics-dependency-upgrade.md) for versioned upstream references and exact descriptor changes.

## Highest-value code improvements

### 1. Repair and specify volume encoding before using it as an upgrade oracle

`src/application/app_model.rs:35` converts packed data with `(*x + offset as i16) as u16`, using a bias of 1100. `src/rendering/view/view_factory.rs` duplicates this encoding. MPR/MIP shaders decode the unsigned value and subtract that bias.

An arithmetic reproduction of the actual conversion/decode contract shows:

| Input | Packed RG8 decoded value | R16Float displayed value |
|---:|---:|---:|
| -1200 | 64336 | -1200 |
| -1024 | -1024 | -1024 |
| 2049 | 2049 | 2048 |
| 3001 | 3001 | 3000 |

Values below -1100 wrap in the unsigned packed path. At the upper end, `32000 + 1100` overflows the intermediate `i16` and can panic when overflow checks are enabled. R16Float also cannot preserve every integer in the full i16 range; that is a display precision tradeoff that needs an explicit tolerance.

Use one encoding function, widened arithmetic, and a documented supported range. A full-i16 packing contract can use `i32(value) + 32768`, then convert to u16 and decode with bias 32768. Validate the chosen contract with endpoint and interpolation tests. Preserve original voxel values for exact readouts and measurements. These examples establish arithmetic behavior, not observed patient-image artifacts.

### 2. Make render resources belong to the current graphics context

`src/rendering/core/pipeline.rs:38` stores surface format in a process-global `OnceCell`. Later calls in `App::swap_graphics` cannot change it. MPR context creation and mesh pipeline helpers read that global, while MIP creation receives an explicit format.

If a new surface negotiates a different format, those MPR/mesh pipelines can retain the first surface's target format. Pass the format explicitly through the existing factory/context interfaces. On a device replacement, rebuild all resources belonging to that device, recompute texture capabilities, and attach a generation ID to asynchronous `GraphicsReady` messages so obsolete initialization cannot replace a newer context.

Also share `MprRenderContext` per device/format: each factory call currently constructs a new context and pipeline despite its description as shared. Preserve separate per-view uniforms.

### 3. Retain the depth pool and remove unused color allocation

`src/application/app.rs:284` constructs `MeshTexturePool::new()` every frame. The mesh pass calls `get_mesh_views`, which creates an offscreen color texture even though rendering goes directly to the surface and the returned color view is discarded. `App::resize` also creates a depth texture/view and immediately drops it.

Move the pool into graphics-owned state; reuse it until size, device, format, or sample count changes. Give the direct mesh pass a depth-only accessor. If retaining offscreen textures, include format in their invalidation key. This removes confirmed allocation churn; the FPS improvement must be measured.

### 4. Improve render scheduling and error classification

`src/application/render_app.rs:535` requests another redraw unconditionally. Static MPR/MIP layouts therefore keep updating uniforms and rendering while idle. Schedule redraws on state changes, resize, new data, recovery, and active animation; retain continuous redraw for animation.

`src/application/app.rs:349` translates every pass-execution failure to `SurfaceError::Lost`. This can turn a persistent renderer problem into repeated surface reconfiguration. Distinguish frame acquisition outcomes from rendering/validation errors and device loss. Make the ready/configured state reflect successful initialization and zero-size suspension, instead of relying on receipt of a resize event.

### 5. Build regressions that exercise the renderer

`tests/rendering_correctness_tests.rs` contains **14 empty, ignored tests**. Many GPU-safety tests construct CPU volumes or assert arithmetic rather than performing uploads. Existing native library tests are useful, but do not demonstrate GPU image correctness.

Before switching APIs, add a small synthetic-volume test that creates the real production pipelines, renders offscreen, and reads pixels back. Cover MPR orientations/oblique transforms, MIP/MinIP/AvgIP, window/level boundaries, both volume encodings, mesh depth, and surface/device recreation. Parse/validate production WGSL with a Naga version aligned to the selected wgpu stack. A dedicated GPU or software-adapter CI job should report unavailable adapters explicitly rather than silently treating the test as passing.

### 6. Make browser capabilities explicit

`src/rendering/core/graphics.rs:134` forces `Backends::GL` on WASM. Updating the crate alone will leave this browser path on WebGL. Treat WebGPU selection as a separate behavior change after migration: try browser WebGPU, then deliberately initialize the WebGL fallback if necessary.

The browser device request hard-codes `max_texture_dimension_3d: 1024`; the upload helpers do not validate dimensions or exact byte length before allocation. Negotiate limits against the selected adapter and the actual volume; validate nonzero dimensions, checked voxel-count multiplication, byte count, texture format/usage, and device limits before upload. Return a useful error or provide an explicitly labeled display preview if the volume exceeds capabilities.

### 7. Reduce full-volume copying and consolidate data conversion

`CTVolume` derives `Clone` over a `Vec<i16>`. GLCanvas event dispatch and `App::load_render_content` clone whole volumes, then preparation creates intermediate u16 and byte vectors. For a 512-cubed volume, each complete i16 buffer is 256 MiB; this is a size calculation, not a measured peak-memory figure.

Use shared immutable voxel storage behind the public wrapper, consolidate conversion, and encode/upload once per volume/device/encoding combination. Keep the existing shared `RenderContent` path. Separately, DICOM loading parses the same file buffer into patient/study/series/image objects repeatedly and spawns one task per file; parse once and bound concurrency when profiling confirms loading costs.

### 8. Repair toolchain and CI consistency

`src/lib.rs:1` requires `duration_millis_float`, while coverage jobs select stable Rust and release jobs select nightly. `src/core/timing.rs` already provides stable-compatible `DurationExt` methods. Remove the avoidable nightly dependency by consistently using those helpers, then verify stable rather than assuming it builds.

Coverage gates currently scrape textual output repeatedly and use the same aggregate number for both medical and overall coverage. They do not actually select medical files; the fallback string `0%` is also unsuitable for numeric comparison. Generate coverage once, parse its machine-readable output, and calculate the documented file subsets separately. Add locked native checks/tests and WASM compilation to pull-request CI; run browser and GPU regressions as explicit jobs. Pin the Rust toolchain/MSRV and use supported action versions, with compatible Linux build prerequisites.

## Migration sequence and estimated effort

Estimates are engineering judgments for one developer familiar with the project. They overlap and exclude unavailable platform/hardware time; they are not measured schedules.

1. **Baseline and focused fixes: 2–4 days.** Stable toolchain consistency, encoding regression, upload preflight, useful offscreen smoke tests, and a recorded native/browser image baseline. Keep resource fixes in separate commits.
2. **winit 0.29 → 0.30.13: 2–4 days.** Move window creation into the application lifecycle, implement `ApplicationHandler`, handle resume/suspend/resize/exit explicitly, and preserve GLCanvas event dispatch. Keep browser initialization asynchronous and use the web spawning API appropriate to the new event loop. This isolates lifecycle behavior before changing frame acquisition/presentation.
3. **wgpu 23 → 30.0.1: 3–5 days.** Update both Cargo entries and lockfile; migrate adapter/device requests, texture-copy types, descriptor fields, bind-group/vertex slot representation, acquisition outcomes, and queue-based presentation. Use the migrated window layer for current display-handle ownership. Use 27 only as a temporary checkpoint if needed.
4. **Browser WebGPU plus fallback: 2–4 days.** Capability negotiation, labeled failures, canvas replacement, and browser smoke tests. Preserve grayscale behavior while testing the new surface color-space configuration.

Budget roughly **two to three working weeks** for the combined migration and cross-platform validation, with scope extending if existing image baselines reveal defects. The wgpu-only change is medium effort; the native/WASM lifecycle work carries the greater behavioral risk.

wgpu APIs changed more than once between these releases. In particular, do not blindly accumulate intermediate release edits: current 30.0.1 documentation is the destination contract. Follow current definitions for `CurrentSurfaceTexture`, `Queue::present`, `TexelCopyTextureInfo`, `TexelCopyBufferLayout`, `DeviceDescriptor`, `PipelineLayoutDescriptor`, and `RenderPassDescriptor`. Sources: [wgpu 30 API](https://docs.rs/wgpu/30.0.1/wgpu/), [wgpu 30 release changes](https://github.com/gfx-rs/wgpu/releases/tag/v30.0.0), [wgpu 29 release changes](https://github.com/gfx-rs/wgpu/releases/tag/v29.0.0), [winit ApplicationHandler](https://docs.rs/winit/0.30.13/winit/application/trait.ApplicationHandler.html).

## Acceptance criteria

- Native checks/tests and a release build pass on a pinned stable toolchain; project MSRV is checked separately.
- WASM build passes with matching bindings/CLI tooling.
- Known synthetic-volume outputs agree with CPU reference values within explicit per-encoding tolerances, before and after migration.
- Metal, Windows DX12, and at least one browser WebGPU/WebGL configuration receive runtime checks; supported Vulkan/Linux behavior is verified too.
- Resize, minimize/restore, canvas replacement, unavailable adapter, oversized volume, and device/surface recovery behave predictably.
- Grayscale/surface-format changes are tested deliberately; an API upgrade is not evidence of calibrated display accuracy.
- Static-view idle activity, interaction frame-time distribution, and peak memory are measured against the baseline. No FPS improvement is claimed from source inspection alone.

The repository's OpenSpec workflow requires an approved proposal before implementing a breaking API or lifecycle change. This document is the assessment for that proposal; it is not approval or an implementation plan marked complete.
