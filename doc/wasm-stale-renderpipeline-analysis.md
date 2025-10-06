Analysis: WASM panic “RenderPipeline does not exist” and PipelineManager invalidation

Summary
- Symptom: On Web (WASM), loading CT volume data a second time results in a panic: “RenderPipeline[Id(0,1)] does not exist.”
- Root cause: Pipelines cached in PipelineManager were created with an old wgpu Device. After canvas/Graphics are recreated, a new Device is used, but stale pipelines are reused, leading wgpu-core to reject the handle.
- Fix: Invalidate PipelineManager cache when Graphics/Device changes so pipelines are rebuilt for the new Device before rendering.

Context
- Graphics can be recreated in the Web path (SetWindowByDivId → async Graphics creation → GraphicsReady). On the second load, Device is different; any RenderPipeline tied to the prior Device becomes invalid.
- RenderContext requests pipelines via PipelineBuilder, which first checks the PipelineManager cache. A cache hit can return stale pipelines if the cache isn’t invalidated on Device swap.

Evidence from codebase
- Event handling and lifecycle:
  - Web path triggers Graphics recreation, followed by GraphicsReady where the new Graphics is swapped into State: <mcfile name="render_app.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\render_app.rs"></mcfile>
- Pipeline creation and caching:
  - PipelineManager provides caching and pipelines retrieval; TextureQuad pipelines are created or fetched here: <mcfile name="pipeline.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\pipeline.rs"></mcfile>
  - PipelineBuilder orchestrates requests and cache hits/misses, delegating to PipelineManager and creation routines: <mcfile name="pipeline_builder.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\pipeline_builder.rs"></mcfile>
- RenderContext pipeline request path:
  - RenderContext determines target format and requests TextureQuad pipeline via PipelineBuilder: <mcfile name="render_context.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\view\render_context.rs"></mcfile>

Root cause explanation
- Cached pipelines are tied to the Device instance they were created with. After recreating Graphics (and Device), those pipelines are no longer valid.
- Without invalidation, a cache hit returns a pipeline handle referring to resources absent from the new Device’s resource store; wgpu-core then panics when this handle is used.

Fix implemented
- Invalidate PipelineManager cache right after swapping in the new Graphics and before loading data on GraphicsReady. This ensures RenderContext’s subsequent pipeline requests rebuild pipelines with the new Device.
- Change location:
  - GraphicsReady event handler (after state.swap_graphics and resize): call pipeline_manager.invalidate_all() and log the action in <mcfile name="render_app.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\render_app.rs"></mcfile>

Verification steps
- Build WASM:
  - wasm-pack build -t web
- Test flow on the web page:
  - Trigger SetWindowByDivId to initialize Graphics.
  - Load CT volume (first time) and render.
  - Repeat the load (second time) and ensure no panic occurs; pipelines should be rebuilt and rendering should proceed.

Recommendations (optional follow-ups)
- Stronger isolation: Instead of only invalidating, replace PipelineManager with a fresh instance on GraphicsReady to guarantee a clean slate for all pipelines.
- Add a Device generation counter in PipelineManager or PipelineBuilder and invalidate on generation change to prevent accidental reuse if invalidation is missed.
- Add resilience in RenderContext: If pipeline creation fails, fall back to a safe no-op or default pipeline to prevent crash, aligning with the resilience goals in the integration plan.

Risks and mitigations
- Risk: If any Arc<wgpu::RenderPipeline> leaks from views prior to invalidation, it might still be referenced. Mitigation: Ensure views/layout are rebuilt (layout.remove_all and view recreation on data load) so new pipelines replace old ones.

Related documents
- Integration plan and current status details: <mcfile name="pipeline-manager-integration-plan.md" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\doc\pipeline-manager-integration-plan.md"></mcfile>

Notes
- This change targets Web (WASM) behavior where canvas/Graphics recreation is common. It also benefits native by ensuring pipeline-device consistency after any Device swap.