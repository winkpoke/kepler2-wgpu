# Render Architecture Design: Passes, Pipelines, Layout-View, Resources, Performance

This document defines a modular, extensible architecture to manage render passes, pipelines, and their relationships with layout and views. It preserves separation of concerns, keeps existing 2D MPR behavior intact, and provides a clear path to add 3D meshes without breaking changes.

References in your codebase:

- Views and view system:
- Render loop and app orchestration:
- Render context for 2D:
- Math utilities:
- 2D slice shader:

---

## 1) Render Pass Structure and Organization

- **PassGraph abstraction** with two node types:
    - **OffscreenPassNode**: Encapsulates attachments (color/depth), clear/load ops, and a set of pipeline submissions. Used for 3D mesh and post-processing passes.
    - **OnscreenPassNode**: Targets the swapchain with configurable load/clear and submissions. Used for 2D slice rendering and UI overlays.
- **PassRegistry** owns pass descriptors and builds a per-frame **PassPlan**:
    - Static pass descriptors (name, attachments, intended pipelines, ordering).
    - PassPlan constructed at frame start based on active views in the Layout and their capabilities.
    - Cache PassPlan configurations for common view layouts to reduce construction overhead.
    - Validate attachment compatibility (format, size, multisampling) during PassPlan construction to prevent runtime errors.
    - Support a dependency graph in PassRegistry to handle future multi-pass dependencies (e.g., post-processing).
- **Recommended default pass plan**:
    - **MeshPass** (Offscreen with depth): Executes only if any MeshView exists and is enabled.
    - **SlicePass** (Onscreen without depth): Always executes; renders 2D MPR views.
    - **OverlayPass** (Onscreen with alpha blending, optional depth read): Executes if overlays are present; keeps separation from slice pass.

## 2) Pipeline Configuration and State Management

- **PipelineManager** centralizes pipeline creation and caching:
    - Keyed by **PipelineDescriptor** (shader modules, vertex layout, blend, depth state, cull, topology, multisampling).
    - Pipelines created lazily on first use and cached for reuse across frames.
    - Implement a cap or LRU policy for the cache to manage memory usage.
    - Separate pipeline namespaces for 2D slices and 3D meshes to avoid config cross-talk.
    - Document MSAA compatibility rules in PipelineDescriptor to ensure pipelines sharing a pass have compatible attachments.
- Standardize bind group layout and uniform buffers per pipeline class:
    - **SlicePipeline**: texture 3D + sampler, per-view uniforms (matrices, window/level), no depth, alpha blend optional for overlays.
    - **MeshPipeline**: vertex uniforms (model/view/projection), lighting/material buffers, depth enabled, backface culling, MSAA optional.
- **Dynamic state management**:
    - Minimize pipeline churn: prefer dynamic uniform updates and bind group reuse over pipeline re-creation.
    - Batch uniform updates across views to reduce CPU overhead.
    - Use a small set of well-defined PipelineDescriptors; sort draw submissions by pipeline to reduce state transitions.

## 3) Layout-View Relationships and Data Flow

- Preserve the **View trait** and let each view produce a **DrawList** for the frame:
    - Each View returns a DrawList containing **DrawItems** (mesh draws or slice draws) tagged with a target **PassId** and **PipelineId**.
    - DrawItem encapsulates geometry handles or slice quads, bind groups, and per-draw uniforms (or instance ranges).
    - Cache static DrawItems across frames for unchanged view states to optimize DrawList generation.
- **Data flow per frame**:
    - Layout collects active views and requests their DrawLists.
    - RenderApp builds a PassPlan from the union of DrawItems’ PassIds.
    - RenderApp submits DrawItems into their respective passes; within a pass, submissions are grouped and sorted by PipelineId to minimize state switches.
    - Validate DrawItems in RenderApp to ensure correct PassId/PipelineId tagging, with error logging for debugging.
    - Batch similar DrawItems (e.g., 2D slices sharing textures) to reduce submissions.
- **Separation of concerns**:
    - Views manage content and per-view uniforms.
    - Layout manages composition and visibility.
    - Renderer (RenderApp) orchestrates passes and pipelines, without inspecting domain details of views.
    - RenderContext remains per-view for 2D; add MeshRenderContext for mesh views, both producing DrawItems with shared format.

## 4) Resource Management and Synchronization

- **ResourceManager** for textures, buffers, and bind groups:
    - **TexturePool**: caches depth attachments and offscreen color targets by size/format to avoid reallocation. Use adaptive bucket sizing based on historical resize patterns for memory efficiency.
    - **BufferArena**: manages uniform and vertex/index buffers with suballocation; supports dynamic offsets where applicable.
    - **BindGroupCache**: caches bind groups keyed by resource views + layout; reduces redundant allocation. Monitor hit/miss rates and implement a cleanup policy for stale entries.
- **Synchronization and lifetime**:
    - Frame-local allocators reset each frame; long-lived resources (volume textures, mesh vertex buffers) held by handles referenced in DrawItems.
    - Command encoding:
        - Prepare per-view uniforms and bind groups before pass encoding.
        - Encode passes in PassPlan order; submit single command buffer per frame where possible.
    - Avoid mapping conflicts: use staging buffers or queue.write_buffer for small uniform updates; batch updates into a single transfer per frame.
- **Attachment management**:
    - Depth resources created by TexturePool on demand; shared across mesh draws in the mesh pass.
    - Onscreen pass uses swapchain view; overlays read depth only if enabled, otherwise depthless for maximum throughput.

## 5) Performance Optimization Strategies

- **Draw sorting and batching**:
    - Sort DrawItems within each pass by PipelineId and BindGroup layout to minimize pipeline and bind group changes.
    - Batch instanced draws for repetitive meshes; for 2D slices, batch quads that share texture + shader parameters when feasible. Use heuristics to detect when instancing is beneficial.
- **Culling and LOD**:
    - Implement frustum culling in MeshView to skip off-screen meshes. Consider spatial data structures (e.g., BVH or octree) for dense scenes.
    - Optional LOD selection for distant or small meshes; reduces vertex workload.
- **Pipeline reuse**:
    - Use PipelineManager caching; avoid rebuilding pipelines except when descriptor changes.
    - Shared shader modules; keep a small set of PipelineDescriptors tuned for common cases.
- **Resource reuse**:
    - TexturePool to reuse depth textures across frames; avoid reallocations on window resize by rounding to buckets.
    - BindGroupCache to reuse bind groups across frames if underlying resources didn’t change.
- **Efficient uniforms**:
    - Prefer struct-of-arrays uniform buffers for frequently updated values.
    - Use dynamic offsets when many DrawItems share the same layout, reducing bind group count.
- **Asynchronous preparation**:
    - Generate meshes (e.g., marching cubes) off the main thread; upload on the render thread only when ready. Use double-buffering for async uploads to ensure data consistency.
    - Defer large resource uploads to avoid blocking the render loop; use staging and double buffering.

## Incremental Adoption Plan (Non-Breaking)

- **Phase 1**: Introduce PipelineManager and PassRegistry as internal utilities; keep current 2D slice path intact. RenderApp continues to call existing 2D code but internally routes through PassPlan and PipelineManager without changing public APIs. Maintain a parallel legacy render path for fallback.
  - Goals
    - No public API changes; default behavior and visual output for 2D MPR remain identical to the legacy path.
    - New path is opt-in via a runtime flag (e.g., `RENDER_USE_PASSPLAN=false` by default) and can be toggled at startup or via developer settings.
  - New internal utilities (private modules)
    - PipelineManager
      - Responsibility: centralize creation and caching of render pipelines keyed by `PipelineDescriptor` (shader modules, vertex layout, blend, depth, cull, topology, multisampling).
      - API surface (internal): `get_or_create(descriptor) -> wgpu::RenderPipeline`, `has(descriptor)`, `clear_cache(optional)`.
      - Caching: lazy creation on first use; per-namespace caches for `slice` and `mesh` to prevent configuration cross-talk.
      - Concurrency: internal mutability via `RwLock`/`RefCell` as appropriate; device reference held by RenderApp.
    - PassRegistry
      - Responsibility: define available pass descriptors and construct a per-frame `PassPlan` based on active views.
      - Pass descriptors: `SlicePass` (onscreen, no depth), `OverlayPass` (onscreen, alpha blend, optional depth read), `MeshPass` (offscreen, with depth). MeshPass remains disabled unless mesh feature + runtime flag are enabled.
      - Validation: attachment format/size/MSAA compatibility checked at plan construction time; fall back to legacy path on validation failure to avoid runtime errors.
      - Ordering: `MeshPass` -> `SlicePass` -> `OverlayPass` (when present), matching the recommended default plan.
  - Compatibility shims and routing
    - RenderApp builds a PassPlan when the runtime flag is enabled; otherwise uses the current legacy render flow unchanged.
    - 2D slice rendering continues to encode into the onscreen base pass with no depth attachment; overlays remain unchanged.
    - Mesh remains off by default. When enabled, it renders in an offscreen pass with depth, then composites into the onscreen pass. If disabled, no mesh-related resources or passes are created.
    - Errors or unsupported configurations during PassPlan assembly automatically fall back to the legacy path with a warning log, ensuring uninterrupted rendering.
  - Configuration and observability
    - Runtime flag: `RENDER_USE_PASSPLAN` (bool) with default `false` to keep the legacy path. Optionally expose a CLI/env var.
    - Feature gating: honor existing `mesh` feature flags; Phase 1 does not introduce new public features.
    - Logging: one-time info log indicating whether the PassPlan/PipelineManager path is active for the session.
  - Acceptance criteria
    - Project builds and runs without changing any public API types or function signatures.
    - 2D MPR output is pixel-identical (or within tolerance) to the legacy path across typical view layouts.
    - No `wgpu` validation errors introduced by attachment mismatch or pipeline misconfiguration.
    - Mesh functionality remains disabled unless explicitly enabled; enabling it does not affect 2D-only workloads.
  - Migration steps
    1) Add private modules for `PipelineManager` and `PassRegistry`; wire them into RenderApp behind the runtime flag.
    2) Implement descriptors for `SlicePass` and (disabled-by-default) `MeshPass`; integrate existing texture/depth resources via pooling only when required.
    3) Route existing 2D draw encoding through PassPlan submissions when the flag is enabled; preserve direct legacy calls otherwise.
    4) Add non-invasive checks/tests comparing legacy vs PassPlan outputs in 2D scenarios; ensure a single runtime flag can roll back to the legacy path instantly.
  - Rollback strategy
    - Immediate revert by disabling `RENDER_USE_PASSPLAN` or by forcing the legacy path via a configuration option; no code changes required.
- **Phase 2**: Adapt existing 2D MPR views to emit DrawLists instead of direct command encoding. RenderContext translates DrawList into pass submissions. Visual output remains identical. Add automated tests to compare DrawList-based output against the legacy path.
- **Phase 3**: Add MeshRenderContext and MeshView that produce DrawItems routed to MeshPass. Keep MeshPass disabled by configuration until explicitly enabled. Profile MeshPass impact on frame times, even when disabled, to avoid overhead.
- **Phase 4**: Add OverlayPass as a separate path when overlays are present; SlicePass remains unchanged otherwise.
- **Phase 5**: Integrate ResourceManager pools and caches; replace ad hoc resource creation with pooled resources behind the same interfaces.

## Component Boundaries Summary

- **Layout**: Chooses visible views and provides ordering; no render API coupling beyond gathering DrawLists from views.
- **View**: Owns content-specific logic and per-view uniforms; produces DrawItems tagged with PassId/PipelineId; no direct command encoding.
- **RenderApp**: Builds PassPlan, coordinates execution, and submits DrawItems in pass order; no domain knowledge of view internals.
- **PipelineManager**: Owns pipeline creation/caching; views never create pipelines directly.
- **PassRegistry**: Defines available passes and builds per-frame PassPlan; enforces ordering and attachment lifetimes.
- **ResourceManager**: Centralizes textures, buffers, and bind groups; ensures efficient reuse and safe synchronization.