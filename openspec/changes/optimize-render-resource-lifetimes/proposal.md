# Change: Optimize render resource lifetimes and idle redraw scheduling

## Why

Rendering currently puts reusable GPU objects into shorter lifetimes than necessary. Mesh frames allocate a new texture pool, allocate an unused offscreen color texture, and recreate depth views. MPR view creation and GPU rebinding construct duplicate immutable contexts. The event loop requests another redraw on every redraw, including when every visible view is static.

These changes address resource churn and idle CPU/GPU work without changing shaders, medical image sampling, or upgrading wgpu.

## What Changes

- Keep a depth-only `MeshTexturePool` in `GraphicsContext`, scoped to its device, rather than constructing it inside `App::render`.
- Reuse the depth texture **and its existing view** using an explicit `(width, height, sample_count, depth_format)` key. Do not allocate for zero-sized surfaces. Replace resources on key changes and discard them on graphics replacement.
- Remove the unused mesh offscreen color allocation and its obsolete internal helpers after confirming repository callers.
- Give `DefaultViewFactory` a lazily initialized shared `Arc<MprRenderContext>` for its device and attachment format. Use that same context for both creation methods and bulk GPU rebinding. Keep uniform buffers and bind groups view-specific; keep the already shared volume sampler in `RenderContent`.
- Add visual invalidation state and animation awareness. Stop requesting redraws unconditionally. Upload dirty view state only when necessary, but redraw all visible views into each acquired surface image.
- Invalidate on initial display, visual mutations, linked slice changes, layout changes/clearing, resize, surface recovery, and GPU rebinding. Preserve pending work after failed frames and avoid rendering while minimized.
- Preserve existing native/WASM control interfaces and image output. Keep wgpu 23 and dependency versions unchanged.
- Replace documentation recommending per-frame texture pools with the device-scoped caching policy.

## Review Findings

1. **Depth churn is confirmed.** `src/application/app.rs:304` constructs `MeshTexturePool::new()` inside `App::render`. The render pass already receives a mutable pool, so persistence does not require moving it into the `Arc<BasicMeshContext>` or introducing locking.
2. **Unused color allocation is confirmed.** `src/rendering/core/render_pass.rs:563` calls `get_mesh_views` and discards its color result. `src/rendering/view/mesh/mesh_texture_pool.rs:101` allocates that color texture. The actual mesh color attachment is `frame_view`.
3. **MPR duplication is confirmed.** Both MPR factory paths construct a new `MprRenderContext`; `MprView::rebuild_gpu_resources` does so again for each view. The existing `Arc<MprRenderContext>`/per-view GPU implementation split already provides the required sharing boundary. The sampler is owned by `RenderContent`, not independently created per MPR view when content is shared.
4. **Idle redraws are confirmed.** `src/application/render_app.rs:540` unconditionally requests the next redraw before checking surface readiness. `App::update` then updates every view, including unchanged MPR uniform buffers.
5. **Per-view draw skipping needs adjustment.** The renderer draws directly to an acquired surface image. Clean view pixels are not a retained backing store. This change suppresses whole idle frames and unnecessary uniform uploads, not individual draws within a newly presented frame. Per-view cached output/compositing is a separate future optimization.
6. **Documentation conflicts with the desired policy.** `doc/agents/RENDERING.md` and `doc/agents/PITFALLS.md` currently recommend per-frame pools; update them alongside the implementation.

## Impact

- Affected specs: `rendering`, `application`.
- Affected code: `src/rendering/core/graphics.rs`, `src/rendering/core/render_pass.rs`, `src/rendering/view/mesh/mesh_texture_pool.rs`, `src/rendering/view/view_factory.rs`, MPR/MIP/mesh view invalidation/update implementations, `src/rendering/view/view.rs`, `src/rendering/view/layout.rs`, `src/application/app.rs`, `src/application/appview.rs`, `src/application/render_app.rs`.
- Affected tests: resource-lifetime and redraw-state regressions; existing headless image correctness and device/format rebinding tests.
- No wgpu migration, shader algorithm changes, offscreen composition architecture, or unrelated MIP glam refactor.
- Pending `refactor-mip-view-glam` work touches MIP state; keep invalidation edits narrow. Coordinate rendering test additions with `implement-test-coverage-plan` without changing its checklist.

## Approval

Approved by the user on 2026-10-01. Rendering implementation and automated verification are complete; see `verification.md`. Interactive native/browser smoke checks remain outstanding, and the change has not been archived.
