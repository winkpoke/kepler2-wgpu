# PipelineManager Integration Plan — Updated Status and Action Plan

This document reflects the current implementation status of the PipelineManager integration, highlights deviations and issues, and outlines clear next steps to complete Phase 1 and prepare for Phase 2. Objectives remain: stability, performance, and full backward compatibility with existing 2D MPR behavior across native and WASM targets.

## Goals (unchanged)
- Introduce PipelineManager as an internal utility to centralize pipeline creation and set the stage for caching/reuse.
- Maintain current visual output and frame-time characteristics; avoid public API churn.
- Parameterize pipeline color target format to improve portability across OS/GPU combinations.
- Ensure mesh rendering can use an optional depth buffer across native and WebGPU with a portable depth format.

## Current Implementation Status

Completed
- Parameterized color target format in texture-quad pipeline API; creation uses the provided format consistently.
- PipelineManager added to the application orchestrator and instantiated at startup; pipelines are cached and returned as `Arc<wgpu::RenderPipeline>`.
- Global surface/swapchain color format accessor implemented and set during initialization; downstream pipeline creation reads it consistently.
  - Setter/getter live in the pipeline module: <mcfile name="pipeline.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\pipeline.rs"></mcfile>
  - Call sites set the format after surface configuration in initialization and when swapping graphics: <mcfile name="state.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\state.rs"></mcfile>
  - RenderContext obtains target format via the accessor: <mcfile name="render_context.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\view\render_context.rs"></mcfile>
- Centralized creation via helper for TextureQuad; PipelineBuilder retained internally but not used by RenderContext path; cache keys include target format and layout signatures; cache hit/miss counters added.
- Unified pipeline acquisition in view and mesh contexts
  - RenderContext uses `get_or_create_texture_quad_pipeline` via the centralized helper and PipelineManager cache; MeshRenderContext uses `get_or_create_mesh_pipeline`; no direct `PipelineBuilder` usage in these runtime paths.
- Pipelines created during setup (context/view construction), not per frame; rendering reuses pipelines.
- Optional debug logging is gated behind a feature flag for cache hits/misses and builder status.
- Mesh depth support integrated under the `mesh` feature flag:
  - Portable depth format helper added: <mcsymbol name="get_mesh_depth_format" filename="pipeline.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\pipeline.rs" startline="69" type="function"></mcsymbol>
  - Mesh pipeline enables depth-stencil state (write enabled, compare Less): <mcsymbol name="get_or_create_mesh_pipeline" filename="pipeline.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\pipeline.rs" startline="341" type="function"></mcsymbol>
  - Depth texture lifecycle managed via TexturePool during initialize/resize; lazy creation ensures a depth attachment exists when mesh is enabled: <mcfile name="texture_pool.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\mesh\texture_pool.rs"></mcfile> and <mcsymbol name="initialize" filename="state.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\state.rs" startline="252" type="function"></mcsymbol> / <mcsymbol name="resize" filename="state.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\state.rs" startline="363" type="function"></mcsymbol> / <mcsymbol name="render" filename="state.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\state.rs" startline="414" type="function"></mcsymbol>
  - WASM zero-dimension guard: Depth texture creation is skipped when surface size is 0x0; lazy creation occurs once a non-zero size is available to avoid WebGPU validation error (“Dimension X is zero”). See <mcfile name="mesh-depth-texture-zero-dimension-fix.md" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\doc\mesh-depth-texture-zero-dimension-fix.md"></mcfile>.
- Native and WASM builds succeed (wasm-pack with `-t web`).
- Mesh pipeline helper configured for `TriangleList`; builds verified; native and WASM visual verification pending.

Partial / Deviations
- RenderContext texture-quad path already uses the centralized helper; MeshRenderContext now also uses the centralized mesh helper (no direct PipelineBuilder usage). Signatures accept a `PipelineManager` reference where needed.
- A typed `PipelineKey` already exists (originally planned for Phase 2); acceptable but should be documented/stabilized.
- Doctest snippets in geometry/dicom modules have been stabilized by gating environment-dependent examples with `rust,ignore`; unit tests continue to pass.

## Resolved in Phase 1 (Updated)
- Backend selection portability
  - Non-WASM uses `wgpu::Backends::PRIMARY` to enable Vulkan/Metal/DX12 automatically across platforms.
  - WASM retains WebGL2 downlevel defaults to ensure broad compatibility.
- Surface format negotiation
  - The surface is configured with the negotiated `surface_format` (preferring sRGB when available).
  - The selected swapchain format is propagated consistently to downstream pipeline creation via global accessor.
- Safe fallback on pipeline creation
  - In RenderContext setup, `expect()` on builder failure was replaced with resilient error handling that logs and falls back to direct pipeline construction.
- Mesh depth support (feature-gated)
  - Depth format helper and mesh pipeline depth-stencil configuration implemented.
  - Depth texture created and recreated on resize; depth attachment included in render pass when mesh is enabled.
- Optional routing alignment
  - RenderContext and MeshRenderContext now acquire pipelines exclusively via centralized helpers; `PipelineBuilder` is retained internally but not used directly by these contexts.

## Open Issues
- [HIGH PRIORITY] Mesh rasterization completion and visual verification
  - Triangle-based rasterization path is implemented (TriangleList with indexed drawing); confirm visual output on native and WASM, validate culling and depth testing, and ensure feature toggling works via native keybinding and web UI toggle.
- [HIGH PRIORITY] Typed `PipelineKey` stabilization
  - Document the key schema; ensure it includes shader identity, bind group layouts, vertex layouts, primitive/multisample state, depth-stencil configuration, and target format.
  - Add unit tests verifying key stability across identical inputs and cache behavior.
- Invalidation wiring
  - Device-lost and shader-reload invalidation present but not fully wired to runtime events; pipelines tied to older devices should be invalidated and lazily rebuilt on next use.
- Warning cleanup
  - Several unused imports/variables remain; clean up to reduce noise and improve maintainability.

## Principles for Non-Breaking Integration (affirmed)
1. Additive and internal-only in Phase 1; avoid public API changes wherever practical.
2. Keep RenderContext behavior identical; delegate pipeline/uniform bind group creation to centralized helpers.
3. Create pipelines only during setup; never per frame.
4. Provide clear invalidation paths for device-lost/shader changes (to be finalized in Phase 2).
5. Add observability under feature flags without runtime overhead.

## Implementation Checklist — Updated
- [x] Parameterize color target format in the pipeline creation API.
- [x] Add PipelineManager to the app orchestrator and instantiate at startup.
- [x] Add app helper and global accessor to create/get pipelines using the correct format.
- [x] Route RenderContext pipeline creation through the helper (RenderContext uses cache helper; MeshRenderContext updated to use centralized mesh helper).
- [x] Add optional debug logging for pipeline creation under a flag.
- [x] Verify behavior via build/run and visual inspection (native and WASM).
- [x] Safe fallback on pipeline creation in RenderContext.
- [x] Backend selection portability (PRIMARY on native, GL on WASM).
- [x] Surface format negotiation (prefer sRGB when available) and swapchain format propagation via global accessor.
- [x] Mesh depth support: depth format helper, pipeline depth-stencil enabled, depth texture lifecycle managed, render pass attachment wired.
- [x] Doctest stabilization: environment-dependent examples in `geometry` and `dicom/fileio` gated with `rust,ignore`; native doctests pass.
- [~] Mesh topology and rasterization updated to indexed triangles (TriangleList + draw_indexed) — vertex and index buffers implemented; depth-stencil enabled; verification pending on native and WASM; culling and feature toggling (non-invasive Mesh Mode) to be confirmed. [HIGH PRIORITY]
- [ ] Web UI toggle for Mesh Mode in static/index.html to call set_mesh_mode_enabled(true/false) and re-render for browser validation. [HIGH PRIORITY]
- [ ] Unit tests for `PipelineKey` stability and cache behavior.
- [ ] Invalidation wiring finalized and tested.
- [ ] Warning cleanup (unused imports/variables).

Legend: [x] completed, [~] partial, [ ] pending

## Action Plan (Next Steps)

High Priority (1–3 days)
Recommendation: Begin with visual verification across native and WASM and add a simple web UI toggle for mesh.
1. **[HIGH PRIORITY] Mesh visual verification and UI toggle**
   - Confirm triangle mesh rasterization visuals on native and WASM targets; validate culling and depth testing remain correct.
   - Add a web UI control to toggle Mesh Mode (calls set_mesh_mode_enabled(true/false) and re-renders) to streamline browser validation.
2. **[HIGH PRIORITY] Unit tests, doctest stabilization, and basic CI**
   - Add tests covering `PipelineKey` stability and cache hit/miss behavior.
   - Doctests stabilized: environment-dependent examples in `geometry` and `dicom/fileio` are gated with `rust,ignore`; native doctests pass.
   - Add CI tasks for `cargo build --features mesh` and `wasm-pack build -t web`; include `cargo test --lib` and doctests gating to guard regressions across targets.
3. **[HIGH PRIORITY] Typed `PipelineKey` documentation**
   - Document the key schema and finalize inputs required for stable caching across platforms.

Medium Priority (3–5 days)
4. Invalidation wiring
   - Wire device-lost and shader-reload events to PipelineManager invalidation and verify lazy rebuild on next use.
5. Warning cleanup and refactors
   - Remove or gate unused imports/variables; apply `cargo fix` where appropriate and follow up with manual cleanups.

Low Priority Optional Items (Deferred)
6. Quality/performance parameters
   - Parameterize MSAA, culling, and optional depth-stencil for TextureQuad and Mesh pipelines.
7. Observability improvements
   - Add counters and simple timing under a feature flag; provide builder/manager status reporting APIs and startup summaries.

## Timelines and Milestones
- Week 1 (Completed)
  - Backend selection and surface format negotiation fixes.
  - Safe fallback on pipeline creation; verified on Windows and WASM builds.
  - Mesh depth support integrated (feature-gated).
- Week 2 (Planned)
  - Optional routing alignment (helper vs builder) and `PipelineKey` stabilization tests.
  - Begin invalidation wiring; add basic tests for cache and invalidation behavior.
  - Mesh rasterization path updated to triangle primitives.

## Acceptance Criteria (Phase 1) — Status
- Build succeeds without public API changes beyond internal-only modules. [Met]
- Visual output and frame time remain consistent with baseline runs. [Met]
- Pipeline color target format is sourced from the negotiated surface/swapchain and used consistently. [Met]
- System remains resilient: pipeline creation failures do not crash the app. [Met]
- Mesh depth support functions across native and WASM, with correct resize behavior and render pass attachment. [Met]

## Rollback Strategy
- Revert to legacy creation paths while keeping PipelineManager present; since Phase 1 is additive, rollback is low-risk.

## Notes
- All changes continue to compile on native targets and WASM with `wasm-pack build -t web`.
- Cross-platform behavior (Windows, macOS, Linux) is improved by avoiding backend hardcoding and honoring surface format negotiation.
- Depth format uses `Depth24Plus` for portability; stencil is omitted.