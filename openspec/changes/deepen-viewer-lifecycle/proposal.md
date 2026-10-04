# Proposal

## Why

After concentrating view-layout transitions, Winit dispatch still coordinates redraw acknowledgement, readiness, complete frames, scheduling, graphics replacement, and shutdown. Existing deterministic tests exercise frame and scheduling policy separately, leaving the live calling sequence uncovered.

## What Changes

- Concentrate existing lifecycle ordering in a private module used by Winit dispatch and a recording test adapter; retain the existing frame transaction and redraw policy.
- Preserve successful and failed graphics replacement ordering, active-window routing, pending work, bounded retries, and the distinction between fatal rendering stop and explicit shutdown.
- Follow with the smaller internal AppView rebinding refactor: select a compatible factory and rebuild existing views in one operation, removing the caller's factory-before-rebuild requirement.
- Add production-interface regressions and current ownership/testing documentation. Record native and browser checks separately.

## Capabilities

### New Capabilities

None. This is a behavior-preserving internal refactor; `skip_specs: true` avoids inventing a new observable requirement.

### Modified Capabilities

None. Baseline application/rendering/testing requirements and the approved lifecycle/resource contracts in the active WGPU migration and resource-lifetime changes remain authoritative. Their unperformed runtime checks remain open.

## Impact

Affected implementation: `render_app.rs`, a private lifecycle module, `App`/`AppView` GPU rebinding, and associated tests. Public native/browser controls, bootstrap, ownership, dependencies, shaders, voxel encoding, and medical semantics remain unchanged.

Acceptance includes deterministic production lifecycle sequencing, real GPU rebinding/state/pixel regressions, locked native/WASM builds and tests, packaging, scoped/full quality gates, docs, maintenance tests, and a synthetic native smoke check. Browser runtime acceptance remains separate and must stay pending if browser control or upload permission prevents execution.

Non-goals: redesign bootstrap or queued controls, new device-loss recovery, mesh-filter/cache repair, dependency updates, performance claims, or live acquisition. Rollback restores the previous private sequencing and rebinding callers without data migration or public compatibility changes.
