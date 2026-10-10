# Design

## Context

See [proposal.md](proposal.md) for motivation and scope, and the [rendering](specs/rendering/spec.md) and [application](specs/application/spec.md) deltas for behavior contracts. Design is required because this migration crosses public errors, rendering resources, window ownership, and native/WASM initialization.

Observed source at planning time:

- [Cargo.toml](../../../Cargo.toml) declares wgpu in common and WASM dependencies; [Cargo.lock](../../../Cargo.lock) resolves 23.0.1 and winit 0.29.15. WASM explicitly enables WebGPU and WebGL. Native backend environment overrides include GL as well as the primary backends.
- [get_render_app](../../../src/lib.rs) creates the window, appends/sizes the browser canvas, and awaits `App::new` before returning. [index_image.html](../../../static/index_image.html) and [index.html](../../../static/index.html) enqueue volume data before calling `run`. Moving initialization into `resumed` would change constructor readiness and startup-error timing, not merely replace a builder type.
- [RenderApp](../../../src/application/render_app.rs) moves initialized `App`, proxy, scheduler, clock, and occlusion state into one closure. Browser replacement windows are created inside an active event callback, with async graphics completion sent back through `GraphicsReady`.
- [App::render](../../../src/application/app.rs) uses acquisition `?`, presents the frame, then marks layout and surface state clean. It currently converts every pass-execution failure to surface `Lost`; the new error boundary will remove that misleading classification.
- [Renderable](../../../src/rendering/view/renderable.rs), layout/view implementations, application pass dispatch, and [KeplerError](../../../src/core/error.rs) expose the old backend surface error. The existing `KeplerResult` contract is available for the migration.
- [Graphics](../../../src/rendering/core/graphics.rs) already performs disposable-canvas browser preflight, owns selected-device capabilities, and prefers the first non-sRGB surface format. Its instance is currently a local variable, not retained for surface recreation.
- [RedrawScheduler](../../../src/application/redraw.rs) already uses a deterministic clock and a 16 ms animation/retry interval. [Headless helpers](../../../tests/common/headless_gpu.rs) fail without an adapter and validate production pipelines, but do not exercise real surfaces or Winit.

The user accepted planning the necessary Rust rendering-error API break. Implementation and the consequential choices below still require review. The overlapping backend-selection and resource-lifetime changes have pending manual checks; this proposal does not complete or sync them.

Existing discrepancies are kept visible: `SetOneCellLayout` is declared/sent but has no dispatch arm, and shader reload/invalidation arms remain placeholders. These are not repaired by this dependency migration. The browser procedure currently contains a nightly override, whereas current [BUILD.md](../../../doc/agents/BUILD.md) uses the repository compiler pin; verification for this change follows BUILD.md, not historical commands.

## Goals / Non-Goals

**Goals:** make the upgraded libraries usable behind Kepler-owned error/frame boundaries; keep readiness, control order, rendering output, and resource ownership stable; make surface recovery decisions independently testable; and keep implementation commits and verification evidence attributable to this upgrade.

**Non-Goals:** a new async startup API, an eager hidden browser event loop, mobile lifecycle support, general device-loss recovery, a new compositor or data representation, removal of unrelated baseline warnings, or certification of clinical correctness, driver-memory behavior, or performance targets.

## Decisions

### 1. Resolve the requested stack narrowly, retaining target features

Use wgpu `30.0.1` in both manifest locations and winit `0.30.13`, with the lockfile resolving exactly those requested versions. Preserve the existing semver declaration style and rely on committed locked builds for reproducibility. Enable raw-window-handle 0.6 compatibility and remove the unused explicit `rwh_05` request; retain Winit's existing desktop feature coverage.

Keep both browser wgpu backends and the native backends needed by current environment overrides. Do not combine this migration with a broad `default-features = false` rewrite or optional feature expansion. Update only transitive dependencies required by the two selected packages. Review native and WASM feature graphs and ensure there is one effective wgpu version, without a retained wgpu 23 compatibility dependency.

The existing Rust 1.98.0 pin is retained. Resolve the new dependency graph during apply and verify it against that pin; do not alter the toolchain file to work around a dependency/compiler problem without a new decision.

**Alternatives:** blanket `cargo update` adds unrelated churn; shipping each intervening wgpu major would repeat migration/testing without a user requirement. Neither is chosen. Preserve helper signatures that accept ordinary vertex layouts by adapting to new optional slots internally where possible.

### 2. Keep public error ownership separate from private frame scheduling

Use the existing `KeplerResult<()>` for `Renderable::render`, layout render implementations, and `AppView::render_pass`. Replace `KeplerError::Surface(wgpu::SurfaceError)` with an application-owned `SurfaceFailure` payload, with meaningful classifications and operation context. Replace the backend-specific conversion with a conversion from that owned type. Keep `Display`, error sources where meaningful, and WASM `JsValue` conversion actionable.

Introduce a crate-private frame execution result used by the window owner, conceptually `Presented { reconfigure_pending }` versus `Skipped { recovery_action }`. It carries no public backend-owned texture or status. An internal frame method performs acquisition, complete drawing, submission, and queue presentation. The existing public `App::render` remains a unit-success wrapper returning `KeplerResult<()>`: skipped recoverable acquisition is mapped to an inspectable owned surface failure for standalone Rust callers; the existing zero-size no-op remains successful. Retain post-presentation reconfiguration requests in the application's graphics state as well as the private outcome, so the public unit-result wrapper cannot discard them. A subsequent frame honors that pending reconfiguration before acquisition. The event-loop owner uses the private frame result rather than treating every unit success as a presentation.

Draw/pass failures propagate as actual rendering errors, not synthetic surface loss. Preserve invalidation until submission and presentation handoff succeeds. `Presented` means the API handoff completed; it is not an acknowledgment that a physical monitor displayed the frame.

The user additionally approved separating the public pass context lifetimes on 2026-10-01: `PassContext<'pass, 'encoder>` stores `&'pass mut RenderPass<'encoder>`. wgpu 30's encoder drop guard rejects the previous coupled lifetime. Explicit downstream annotations must use two lifetime arguments; inferred callback arguments retain their shape. Keep the encoder lifetime enforced rather than using unsafe casts or removing its guard.

Document all changed Rust signatures, the pass-context lifetime arguments, and `KeplerError::Surface` payload matching in current migration guidance/changelog. Audit crate exports and a downstream-style custom `Renderable` compile regression; do not expose the private frame enum as another new public API.

**Alternatives:** returning `CurrentSurfaceTexture` publicly leaks backend lifetime/status choices; changing public success to a frame enum adds an unnecessary API break; retaining wgpu 23 only for its error duplicates the graphics stack. All are rejected.

### 3. Map acquisition outcomes to explicit recovery actions

The production acquisition boundary translates target-library statuses once. Test that translation and the consuming scheduling/recovery path using injected outcomes/actions, not a test-only copy of the match expression.

| Acquisition result | Drawing/presentation | Owner action |
| --- | --- | --- |
| Success | Complete frame, submit, queue-present once | Mark presented state clean; retain existing animation scheduling |
| Suboptimal | Usable frame may be fully drawn/presented once | Schedule compatible reconfiguration; do not discard later mutations |
| Timeout | No frame presented | Keep dirty state; use the existing bounded retry deadline |
| Occluded | No frame presented | Retain work; wait while OS-hidden/suspended; permit restore events or a bounded probe when the OS provides no occlusion signal |
| Outdated | No frame presented | Configure a nonzero surface, then schedule retry |
| Lost, healthy device | No frame presented | Recreate/configure the selected-backend surface, then schedule retry |
| Validation | No successful frame | Report operation/backend; stop automatic recovery for this failure |

Retain the original `wgpu::Instance` in `Graphics` for healthy-device surface recreation. Recreate only the surface on its existing window/canvas and backend; retain device resources if device/attachment format remain compatible. On a format change, use existing factory/context rebinding. Never rerun cross-API browser negotiation on an already-bound canvas.

Handle out-of-memory and device-loss notifications separately from acquisition; the old surface out-of-memory arm cannot simply disappear. Report typed fault context and wake the live event-loop owner through the existing quit/shutdown transport so fatal errors are visible even when idle. Do not add another public `UserEvent` variant or exported control merely for this notification. Out-of-memory retains exit behavior; device loss is reported and terminates automatic rendering, with restart/fresh-page guidance rather than general device reconstruction. Acquisition validation failures stop automatic recovery in the frame owner without pretending that a surface resize repaired the fault.

A failed drawing pass or fatal fault must not trigger endless surface recreation. Recovery failure returns an owned error with its stage. Zero-size checks run before configuration/allocation. An occluded acquisition without a matching Winit event must not leave the viewer permanently unable to restore or cause a busy redraw loop. Use the scheduler's existing bounded deadlines and explicit restoration invalidation, not sleeps inside callbacks.

**Alternatives:** collapsing statuses into `Lost` corrupts error meaning and recovery; treating every skipped frame as success loses pending work; rebuilding a device on every surface loss discards compatible caches. None is chosen.

### 4. Adopt ApplicationHandler without changing GPU-ready construction

Move the closure-captured execution state into one internal `ApplicationHandler<UserEvent>` owner. Dispatch implemented user controls in `user_event`, native/window input in `window_event`, redraw coalescing in `about_to_wait`, and availability changes in `resumed`/`suspended`. Retain the scheduler, clock, window-id filtering, mesh recovery deadlines, and proxy transport.

Keep `RenderApp::new`, the async `get_render_app`, `get_glcanvas`, `set_window`, and the exported async `run` facade compatible except for the explicitly documented rendering error signatures. Native `run` consumes the loop into `run_app` and returns after exit; WASM uses `EventLoopExtWebSys::spawn_app` with owned handler state, returning to JavaScript normally. Previously obtained control handles stay attached to the same proxy. No extra event loop or pre-run browser execution is introduced.

The initial window uses `Window::default_attributes` and the still-available pre-run `EventLoop::create_window` behind exactly one private compatibility helper. That call is deprecated in Winit 0.30.13; the proposed bridge uses a narrowly scoped `#[expect(deprecated, reason = ...)]` naming the GPU-ready constructor contract. This is an explicit compatibility trade-off for existing desktop/browser callers, not a module-wide lint allowance or a claim that pre-run construction is the preferred Winit lifecycle.

Initial/resumed callbacks reuse the already initialized window/graphics and invalidate presentation; they do not create duplicates. Browser div-targeted replacement is already an active-loop operation and uses `ActiveEventLoop::create_window` with attributes, existing DOM attachment/sizing, `spawn_local`, and `GraphicsReady`. Maintain active window-id filtering and existing successful replacement/load behavior. Shutdown stops further redraw/retry work.

```text
get_render_app()
  --> create initial window through scoped bootstrap bridge
  --> append/size canvas where applicable
  --> await graphics/App initialization (or return error)
  --> return RenderApp + control proxy

controls enqueue --> run()
                       --> native run_app / browser spawn_app
                       --> one initialized handler
                       --> ordered controls + bounded redraws
```

**Alternatives:** moving all startup to `resumed` changes readiness/error timing and needs an explicitly redesigned API; eager browser spawning before the constructor returns changes pre-run execution and diverges from native startup; retaining deprecated closure `run` spreads compatibility debt through all dispatch. The single construction bridge plus modern dispatch is the narrower migration. If it fails on a supported desktop/browser target, stop and obtain approval for a startup-contract redesign rather than quietly deferring initialization. Remove the bridge only in a separately approved readiness migration.

### 5. Adapt descriptors and test utilities without changing rendering intent

Known target API seams to migrate:

| Existing usage | Target adaptation |
| --- | --- |
| Value-based instance descriptor; adapter `Option` handling | Target constructor borrowing; adapter `Result` with contextual errors |
| Two-argument device request; complete old descriptor literals | One descriptor argument; explicit disabled tracing/experimental defaults and negotiated limits |
| Surface format list/configuration | Target capability representation; preserve first-non-sRGB preference; select `SurfaceColorSpace::Auto` |
| Plain layout entries, push-constant ranges, vertex buffer entries | Optional layout/buffer slots; zero immediate size; no new GPU features |
| Old pass/pipeline literals | Current attachment/depth-slice/multiview fields with existing disabled/default semantics |
| `ImageCopy*`, `ImageDataLayout` | `TexelCopy*` types with unchanged bytes, dimensions, row padding, and decode parameters |
| `Maintain::Wait`, device push/pop error scopes | Checked `PollType` polling and retained `ErrorScopeGuard` with explicit result inspection |
| `SurfaceTexture::present` | Submit first, then owning queue presentation |

Keep a scope guard alive until its protected operations finish and explicitly pop/inspect it; merely dropping the new guard discards captured errors. The guard is not Send/Sync, so keep scope ownership local and do not move it across Tokio workers. Browser async setup continues through `spawn_local`/existing futures; native synchronous GPU tests may block with pollster. Install error handling without competing loggers or native-only runtimes on WASM.

Review changed limit defaults as capabilities, not platform identity. Preserve browser downlevel negotiation with each adapter's supported 3D limit and selected-device format legality. Do not reject formerly usable devices solely because a new default raises an unused requirement, request every adapter maximum, or alter voxels to satisfy a limit. Keep upload rejection before allocation and preserve packed preference on existing graphics-rebinding paths.

Do not rewrite shaders merely because the Naga version changes. Compile production WGSL in the existing GPU suites; any required syntax/validation adjustment must leave algorithms, affine decode, and Rust/WGSL layout intact. In particular, do not add gamma correction, HDR, or wide-gamut output while adopting the new surface color field. Preserve device/format ownership, shared MPR resources, depth-cache keys, independent uniforms, and unchanged-format resize behavior described in current rendering guidance.

**Alternative:** broad use of new defaults without reviewing intent can silently alter required capabilities, precision, or resource behavior. The migration uses compatibility values and existing regression expectations instead.

### 6. Separate automated evidence from native/browser acceptance

Before dependency edits, record the pinned compiler, actual baseline commands, adapter/backend, results, and existing quality-gate failures. New deterministic frame/handler tests cover injected failure decisions, redraw outcomes, unit-success/public errors, control order, repeated resume, and shutdown. Tests must exercise the production seams.

Migrate [headless helpers](../../../tests/common/headless_gpu.rs) and other active setup helpers without silent adapter skips, dropped error-scope results, disabled tests, or loosened expectations. Keep exhaustive packed-i16/filtering tests and the existing one-byte signed grayscale and two-byte format/device rebinding tolerances. Keep layout, cache identity/generation, complete multi-view drawing, and CPU-state preservation regressions.

Apply runs [BUILD.md](../../../doc/agents/BUILD.md)'s native/WASM-library checks, full/library tests, formatting, strict Clippy, active diagnostics on changed Rust paths, required-adapter suites, and locked wasm-pack packaging. Record all statuses rather than asserting quality gates passed because focused tests did. CI's Linux software Vulkan path remains supported; macOS/Windows surface behavior needs its own environment evidence.

Manual acceptance uses synthetic data through public bootstrap/control APIs, not the hard-coded native DICOM inputs or live Remedy page. Independently check native windows and browser automatic WebGPU, unavailable-WebGPU fallback, forced GL, controlled pre-canvas failure, and post-binding failure boundaries. Cover repeat loading/canvas replacement, async getters, all existing view types, signed fixture values, window/level, resize/input, idle, animation, hide/restore, and close. Record exact environment/backend/encoding/limits and limitations. Current browser pages have no numerical framebuffer readback assertion; visual checks are not quantitative parity proof. Do not mark pending manual checks in other changes complete from this proposal.

## Risks / Trade-offs

- Pre-run Winit construction remains deprecated and can be platform-sensitive -> isolate the single bridge, require supported-target smoke evidence, and block release if readiness cannot be preserved; no blanket lint suppression.
- Surface loss, device loss, and pass failure have different repair scopes -> retain the instance, type the production outcomes/fault path, and test action identity as well as eventual redraw.
- Fatal GPU callbacks can arrive while idle -> wake the event-loop owner; do not rely on a future animation tick to notice failure.
- A dropped validation guard can make broken pipelines appear verified -> inspect popped results and retain deferred-submission validation coverage in headless helpers.
- New limits, feature tables, or presentation color management may change output/support -> retain compatibility negotiation, byte/layout contracts, strict pixel bounds, and independent browser checks; stop for a new decision if preservation is not possible.
- Existing shader-reload placeholders and unhandled controls can be mistaken for upgrade regressions -> record the baseline and preserve implemented dispatch; do not silently enlarge scope.
- Pending prior manual checks and unrelated format/Clippy failures can obscure results -> keep per-command evidence and this change's manual tasks distinct; no unsupported performance, coverage, or clinical claims.

## Migration Plan

1. Review this design, including the explicit bootstrap bridge and Rust error migration, and record the pre-upgrade verification baseline.
2. Add focused failing frame/error/lifecycle regressions. Update the dependency graph and wgpu-facing implementation/test utilities in a coherent dependency/API commit.
3. Migrate the event-loop facade/handler and active-window creation in a separate reviewable commit, preserving the existing constructor/control sequence.
4. Run integrated checks and update current API/rendering/browser guidance and changelog with actual behavior and observed limits.
5. Perform independent synthetic native/browser acceptance and keep unavailable/manual work open. Do not archive as completed until agreed acceptance is evidenced.

Rollback restores both migration commits and the matching prior Cargo manifest/lockfile, not the earlier backend/precision/resource changes. No persisted data or voxel migration is required. Generated browser packages are rebuilt, not edited or committed as planning artifacts.

## Upstream Evidence

Consulted on 2026-10-01 via indexed official release/documentation results; direct page retrieval was blocked by this environment's address-safety check. These are API planning evidence, not compilation/runtime evidence. Confirm exact signatures and feature declarations against the resolved target crate sources during apply.

- [wgpu releases](https://github.com/gfx-rs/wgpu/releases): cumulative API migration, including removed surface error and target-version changes.
- [wgpu 30.0.1 surface acquisition](https://docs.rs/wgpu/30.0.1/wgpu/enum.CurrentSurfaceTexture.html), [surface](https://docs.rs/wgpu/30.0.1/wgpu/struct.Surface.html), and [queue](https://docs.rs/wgpu/30.0.1/wgpu/struct.Queue.html): acquisition outcomes, surface recreation, and queue presentation.
- [Pipeline layout](https://docs.rs/wgpu/30.0.1/wgpu/struct.PipelineLayoutDescriptor.html), [vertex state](https://docs.rs/wgpu/30.0.1/wgpu/struct.VertexState.html), and [scope guard](https://docs.rs/wgpu/30.0.1/wgpu/struct.ErrorScopeGuard.html): descriptor and validation ownership changes.
- [wgpu feature inventory](https://docs.rs/crate/wgpu/30.0.1/features): native and browser backend features.
- [Winit 0.30.13 event loop](https://docs.rs/winit/0.30.13/winit/event_loop/struct.EventLoop.html), [application handler](https://docs.rs/winit/0.30.13/winit/application/trait.ApplicationHandler.html), and [browser loop extensions](https://docs.rs/winit/0.30.13/winit/platform/web/trait.EventLoopExtWebSys.html): callback ownership, deprecated pre-run construction, and browser spawning.
