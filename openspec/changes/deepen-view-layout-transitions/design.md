# Design

## Context

See [proposal.md](proposal.md) for motivation and the [application](specs/application/spec.md) and [testing](specs/testing/spec.md) deltas for intended behavior. The user accepted successful recreation with existing defaults and preservation of the displayed layout on returned preparation failure on 2026-10-04.

The live operation is `App::set_render_mode` in [app.rs](../../../src/application/app.rs). It uploads volume content, selects or generates mesh content, updates orientation selections and the mesh-mode flag, and calls destructive layout helpers in [appview.rs](../../../src/application/appview.rs). Those helpers clear or resize the old layout before all replacement views exist. The public `ViewManager` is outside this live path.

`AppView` owns the concrete `DefaultViewFactory` and `DynamicLayout`. The factory's content constructors take an immutable borrow and can construct candidate views without changing the displayed collection. `DynamicLayout` owns the parent dimensions, strategy, views, and presentation tracking. Its existing strategy-changing operation immediately resizes old views, so it cannot be used during preparation. Factory constructors currently return success after GPU construction; returned-error coverage therefore needs narrow failure adapters in the production sequence. Asynchronous fatal GPU faults follow a different existing path.

The baseline testing spec requires restoration across grid/single switches, whereas production recreation currently resets interaction settings. The deltas explicitly reconcile that conflict with the accepted reset policy. CPU-state preservation during GPU rebinding remains a separate contract. [CONTEXT.md](../../../CONTEXT.md) distinguishes view interaction state from view-slot orientation.

## Goals / Non-Goals

**Goals:** Give one private module responsibility for preparation order, failure reporting, and committing all related viewer state. Allow tests to drive that same operation without creating a native window. Keep application state and graphics resources with their current owners.

**Non-Goals:** Introduce a second viewer-state owner, adopt the dormant `ViewManager`, or generalize the rendering hierarchy. This operation does not make broader volume loading transactional, repair mesh-cache identity, restore successful-switch interaction state, or recover a lost GPU device. The remaining exclusions are listed in the proposal.

## Decisions

### 1. Keep existing controls; concentrate their operation in one private module

Add a private `application/view_layout_transition.rs` module. `App::set_render_mode` translates the legacy arguments into an internal request and invokes the operation. It keeps its current public signature and return shape, logging a contextual `KeplerResult` failure through the existing crate logging conventions. Native and browser controls continue through this path.

The module borrows the state it needs from `AppModel`, `AppView`, and the application-owned mesh cache and orientation selections. It does not store another copy of committed state. `AppModel` remains the canonical volume owner; `AppView` retains its layout, concrete factory, and explicit state-capture/restore helpers. The operation prepares rendering content from read-only volume access rather than calling the load helper that first replaces the model's volume.

Moving only individual helper calls would leave the ordering policy spread across callers. Routing through `ViewManager` would require changing a different ownership model and would not deepen the existing live path. Both alternatives add work without providing the required locality.

### 2. Prepare a bounded replacement, then commit without fallible work

The operation has three phases:

1. Validate prerequisites and indices actually used by the request, and determine the target layout policy. Missing volume, unusable selection, or unavailable required mesh returns before committed state changes.
2. Prepare uploaded content, any newly generated mesh, prospective orientation selections and mesh-mode flag, and the complete replacement view collection. All fallible work stays here. Reuse existing mesh content under the existing explicit reuse policy when applicable.
3. Commit the prepared views, strategy choice, orientation selections, applicable mesh cache, and mesh-mode configuration in one synchronous operation. No upload, view construction, or validation remains after commit begins.

Preparation owns one candidate collection and any newly created resources. A returned error drops that candidate; old views are neither removed nor resized, and old presentation tracking remains intact. Compatible, lazily initialized factory caches may remain populated because they do not change displayed or application state. Successful commit invalidates the layout for a complete frame through existing redraw tracking; it must not mark a frame presented.

This is atomic with respect to the synchronous control operation and its observable committed state. It does not promise rollback from a panic, asynchronous validation fault, or device loss. Catching those failures or adding recovery machinery would change a separate graphics contract. Restoring copied state after destructive changes was rejected because views and GPU resources would already have been lost or changed.

### 3. Keep layout placement and presentation ownership in `DynamicLayout`

Add only the crate-private support needed to size and replace a complete candidate collection using the current parent dimensions and either the retained strategy or a supplied target strategy. Placement must use the real layout parent dimensions, including the last positive dimensions retained during minimized-window handling.

When reusing a strategy, borrow it to compute prospective placements; arbitrary `LayoutStrategy` objects need not be cloned. When changing a strategy, prepare the new strategy separately. Only incoming views may be resized before commit. A successful replacement updates the complete collection and relevant strategy and invalidates presentation once, without first resizing the old collection.

Single MPR, single MIP, and single mesh use the current one-cell layout semantics. A mixed request from a one-cell layout uses the existing two-by-two grid with spacing 2; a mixed request from another existing strategy retains that strategy as the current code does. Always replacing the strategy with a new grid would silently change custom-strategy behavior.

### 4. Preserve valid legacy behavior and make failed requests explicit

Keep orientation index order, all mode values other than 0/1/2 selecting mixed layout, and mesh precedence when mesh and MIP target the same slot. Preserve the current generation trigger, explicit cache-reuse behavior, crop/ISO calculations, mesh-mode flag semantics, and empty-mesh fallback. A mesh request can still trigger preparation in modes that do not display it. This change makes those existing updates conditional on successful preparation; it does not redesign their meaning.

Validate only indices used for an orientation lookup or a four-slot selection. Unused legacy arguments must not become newly restrictive. A single mesh request with neither usable cached mesh nor requested generation returns a contextual error instead of reaching the current panic.

Without a volume, reject before mutation and log the missing prerequisite. The current deferred-load message has no stored request behind it; the proposed contract does not add a queue or change subsequent volume-loading layout policy. This consequence is explicit in the proposal for review.

Successful recreation uses existing factory defaults. Retain public explicit capture/restore helpers and GPU rebinding behavior. Removing those helpers or applying captured state automatically would change separate callers or reverse the user's accepted choice.

### 5. Put failure adapters at preparation seams, not in a duplicate workflow

The transition module accepts internal, borrowed preparation adapters for upload and view construction. Production adapters invoke the current graphics upload and concrete factory. Test adapters inject a returned upload error or decorate real factory calls to fail at a chosen construction step. Use the existing `ViewFactory` interface where it fits; otherwise use a narrow internal constructor adapter. Keep `AppView`'s concrete factory ownership for compatible MPR sharing and GPU rebinding.

Tests borrow real application state and drive the production operation with synthetic volumes and headless graphics. They do not need `winit` or a test-local transition algorithm. Avoid converting the entire application factory to a public trait object merely to inject failures.

Success coverage checks MPR, MIP, mesh, and mixed arrangements, orientation, strategy, positions and dimensions, recreated defaults, cache policy, overlap precedence, and pending presentation. Failure coverage checks real malformed-upload validation and injected failure after candidate views and related state have been prepared. Snapshot old settings, positions, view identities, cache/orientation/flag state, source volume, and both clean and already-pending redraw states. Render synthetic frames before and after a failed transition to verify that the old displayed content remains usable.

Replace detached simulations only after equivalent production checks exist. Existing rendering correctness, signed-volume encoding, resource lifetime, redraw, and GPU-rebinding regressions remain active. Native and browser smoke checks use existing controls and are recorded separately from headless evidence; this work does not claim to validate concurrent view mutation or driver-level memory freedom.

### 6. Keep platform and graphics contracts stable

The private operation introduces no dependency or runtime and compiles with the native application and WASM library. It uses existing domain errors. Upload bytes, signed voxel precision, `glam` geometry, uniform layout, shaders, and attachment-format/device resource ownership are unchanged. Error context identifies the failed operation and applicable numeric dimensions or limits without copying patient metadata into logs.

## Risks / Trade-offs

- **Temporary GPU memory increase:** old and candidate resources coexist until commit or failure. Keep one candidate, release it promptly on failure, and retain bounded-resource regressions. No memory or performance improvement is claimed without measurement.
- **Previously hidden partial updates become failures:** validate used selections and missing prerequisites before preparation; retain valid legacy argument and mode semantics and cover them through the production seam.
- **Factory errors are uncommon in current constructors:** injected returned errors establish operation ordering, while real uploads and headless rendering establish integration. They do not establish recovery from fatal GPU faults.
- **Existing mesh-cache policy can reuse stale content:** retain and document that policy; add no implicit invalidation in this scoped refactor.
- **A successful replacement could accidentally preserve old interaction settings:** assert existing defaults after modified old settings, and keep GPU rebinding checks separate.
- **Platform or full quality gates may fail for unrelated reasons:** report actual failures and manual gaps without suppressing tests or changing unrelated work.

## Migration Plan

1. Present these artifacts for approval of the concrete ownership and prepare/commit design, including missing-volume rejection. Planning does not implement the operation.
2. Add production-seam regression coverage and the private operation, then route existing controls through it. Keep public helpers compatible and retain legacy semantics.
3. Run the native, WASM-library, formatting, lint, application, and required GPU checks in [BUILD.md](../../../doc/agents/BUILD.md). Complete or explicitly report native/browser synthetic runtime checks.
4. Update current architecture/testing documentation and the user-visible failure-handling entry in [CHANGELOG.md](../../../doc/CHANGELOG.md). Reconcile the deltas into baseline specs only through the approved completion workflow.
5. Roll back scoped code and corresponding contract/docs together if acceptance fails. No persisted data or caller migration is required. Preserve unrelated pending changes.
