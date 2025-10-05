# Render Architecture Design: Passes, Pipelines, Layout-View, Resources, Performance

This document defines a modular, extensible architecture to manage render passes, pipelines, and their relationships with layout and views. It preserves separation of concerns, keeps existing 2D MPR behavior intact, and provides a clear path to add 3D meshes without breaking changes.

References in your codebase:
- Views and view system: <mcfile name="view.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\view.rs"></mcfile> <mcfile name="mod.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\mod.rs"></mcfile> <mcfile name="layout.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\layout.rs"></mcfile>
- Render loop and app orchestration: <mcfile name="render_app.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\render_app.rs"></mcfile>
- Render context for 2D: <mcfile name="render_context.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\render_context.rs"></mcfile>
- Math utilities: <mcfile name="coord.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\coord.rs"></mcfile>
- 2D slice shader: <mcfile name="shader_tex.wgsl" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\shader\\shader_tex.wgsl"></mcfile>

---

1) Render pass structure and organization
- PassGraph abstraction with two node types:
  - OffscreenPassNode: Encapsulates attachments (color/depth), clear/load ops, and a set of pipeline submissions. Used for 3D mesh and post-processing passes.
  - OnscreenPassNode: Targets the swapchain with configurable load/clear and submissions. Used for 2D slice rendering and UI overlays.
- PassRegistry owns pass descriptors and builds a per-frame PassPlan:
  - Static pass descriptors (name, attachments, intended pipelines, ordering).
  - PassPlan constructed at frame start based on active views in the Layout and their capabilities.
- Recommended default pass plan:
  - MeshPass (Offscreen with depth): Executes only if any MeshView exists and is enabled.
  - SlicePass (Onscreen without depth): Always executes; renders 2D MPR views.
  - OverlayPass (Onscreen with alpha blending, optional depth read): Executes if overlays are present; keeps separation from slice pass.

2) Pipeline configuration and state management
- PipelineManager centralizes pipeline creation and caching:
  - Keyed by PipelineDescriptor (shader modules, vertex layout, blend, depth state, cull, topology, multisampling).
  - Pipelines created lazily on first use and cached for reuse across frames.
  - Separate pipeline namespaces for 2D slices and 3D meshes to avoid config cross-talk.
- Standardize bind group layout and uniform buffers per pipeline class:
  - SlicePipeline: texture 3D + sampler, per-view uniforms (matrices, window/level), no depth, alpha blend optional for overlays.
  - MeshPipeline: vertex uniforms (model/view/projection), lighting/material buffers, depth enabled, backface culling, MSAA optional.
- Dynamic state management:
  - Minimize pipeline churn: prefer dynamic uniform updates and bind group reuse over pipeline re-creation.
  - Use a small set of well-defined PipelineDescriptors; sort draw submissions by pipeline to reduce state transitions.

3) Layout-view relationships and data flow
- Preserve the View trait and let each view produce a DrawList for the frame:
  - Each View returns a DrawList containing DrawItems (mesh draws or slice draws) tagged with a target PassId and PipelineId.
  - DrawItem encapsulates geometry handles or slice quads, bind groups, and per-draw uniforms (or instance ranges).
- Data flow per frame:
  - Layout collects active views and requests their DrawLists.
  - RenderApp builds a PassPlan from the union of DrawItems’ PassIds.
  - RenderApp submits DrawItems into their respective passes; within a pass, submissions are grouped and sorted by PipelineId to minimize state switches.
- Separation of concerns:
  - Views manage content and per-view uniforms.
  - Layout manages composition and visibility.
  - Renderer (RenderApp) orchestrates passes and pipelines, without inspecting domain details of views.
  - RenderContext remains per-view for 2D; add MeshRenderContext for mesh views, both producing DrawItems with shared format.

4) Resource management and synchronization
- ResourceManager for textures, buffers, and bind groups:
  - TexturePool: caches depth attachments and offscreen color targets by size/format to avoid reallocation.
  - BufferArena: manages uniform and vertex/index buffers with suballocation; supports dynamic offsets where applicable.
  - BindGroupCache: caches bind groups keyed by resource views + layout; reduces redundant allocation.
- Synchronization and lifetime:
  - Frame-local allocators reset each frame; long-lived resources (volume textures, mesh vertex buffers) held by handles referenced in DrawItems.
  - Command encoding:
    - Prepare per-view uniforms and bind groups before pass encoding.
    - Encode passes in PassPlan order; submit single command buffer per frame where possible.
  - Avoid mapping conflicts: use staging buffers or queue.write_buffer for small uniform updates; never map buffers in use.
- Attachment management:
  - Depth resources created by TexturePool on demand; shared across mesh draws in the mesh pass.
  - Onscreen pass uses swapchain view; overlays read depth only if enabled, otherwise depthless for maximum throughput.

5) Performance optimization strategies
- Draw sorting and batching:
  - Sort DrawItems within each pass by PipelineId and BindGroup layout to minimize pipeline and bind group changes.
  - Batch instanced draws for repetitive meshes; for 2D slices, batch quads that share texture + shader parameters when feasible.
- Culling and LOD:
  - Implement frustum culling in MeshView to skip off-screen meshes.
  - Optional LOD selection for distant or small meshes; reduces vertex workload.
- Pipeline reuse:
  - Use PipelineManager caching; avoid rebuilding pipelines except when descriptor changes.
  - Shared shader modules; keep a small set of PipelineDescriptors tuned for common cases.
- Resource reuse:
  - TexturePool to reuse depth textures across frames; avoid reallocations on window resize by rounding to buckets.
  - BindGroupCache to reuse bind groups across frames if underlying resources didn’t change.
- Efficient uniforms:
  - Prefer struct-of-arrays uniform buffers for frequently updated values.
  - Use dynamic offsets when many DrawItems share the same layout, reducing bind group count.
- Asynchronous preparation:
  - Generate meshes (e.g., marching cubes) off the main thread; upload on the render thread only when ready.
  - Defer large resource uploads to avoid blocking the render loop; use staging and double buffering.

---

Incremental adoption plan (non-breaking)
- Phase 1: Introduce PipelineManager and PassRegistry as internal utilities; keep current 2D slice path intact. RenderApp continues to call existing 2D code but internally routes through PassPlan and PipelineManager without changing public APIs.
- Phase 2: Adapt existing 2D MPR views to emit DrawLists instead of direct command encoding. RenderContext translates DrawList into pass submissions. Visual output remains identical.
- Phase 3: Add MeshRenderContext and MeshView that produce DrawItems routed to MeshPass. Keep MeshPass disabled by configuration until explicitly enabled.
- Phase 4: Add OverlayPass as a separate path when overlays are present; SlicePass remains unchanged otherwise.
- Phase 5: Integrate ResourceManager pools and caches; replace ad hoc resource creation with pooled resources behind the same interfaces.

Component boundaries summary
- Layout: Chooses visible views and provides ordering; no render API coupling beyond gathering DrawLists from views.
- View: Owns content-specific logic and per-view uniforms; produces DrawItems tagged with PassId/PipelineId; no direct command encoding.
- RenderApp: Builds PassPlan, coordinates execution, and submits DrawItems in pass order; no domain knowledge of view internals.
- PipelineManager: Owns pipeline creation/caching; views never create pipelines directly.
- PassRegistry: Defines available passes and builds per-frame PassPlan; enforces ordering and attachment lifetimes.
- ResourceManager: Centralizes textures, buffers, and bind groups; ensures efficient reuse and safe synchronization.