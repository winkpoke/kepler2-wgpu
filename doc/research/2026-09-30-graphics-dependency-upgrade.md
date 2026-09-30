# Graphics dependency upgrade research

Verified: 2026-09-30. This is an assessment, not an implemented upgrade. Sources are versioned upstream release notes, crate metadata, and public API documentation. Recheck latest releases before implementation; the verified release dates below precede this assessment date.

## Recommended destination

Target **wgpu 30.0.1 + winit 0.30.13**, after establishing a working native/WASM baseline. The wgpu crate lists 30.0.1 as released on 2026-08-22. Winit lists stable 0.30.13 released on 2026-03-02; 0.31.0-beta.3 is a prerelease. [wgpu crate metadata](https://docs.rs/crate/wgpu/30.0.1), [winit crate metadata](https://docs.rs/crate/winit/0.30.13), [winit releases](https://github.com/rust-windowing/winit/releases).

Wgpu 30.0.1 declares Rust **1.87.0**, while winit 0.30.13 declares Rust **1.70.0**. These are crate minimums, not proof that this complete application resolves/builds on Rust 1.87. Set and test a project toolchain deliberately. Wgpu 29 lowered its MSRV from 27's 1.88 and 28's 1.92 to 1.87; therefore an intermediate 27 target does not inherently lower the compiler requirement. [wgpu manifest](https://docs.rs/crate/wgpu/30.0.1/source/Cargo.toml), [winit manifest](https://docs.rs/crate/winit/0.30.13/source/Cargo.toml), [wgpu 29 release](https://github.com/gfx-rs/wgpu/releases/tag/v29.0.0).

**Assessment:** 27 can be a temporary checkpoint if diagnostics or existing migration work justify splitting the API conversion, but avoid making it the permanent target by default. A direct 30 migration avoids implementing intermediate APIs that were subsequently removed. Winit's stable 0.30 line is preferable to combining this rendering migration with a 0.31 beta adoption.

## Local dependency inventory

Read from the current `Cargo.toml` and `Cargo.lock`:

| Dependency | Manifest constraint | Locked version | Upgrade relationship |
| --- | --- | --- | --- |
| wgpu | 23.0.0; WASM 23.0 with webgl | 23.0.1 | Update both declarations together |
| winit | 0.29.15 with rwh_05 | 0.29.15 | Window lifecycle/API migration |
| naga / wgpu-core / wgpu-hal | Transitive | 23.0.0 / 23.0.1 / 23.0.1 | Resolve with wgpu; no direct pins needed |
| raw-window-handle | Transitive | 0.5.2 and 0.6.2 | Remove unused 0.5 feature if no consumer needs it |
| wasm-bindgen / js-sys / web-sys | 0.2 / 0.3.22 / 0.3 | 0.2.100 / 0.3.77 / 0.3.72 | Resolve and regenerate WASM bindings together |
| glam / image | 0.30 / 0.25.2 | 0.30.9 / 0.25.5 | Separate routine updates from graphics API migration |
| axum / tower / tower-http | 0.7 / 0.4 / 0.5 | 0.7.9 / 0.4.13 and 0.5.3 / 0.5.2 | Separate server migration; duplicated tower deserves review |
| dicom-object / ndarray / thiserror | 0.8.2 / 0.15 / 1.0 | 0.8.2 / 0.15.6 / 1.0.69 and 2.0.18 | Independent scope; DICOM behavior needs dedicated checks |

Wgpu 30's web dependencies require newer wasm-bindgen/js-sys/web-sys than this lockfile. Its published dependency list includes wasm-bindgen ^0.2.127 and js-sys/web-sys ^0.3.104. Regenerate the package with the compatible wasm-bindgen tooling after Cargo resolves the graph. [wgpu crate dependencies](https://docs.rs/crate/wgpu/30.0.1).

Winit 0.30 defaults to `rwh_06`, and wgpu 30 consumes raw-window-handle 0.6.2. The current explicit `rwh_05` enables an additional generation rather than selecting the generation wgpu needs. Keep 0.6 and remove 0.5 only after confirming no other consumer uses it. [winit features](https://docs.rs/crate/winit/0.30.13/source/Cargo.toml), [wgpu dependencies](https://docs.rs/crate/wgpu/30.0.1/source/Cargo.toml).

## Actual migration surface

| Local code | Required destination API / behavior | Primary source |
| --- | --- | --- |
| `src/rendering/core/graphics.rs` instance setup | 24 introduced backend option structs and borrowed descriptors; **30 takes an owned descriptor again**. At 29+, choose explicit instance constructors with/without an owned display handle; winit provides `owned_display_handle()` | [24 release](https://github.com/gfx-rs/wgpu/releases/tag/v24.0.0), [30 Instance API](https://docs.rs/wgpu/30.0.1/wgpu/struct.Instance.html), [29 release](https://github.com/gfx-rs/wgpu/releases/tag/v29.0.0) |
| Same file, adapter/device creation | `request_adapter` returns `Result` at 25+; `request_device` takes only its descriptor. Descriptor includes `trace` and `experimental_features`, in addition to limits/features/memory hints | [25 release](https://github.com/gfx-rs/wgpu/releases/tag/v25.0.0), [30 Adapter](https://docs.rs/wgpu/30.0.1/wgpu/struct.Adapter.html), [DeviceDescriptor](https://docs.rs/wgpu/30.0.1/wgpu/type.DeviceDescriptor.html) |
| `texture.rs`, `view/render_content.rs` uploads | Replace ImageCopyTexture and ImageDataLayout with TexelCopyTextureInfo and TexelCopyBufferLayout | [TexelCopyTextureInfo](https://docs.rs/wgpu/30.0.1/wgpu/type.TexelCopyTextureInfo.html), [TexelCopyBufferLayout](https://docs.rs/wgpu/30.0.1/wgpu/struct.TexelCopyBufferLayout.html) |
| `core/pipeline.rs`, MPR/MIP pipeline creation | Empty push_constant_ranges becomes immediate_size: 0; bind group layout references are wrapped in Some; vertex buffer layouts become optional slots; pipeline multiview becomes multiview_mask | [28 release](https://github.com/gfx-rs/wgpu/releases/tag/v28.0.0), [PipelineLayoutDescriptor](https://docs.rs/wgpu/30.0.1/wgpu/struct.PipelineLayoutDescriptor.html), [30 release](https://github.com/gfx-rs/wgpu/releases/tag/v30.0.0), [RenderPipelineDescriptor](https://docs.rs/wgpu/30.0.1/wgpu/struct.RenderPipelineDescriptor.html) |
| Depth state and render passes | Depth write/compare become Option values; color attachment depth_slice is None for non-3D views; add multiview_mask: None to normal passes | [DepthStencilState](https://docs.rs/wgpu/30.0.1/wgpu/struct.DepthStencilState.html), [RenderPassColorAttachment](https://docs.rs/wgpu/30.0.1/wgpu/struct.RenderPassColorAttachment.html), [RenderPassDescriptor](https://docs.rs/wgpu/30.0.1/wgpu/struct.RenderPassDescriptor.html) |
| `src/application/app.rs:252`, `:360`; render_app error handling | Surface acquisition returns CurrentSurfaceTexture at 29+. Rewrite existing `?` and SurfaceError return/match. Present via Queue::present at 30 | [CurrentSurfaceTexture](https://docs.rs/wgpu/30.0.1/wgpu/enum.CurrentSurfaceTexture.html), [30 release](https://github.com/gfx-rs/wgpu/releases/tag/v30.0.0) |
| Surface configuration and shaders | Add color_space: Auto to preserve prior output behavior. Integer shader I/O needs explicit flat interpolation; check all shaders even though a simple local location/type search found no obvious integer I/O | [30 release](https://github.com/gfx-rs/wgpu/releases/tag/v30.0.0) |

Surface recovery now needs explicit outcomes: skip Timeout/Occluded, reconfigure Outdated, handle Suboptimal with its acquired frame, recreate a lost surface (or device/resources for device loss), and expose Validation through error reporting. [CurrentSurfaceTexture documentation](https://docs.rs/wgpu/30.0.1/wgpu/enum.CurrentSurfaceTexture.html). Treat this as lifecycle work, not just enum spelling changes.

Polling is another possible future integration concern: 25 renamed Maintain to PollType and returns Result; 27 added submission/timeout fields to Wait. No local Maintain usage was found in `src`. [25 release](https://github.com/gfx-rs/wgpu/releases/tag/v25.0.0), [27 release](https://github.com/gfx-rs/wgpu/releases/tag/v27.0.0).

## Window and browser lifecycle

Winit 0.30 replaces WindowBuilder with WindowAttributes, EventLoopWindowTarget with ActiveEventLoop, and recommends ApplicationHandler/run_app over the deprecated closure run. Create windows through the active event loop, usually resumed, and move captured state into a handler. This affects `src/lib.rs` and `src/application/render_app.rs`. [0.30 migration changelog](https://rust-windowing.github.io/winit/winit/changelog/v0_30/index.html).

For browser embedding, use EventLoopExtWebSys::spawn_app: it returns immediately and does not use the exception technique associated with run_app. Keep asynchronous GPU initialization with spawn_local and deliver completion into the app's user-event/state lifecycle. This is a recommendation for the current embedded JS API, based on the upstream web entry-point contract. [winit 0.30.13 web source](https://github.com/rust-windowing/winit/blob/v0.30.13/src/platform/web.rs).

The local WASM graphics branch hard-selects Backends::GL. Upgrading the crate alone therefore will not enable the WebGPU backend. Wgpu supports both browser WebGPU and WebGL2; defaults include webgpu while webgl is opt-in. Explicitly test both and decide a fallback policy in a separate behavior change. [wgpu platform support](https://docs.rs/crate/wgpu/30.0.1), [wgpu feature manifest](https://docs.rs/crate/wgpu/30.0.1/source/Cargo.toml).

## Suggested implementation gates

1. Establish native and WASM builds and record deterministic MPR/MIP/MinIP/AvgIP/mesh outputs on the existing dependency graph.
2. Convert winit lifecycle independently, preserving JS event/API behavior.
3. Migrate wgpu to 30 in a focused change; optional 27 compile checkpoint if needed for diagnosis. Keep color output/backend behavior fixed for comparisons.
4. Verify loading/texture upload, resize/minimize/restore, shader reload, lost surface/device recovery, and Windows/macOS/browser rendering.
5. Introduce browser WebGPU preference with measured compatibility and output comparisons; upgrade server/DICOM dependencies in separately testable work.

These gates are engineering recommendations, not verified runtime results. No dependency or source-code changes were made for this research.
