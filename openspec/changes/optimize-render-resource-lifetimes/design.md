# Render Resource Lifetime Design

## Context

`PassExecutor` renders mesh/MIP/MPR content directly to the surface. Its mesh pass needs a full-surface depth attachment, not a per-viewport attachment. The executor already accepts a mutable texture pool, but `App::render` creates it afresh every frame. `GraphicsContext` owns both the device and the executor and is replaced by `App::swap_graphics`.

MPR resources are already separated into immutable `MprRenderContext` and per-view `MprViewWgpuImpl`, but factory and rebind paths do not share context instances. Volume content and its sampler can already be shared through `Arc<RenderContent>`.

The event loop currently drives itself through unconditional `request_redraw`. Visual changes arrive through native window events and `UserEvent` control APIs; mesh animation is an exception to static rendering. Current surface recovery and mesh fallback handling must remain functional after idle scheduling is introduced.

## Goals / Non-Goals

Goals:
- No repeated texture/view allocation for unchanged mesh depth attachment keys.
- No unused mesh color allocation.
- One immutable MPR context per active factory/device/format configuration, shared across creation and bulk rebind paths.
- No self-scheduled idle frames or unchanged uniform uploads.
- Preserve image output, CPU view state, animation, and native/WASM event-driven controls.

Non-goals:
- Upgrade wgpu or change dependency versions.
- Cache view output textures or introduce a compositor.
- Change shader algorithms, projection math, mesh animation math, or medical image sampling.
- Implement currently placeholder shader reload behavior or unrelated refactors.
- Guarantee a measured FPS improvement without profiling evidence.

## Decisions

### 1. Scope the mesh depth pool to GraphicsContext

Add a persistent pool to `GraphicsContext` and pass its mutable reference to `PassExecutor`. This is the narrowest existing owner that has the correct device lifetime and surface-sized attachment semantics. It avoids a mutable pool inside an `Arc<BasicMeshContext>`, interior locking, or an unnecessary renderer hierarchy rewrite.

Use a depth-only cache entry containing a key, texture, and view. The key includes width, height, sample count, and depth format. Match all key fields before reusing resources, and borrow the cached view directly when opening the render pass. The production pipeline currently uses one sample; explicit sample-count invalidation does not introduce MSAA. Ignore the surface color format for this depth cache: only the depth attachment format matters.

Zero-sized requests must not create textures. A replacement `GraphicsContext` starts with an empty pool, preventing old-device attachments from reaching the replacement device. Any future in-place device replacement must explicitly invalidate it. Removing mesh views must release or explicitly bound retained depth memory; do not accumulate entries for old sizes.

### 2. Reuse the existing MPR resource boundary

Store a lazy shared `Arc<MprRenderContext>` in `DefaultViewFactory`, rather than a process-global cache. The factory already fixes the device, queue, and target format. Both MPR creation methods clone this Arc. The context already owns the pipeline, layouts, and quad buffers; no second resource wrapper is necessary.

For format/device rebinding, create the replacement factory/context once, then use its context to rebind all existing MPR views. Preserve per-view uniforms and CPU state and adopt replacement volume decoding parameters as today. Retain a standalone single-view rebind path if required by existing public callers/tests, but route application bulk rebinding through the shared-context path.

Do not move or duplicate `RenderContent::sampler`: it is already shared by views with common volume content. Do not share mutable uniform buffers between views. Replacing the factory for device/format changes invalidates its context; ordinary resize must not rebuild pipelines. Configuration refreshes that do not alter pipeline compatibility should preserve the context where applicable.

### 3. Separate visual invalidation from drawing into a fresh surface image

Views track pending visual changes and whether they require animation updates. The implementation compares small CPU-state snapshots against the last upload and successful presentation, so direct configuration edits and linked updates are captured without adding invalidation calls to every setter. Initialization, changes to displayed parameters, and GPU rebinding invalidate views. No-op setters leave clean state clean after validation/clamping. Layout changes additionally invalidate the whole frame, including clearing the final view.

Only dirty or animating views upload/recompute GPU uniforms during an update. A dirty flag used to suppress uniform uploads must not accidentally suppress a required presentation: distinguish pending upload from pending successful frame, or clear visual invalidation only after submission/presentation succeeds.

Whenever a frame is required, render all visible views into the newly acquired surface texture. A surface image is not a retained per-view backing store. Skipping individual clean draws would require offscreen caching and compositing, deliberately outside this change.

### 4. Request redraw only for pending work or active animation

Remove unconditional `request_redraw` from `RedrawRequested`. Use a single scheduling decision after input/user events (for example `AboutToWait`) to coalesce pending visual changes into one redraw. Idle control flow waits for events. Active animation schedules subsequent frames at a bounded cadence; do not introduce an unbounded polling loop.

A delivered OS redraw still draws a complete frame, even when no application parameter changed. Request the initial frame for an already configured nonzero surface; do not wait indefinitely for a resize event. Resize, exposure/unocclusion, surface reconfiguration, and device replacement require a new complete frame. Zero-sized/minimized windows suppress acquisition and automatic redraws until restored, without discarding pending changes.

Errors must not mark pending work clean. Surface lost/outdated errors reconfigure and retry; timeouts retain invalidation and retry at a bounded cadence; out-of-memory exits. Account for scheduled mesh fallback/recovery work using a deadline if needed, rather than permanent idle rendering. Preserve animation time bookkeeping when enabling animation after a long idle period.

Cover every visual control path, including linked MPR slice changes, layout clear/reset, mesh orientation/opacity controls, and both native keyboard and WASM `UserEvent` mutations. Read-only getter events must not schedule frames solely because they occurred.

## Risks / Trade-offs

- Missing an invalidation path freezes visible output → regression tests exercise control APIs, linked updates, layouts, and resource replacement.
- Clearing dirty state before a failed render loses work → clear only after successful presentation and test retries.
- Partial redraw corrupts clean views → always draw all visible views per acquired surface image.
- Sharing a context across incompatible formats/devices causes validation failures → factory-scoped ownership and existing multi-format/two-device headless regressions.
- Retained depth textures use memory while idle → one bounded cache entry; release with graphics owner and on mesh removal as appropriate.
- Event-driven scheduling removes periodic mesh recovery updates → explicitly schedule necessary recovery deadlines without a permanent game loop.
- Public pool helper removal could affect external consumers → verify repository references and document any low-level API change; preserve compatibility where practical without retaining unused allocations.

## Validation

Write regressions before implementation:
- Drive repeated real mesh passes through the same persistent pool and verify unchanged depth texture/view identity or allocation generation, with resize/format/sample-count/zero-size invalidation cases.
- Verify multiple factory-created MPR views share the exact context Arc while uniform buffers remain independent; verify bulk rebinding shares a new compatible context and preserves displayed state.
- Test redraw state transitions: initial frame, idle after success, visual mutation/no-op/read-only controls, linked slice updates, layout clear, animation start/stop, resize/minimize/restore, and failed-frame retry.
- Run headless pixel correctness tests for MPR/MIP/mesh and existing device/format rebinding tests; add a complete-frame multi-view preservation case as needed.
- Run formatting, native checks/tests, clippy, and a WASM target check when installed. Record environment limits and pre-existing failures rather than claiming unrun checks passed.
- Manually verify native/browser idle rendering, interactions, exposure, and animation when an interactive window/browser is available. Report observed allocation/submission counts separately from unmeasured timing improvements.

## Migration Plan

Implement in three independently verifiable steps: persistent depth-only pooling, shared MPR contexts, then invalidation/redraw scheduling. Keep all of them separate from any wgpu upgrade. No data migration is required. Roll back scheduling independently if a missed invalidation is discovered.

## Open Questions

The proposal was approved on 2026-10-01 and implementation is complete. Interactive native/browser smoke checks remain outstanding; see `verification.md` for automated evidence and environment/tooling limitations.
