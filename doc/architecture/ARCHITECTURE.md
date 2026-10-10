# Current architecture

**Status:** Current reference, reviewed against `947b4fc` on 2026-10-01.
Dependency versions and features are authoritative in `Cargo.toml` / `Cargo.lock`.
This map describes ownership, not an assertion that Rust enforces strict layer rules.

## Subsystems and source anchors

| Subsystem | Responsibility | Source anchors |
| --- | --- | --- |
| Core | Geometry, coordinate bases, errors, window/level, timing | [core](../../src/core/mod.rs), [Base](../../src/core/coord/base.rs), [geometry](../../src/core/geometry.rs) |
| Data | Patient/study/series/image hierarchy, volume construction, MHA/MHD | [data](../../src/data/mod.rs), [DICOM repository](../../src/data/dicom/dicom_repo.rs), [CTVolume](../../src/data/ct_volume.rs) |
| Rendering | GPU context, pipelines/passes, views, shaders, mesh resources | [graphics](../../src/rendering/core/graphics.rs), [passes](../../src/rendering/core/render_pass.rs), [view factory](../../src/rendering/view/view_factory.rs) |
| Application | Data state, view orchestration, event loop, external controls | [App](../../src/application/app.rs), [AppModel](../../src/application/app_model.rs), [AppView](../../src/application/appview.rs), [RenderApp](../../src/application/render_app.rs) |
| Server (native) | HTTP/WebSocket, volume and mesh export, AI segmentation | [server](../../src/server/mod.rs), [routes](../../src/server/routes.rs) |
| Compute GPU (native) | Independent device owner mounted on ServerState.gpu | [compute](../../src/gpu/mod.rs) |
| Acquisition | Remedy framing/state, retries, native/browser bindings | [protocol](../../src/acquisition/remedy.rs), [reference](../acquisition/remedy-serial-web.md) |

The native binary starts at [main.rs](../../src/main.rs); the library/browser API
and logger start at [lib.rs](../../src/lib.rs). `GLCanvas` sends external controls
through application events. Target guards isolate native runtime/filesystem paths
from browser bindings. Mesh lives under `src/rendering/view/mesh/` and is always
available, not behind a `mesh` Cargo feature.

## State and ownership

- `AppModel` owns application data/configuration, including the canonical volume.
- `AppView` owns view composition, layout, and saved CPU view state.
- `App` coordinates model/view state and graphics, and executes complete frames.
- `RenderApp` starts one initialized `ApplicationHandler` with native `run_app` or
  browser `spawn_app`. The private [viewer lifecycle](../../src/application/viewer_lifecycle.rs)
  owns complete lifecycle ordering and borrows Winit/application effects;
  [redraw.rs](../../src/application/redraw.rs) retains readiness and scheduling policy.
  [frame.rs](../../src/application/frame.rs) owns the private
  presented/skipped transaction; public `App::render` retains unit success.
- GPU faults use a private notifier and the existing quit transport to wake an idle
  owner; no new public control event or general device reconstruction is added.
- `GraphicsContext` owns its device/queue/surface configuration and persistent
  mesh depth texture/view pool. Device resources do not cross graphics owners.
- `DefaultViewFactory` lazily shares immutable MPR resources for compatible
  device/attachment-format configurations; uniforms and bind groups stay per view.
- `RenderContent` holds uploaded view data/resources. The [volume encoding contract](../rendering/volume-encoding.md)
  separates signed voxel storage from medical display parameters.

## View-layout transitions

The existing `App::set_render_mode` controls enter the private
[view-layout transition module](../../src/application/view_layout_transition.rs).
It borrows `AppModel`'s canonical volume, `AppView`'s concrete factory/layout,
and `App`'s mesh cache and view-slot orientations. These owners remain unchanged;
captured CPU interaction state in `AppView` is distinct from orientation selection.

The operation validates used selections, uploads content, prepares any candidate
mesh and all replacement views, then commits the complete collection and related
state. `DynamicLayout::replace_all_views` places incoming views with the retained
parent dimensions and retained or new strategy before replacing displayed views.
Returned preparation failures leave the old views, cache/configuration, volume,
and clean or pending presentation state intact. A successful replacement becomes
pending for a complete frame and uses existing view defaults without automatic
state restoration. Explicit capture/restore and GPU rebinding retain their contracts.

Old and candidate resources coexist temporarily; one candidate is dropped on
failure or committed on success. Compatible lazy factory caches may be populated
during preparation. This operation does not recover panics or asynchronous fatal
GPU faults, redesign volume loading, or add mesh-cache identity/invalidation.
Requests without a volume fail without mutation or deferred application.
Production-seam tests live in [AppView tests](../../src/application/appview_tests.rs).

## Frame and rebinding contracts

Winit dispatch translates lifecycle events into `ViewerLifecycle::handle` after
the existing active-window and input-consumption gate. The operation acknowledges
redraw, checks readiness, updates, executes the existing complete frame, and records
the outcome before later scheduling effects. It also orders replacement swap,
fault-wake attachment, old-request reset, resize, and supplied-volume enqueue.
Failed swaps perform no follow-on effects. The same lifecycle owner retains
suspension and fatal stop across replacement. Fatal frame errors stop automatic
rendering; explicit Quit clears views and exits, while Close/Escape exit without
clearing. Explicit shutdown silences subsequent dispatch.

[Recording lifecycle tests](../../src/application/viewer_lifecycle_tests.rs) cross
the production operation with deterministic time and primitive effects. They cover
sequencing and policy integration, not Winit callback delivery, pre-run control
ordering, or native/browser graphics acceptance.

An acquired surface image is cleared once and all visible views are drawn. Upload
invalidation and successful presentation are separate states; failed frames retain
pending work. Ordinary same-format resize updates layout/size-dependent targets,
not pipelines. A format change rebuilds format-dependent resources. Device
replacement reuploads volume data and rebuilds device-owned resources while
preserving CPU view state. See [rendering contracts](../agents/RENDERING.md).

`AppView::rebind_gpu_resources` accepts a borrowed device/queue/format configuration
and any prepared replacement volume content. It selects or refreshes its compatible
factory before rebuilding all existing views from that factory's shared MPR context.
Callers no longer coordinate factory replacement separately. `App` retains upload
validation and graphics/depth ownership; same-format resize avoids this operation.

Regression anchors: [rendering correctness](../../tests/rendering_correctness_tests.rs),
[resource lifetimes](../../tests/render_resource_lifetime_tests.rs),
[redraw scheduling](../../tests/redraw_state_tests.rs), and
[voxel encoding](../../tests/volume_encoding_tests.rs).

## Boundaries, evidence, and proposed work

Use [repository policy](../../AGENTS.md) for safety and approval, [build procedures](../agents/BUILD.md)
for executable checks, and [baseline specs](../../openspec/specs/) for intended
behavior. Discrepancies between source and requirements require an explicit decision;
the existence of a spec is not proof that every scenario is implemented.

Roadmaps and dependency-upgrade investigations belong in [active changes](../../openspec/changes/)
and [dated research](../research/), not in the current ownership map. Older
`doc/redering/`, view designs, and architecture analyses remain historical references.

## Release viewer features

The viewer retains multi-oblique MPR, shared camera controls, DR-to-PRO volumes,
R8Uint segmentation overlays, DVR, spine/needle meshes, mesh overlays, marker
controls, and OBJ exports. HTTP/WebSocket/CLI and browser bindings are separate
contracts. The native compute owner and viewer renderer do not share devices.
Release guidance snapshots are retained in `doc/archive/v017-guidance/`.
