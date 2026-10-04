# Proposal

## Why

The live view-layout transition is spread across `App::set_render_mode`, data preparation, and destructive `AppView` layout helpers. A failed upload or view creation can leave the displayed layout or related state partially changed, while several transition tests exercise simulations rather than this production sequence.

## What Changes

- Concentrate the complete operation in a private view-layout transition module, reached through the existing `App::set_render_mode` control path.
- Prepare upload content, candidate mesh, orientation selections, and replacement views before committing layout or related application state. Returned preparation failures preserve the previous layout, interaction state, cache, orientation selections, and pending presentation state, and produce contextual errors.
- Preserve existing reset behavior when a successful transition recreates views. Keep explicit state-capture/restore helpers and CPU-state preservation during GPU rebinding compatible.
- Reject a transition without a loaded volume before changing the layout; do not add a deferred request. This applies the accepted failure policy and corrects the current misleading deferred-load message.
- Exercise successful modes and returned failures through the same production seam, using real headless rendering and narrow failure adapters. Replace detached workflow simulations once equivalent production checks exist.
- Explicitly reconcile the testing spec's unconditional grid/single restoration scenario with the reset policy accepted by the user on 2026-10-04. The application spec's preservation and active-switching language will distinguish successful recreation from failure and GPU rebinding.

## Capabilities

### New Capabilities

None. View-layout transitions belong to the existing application capability.

### Modified Capabilities

- `application`: define transactional view-layout transitions, existing control compatibility, successful reset behavior, and preservation on returned preparation failure.
- `testing`: require production-seam transition verification and reconcile the layout-switch restoration scenario with the accepted state policy.

## Impact

- Likely source scope: `src/application/app.rs`, `appview.rs`, a private transition module, `app_model.rs` only if needed for read-only preparation, and a minimal internal complete-view replacement operation in `src/rendering/view/layout.rs`.
- Verification scope: production transition tests, existing AppView tests, and existing headless rendering/resource/encoding/redraw suites. Preserve public `ViewManager` behavior; application transitions will not be rerouted through that dormant module.
- Preserve native/browser control names, argument ordering, units, and return shapes; use existing `KeplerResult` errors and logging. No dependency or public-interface expansion is planned.
- Non-goals: restored interaction state on successful switches; mesh cache keys or invalidation; mesh extraction, crop geometry, empty-mesh fallback, voxel encoding, shaders, GPU rebinding ownership, lifecycle dispatch, general GPU recovery, volume-load transaction redesign, or deferred mode requests.
- Preserve existing valid mode semantics, including modes other than 0/1/2 selecting the mixed layout and mesh replacing MIP when both request the same slot. Reject unusable slot/orientation indices and missing required mesh before commit with a returned error.
- Acceptance: failed preparation leaves the old viewer observable state and rendering intact; success produces the expected views, positions, defaults, and pending presentation. Checks use synthetic data and keep native/browser runtime acceptance distinct from headless evidence.
- Rollback: revert the scoped transition implementation and its matching contract/docs change together. No persisted data, voxel, dependency, or caller migration is required. Existing unrelated work and pending migration acceptance remain separate.
