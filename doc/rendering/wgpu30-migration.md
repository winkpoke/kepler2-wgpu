# wgpu 30 / Winit 0.30 caller migration

**Status:** Current implementation reference. Native/browser runtime acceptance remains
open in [the upgrade verification record](../../openspec/changes/upgrade-wgpu-winit/verification.md).
Dependency versions/features come from `Cargo.toml` and `Cargo.lock`.

## Rust rendering errors and pass lifetimes

`Renderable::render`, layout/view rendering, `AppView::render_pass`, and
`App::render` now return `KeplerResult<()>`, not `Result<(), wgpu::SurfaceError>`.
The success value is still unit. `App::render` remains a successful no-op for a
zero-sized surface. Recoverable acquisition skips return `KeplerError::Surface`;
view/pass errors return graphics errors and preserve the failing pass context.

Custom renderers use the existing public module imports:

```rust
use kepler_wgpu::core::error::KeplerResult;
use kepler_wgpu::Renderable;

struct CustomRenderer;
impl Renderable for CustomRenderer {
    fn update(&mut self, _queue: &wgpu::Queue) {}
    fn render(&mut self, _pass: &mut wgpu::RenderPass<'_>) -> KeplerResult<()> {
        Ok(())
    }
}
```

`KeplerError::Surface` contains the owned `SurfaceFailure` with `kind`,
`operation`, and `message`. Replace backend-error matches/conversions with
`SurfaceFailureKind` matches and `From<SurfaceFailure>` (or `failure.into()`).
Classification includes timeout, occluded, outdated, lost, validation, out of
memory, device lost, and other failures. `KeplerError::Graphics` remains separate;
do not reinterpret it as surface loss. Owned textual surface failures have no
backend error source; I/O errors retain their source. WASM error conversion keeps
operation/context in its string.

```rust
use kepler_wgpu::core::error::{KeplerError, SurfaceFailureKind};

fn is_timeout(error: &KeplerError) -> bool {
    matches!(error, KeplerError::Surface(failure) if failure.kind == SurfaceFailureKind::Timeout)
}
```

The public `PassContext` now takes **two lifetimes**:

```rust
use kepler_wgpu::core::error::KeplerResult;
use kepler_wgpu::rendering::core::PassContext;
use kepler_wgpu::Renderable;

fn draw(view: &mut dyn Renderable, context: PassContext<'_, '_>) -> KeplerResult<()> {
    view.render(context.pass)
}
```

`PassContext<'pass, 'encoder>` separates the short mutable borrow from the
encoder guard. Explicit old annotations require the second lifetime argument;
inferred callback arguments remain usable. No unsafe casts or forgotten encoder
lifetime are needed. Neither backend frame textures nor private scheduling outcomes
are a new public error/result API.

## Frames, recovery, and validation

The internal transaction acquires, draws every visible view, submits, and calls
**`Queue::present` exactly once**, then acknowledges presentation. Handoff is not
confirmation of physical monitor display. Uploading uniforms does not acknowledge
a frame. The public unit wrapper retains suboptimal reconfiguration in `App` so
its next nonzero frame reconfigures before acquiring.

- Success presents a complete frame. Suboptimal presents once and schedules
  compatible reconfiguration; subsequent mutations remain pending.
- Timeout skips and retries at the existing bounded 16 ms cadence.
- Occluded acquisition without an OS hide signal uses a bounded probe. Known
  hidden, suspended, or zero-size surfaces wait without acquisition.
- Outdated reconfigures; healthy-device surface loss recreates using the retained
  instance, selected adapter/backend, and existing window/canvas. Neither case
  rebuilds same-device/same-format pipelines. Format changes use existing rebinding.
- Validation or failed drawing stops automatic frame recovery. Device callbacks
  report typed faults and wake the idle owner through the existing quit transport;
  out-of-memory/device-loss faults exit. Restart the viewer (fresh browser page).
  There is no general device reconstruction or cross-API retry on a bound canvas.

The synchronous recovery path checks uncaptured faults around configuration.
Browser validation can arrive asynchronously; its callback still wakes/stops the
owner rather than requiring an animation tick. Initial configuration and headless
tests retain and explicitly pop validation scopes. `push_error_scope` now returns
an `ErrorScopeGuard`: keep it through protected encoding/submission and inspect
`guard.pop()`; merely dropping it discards the captured error. Scope guards must
not move across Tokio workers. GPU tests inspect `Device::poll(PollType::...)`
and mapping results instead of ignoring failures.

Surface selection still prefers the first supported non-sRGB format, falling back
to the first supported format. `SurfaceColorSpace::Auto` retains historical SDR
behavior. There is no added gamma/HDR transform or change to [signed voxel storage,
filtering, or window/level](volume-encoding.md).

Native window instances retain the window as their display-handle provider.
wgpu 30 requires that display during GLES instance creation for Linux window
presentation, including the `KEPLER_WGPU_BACKEND=gl` override on Wayland. Browser
and headless instances continue to initialize without a native display handle.

Browser WGSL enforces derivative uniformity more strictly than native Naga.
MPR samples before its bounds return; MIP derives explicit sampling LOD from
uniform ray-footprint derivatives before misses/loops. Do not suppress the
uniformity diagnostic or force LOD zero: preserve the existing nearest
minification and linear magnification filters. The explicit LOD also avoids
wgpu/Naga 30's Metal 3D-gradient lowering failure. Browser shader compilation is
component evidence, not acceptance of the full viewer lifecycle or every backend.

## Constructor, controls, and window lifecycle

The caller sequence is unchanged: await `get_render_app`, obtain `get_glcanvas`,
enqueue data/controls, then call `run`. Construction still creates the initial
window, appends/sizes the browser canvas in `wasm-example`, and awaits graphics;
startup errors are returned at construction. Previously acquired handles retain
the same proxy and pre-run events remain on the same Winit transport.

Exactly one private bootstrap helper uses the deprecated pre-run
`EventLoop::create_window`, with a scoped expectation explaining GPU-ready
construction. This is a compatibility bridge, not recommended new Winit lifecycle
practice. Native execution uses `run_app`; browser execution uses `spawn_app` and
returns normally to JavaScript. One internal `ApplicationHandler` owns initialized
state and delegates controls/window events, resume/suspend, and `about_to_wait`.
Repeated resume reuses the existing window/device. Quit/close prevents further
scheduling. Browser replacement windows use `ActiveEventLoop::create_window` and
retain `spawn_local` / `GraphicsReady` routing. Replaced-window events are filtered.

Existing unmatched `SetOneCellLayout` and shader reload/invalidation placeholders
are not implemented by this migration. Desktop/browser bootstrap, replacement,
getter delivery, and visual acceptance require separate runtime checks; there is
no new mobile lifecycle support. Follow [BUILD.md](../agents/BUILD.md) and
[the synthetic browser procedure](../testing/browser_gpu.md); a WASM build or
native offscreen pixel test is not browser/window acceptance.
