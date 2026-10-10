# Implementation Tasks

Approved by the user on 2026-10-01. Implementation and automated verification are complete;
interactive smoke checks remain outstanding. See `verification.md` for evidence and quality-gate limitations.

## 1. Approval and regression baseline

- [x] 1.1 Obtain approval of the proposal and direct-surface redraw design before implementation.
- [x] 1.2 Run the existing headless rendering/volume encoding baseline and record results.
- [x] 1.3 Add failing regressions for repeated-frame depth reuse/cache invalidation, MPR context sharing, and idle/redraw state transitions before the corresponding implementation steps.

## 2. Persistent mesh depth resources

- [x] 2.1 Make `GraphicsContext` own the persistent `MeshTexturePool`; remove the frame-local pool from `App::render`.
- [x] 2.2 Introduce a depth-only cache keyed by width, height, sample count, and depth format; handle zero-sized requests without allocation.
- [x] 2.3 Borrow the cached depth view in `PassExecutor`; remove unused offscreen color allocation and audit obsolete helper API compatibility.
- [x] 2.4 Verify unchanged-frame reuse, resize/format/sample-count replacement, graphics replacement invalidation, and bounded retention/release after mesh removal.

## 3. Shared MPR contexts

- [x] 3.1 Lazily cache one `Arc<MprRenderContext>` in each compatible factory and reuse it from both MPR creation paths.
- [x] 3.2 Reuse a single replacement context across application bulk rebinding, preserving CPU state and separate per-view uniforms/bind groups.
- [x] 3.3 Keep volume sampler sharing in `RenderContent`; preserve pipeline resources across ordinary resize and compatible configuration refreshes.
- [x] 3.4 Verify exact context sharing, independent per-view updates, and replacement on incompatible surface formats/devices.

## 4. Visual invalidation and redraw scheduling

- [x] 4.1 Add dirty/animation state to views and frame-level layout/surface invalidation; audit all visible mutations, linked slice changes, restore paths, and native/WASM control APIs.
- [x] 4.2 Avoid uniform recomputation/uploads for unchanged views, without clearing pending presentation work prematurely.
- [x] 4.3 Replace unconditional redraw requests with coalesced event-driven scheduling and bounded active animation/retry/recovery deadlines.
- [x] 4.4 Handle initial configured surfaces, OS redraws, layout clearing, resize, minimize/restore, unocclusion, graphics replacement, and surface errors.
- [x] 4.5 Draw all visible views on every acquired surface frame; mark presentation work clean only after success.
- [x] 4.6 Verify idle suppression, no-op/read-only controls, mutations, multi-view preservation, animation start/stop, zero-size behavior, and failed-frame retries.

## 5. Documentation and verification

- [x] 5.1 Correct per-frame texture pool guidance in `doc/agents/RENDERING.md` and `doc/agents/PITFALLS.md`; document shared contexts and redraw policy.
- [x] 5.2 Run formatting, native type checks, clippy, focused regressions, and the existing headless image/device/format tests; report pre-existing failures separately.
- [x] 5.3 Run the full native suite and a WASM target check when available; document unavailable environments and unrun checks.
- [x] 5.4 Measure scoped coverage for new cache/invalidation logic where tooling is available; do not infer coverage from test counts.
- [ ] 5.5 Perform native/browser interaction, idle, exposure, and animation smoke checks when interactive access is available; report allocation/submission evidence without claiming unmeasured timing gains. Automated evidence is recorded; manual checks were not performed.
- [x] 5.6 Review the diff for dependency/shader migration creep, validate this OpenSpec change strictly, and update task status to reflect actual completion.
