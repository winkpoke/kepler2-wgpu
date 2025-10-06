# PipelineManager Integration Plan (Phase 1) — Status and Action Plan

This document tracks the current implementation status of the PipelineManager integration, highlights deviations and issues, and outlines a prioritized action plan to complete Phase 1 and prepare for Phase 2. The objectives remain: stability, performance, and full backward compatibility with existing 2D MPR behavior across native and WASM targets.

## Goals (unchanged)
- Introduce PipelineManager as an internal utility to centralize pipeline creation and set the stage for caching/reuse.
- Maintain current visual output and frame-time characteristics; avoid public API churn.
- Parameterize pipeline color target format to improve portability across OS/GPU combinations.

## Current Implementation Status

Completed
- Parameterized color target format in texture-quad pipeline API; creation uses the provided format consistently.
- PipelineManager added to the application orchestrator and instantiated at startup; pipelines are cached and returned as `Arc<wgpu::RenderPipeline>`.
- Internal helper defined in the app to create/get texture-quad pipelines with the correct format.
- Centralized creation via PipelineBuilder for TextureQuad; cache keys include target format and layout signatures; cache hit/miss counters added.
- Pipelines created during setup (context/view construction), not per frame; rendering reuses pipelines.
- Optional debug logging is gated behind a feature flag for cache hits/misses and builder status.
- Native and WASM builds succeed (wasm-pack with `-t web`).

Partial / Deviations
- RenderContext routes through PipelineBuilder directly instead of the app helper; function signatures were adjusted internally to accept a `PipelineManager` reference.
- A typed `PipelineKey` already exists (originally planned for Phase 2). This is acceptable but should be stabilized.

Identified Issues
- Cross-platform backend selection uses DX12 for non-WASM and GL for WASM; this may limit portability on macOS/Linux and older GPUs.
- Surface configuration forces `Rgba8Unorm` while negotiated `surface_format` is available (prefers sRGB). This may cause subtle color differences and reduces portability.
- Pipeline creation failure currently panics at the RenderContext setup (no safe fallback path).
- Device-lost/shader-reload invalidation methods exist in PipelineManager but aren’t wired to runtime events yet.

## Principles for Non-Breaking Integration (affirmed)
1. Additive and internal-only in Phase 1; avoid public API changes wherever practical.
2. Keep RenderContext behavior identical; delegate pipeline/uniform bind group creation to centralized helpers.
3. Create pipelines only during setup; never per frame.
4. Provide clear invalidation paths for device-lost/shader changes (to be finalized in Phase 2).
5. Add observability under feature flags without runtime overhead.

## Implementation Checklist — Updated
- [x] Parameterize color target format in the pipeline creation API.
- [x] Add PipelineManager to the app orchestrator and instantiate at startup.
- [x] Add app helper to create/get texture-quad pipelines using the correct format.
- [~] Route RenderContext pipeline creation through the helper (currently uses PipelineBuilder directly).
- [x] Add optional debug logging for pipeline creation under a flag.
- [x] Verify behavior via build/run and visual inspection.

Legend: [x] completed, [~] partial, [ ] pending

## Action Plan (Next Steps)

High Priority (1–2 days)
1. Backend selection portability
   - Non-WASM: use `wgpu::Backends::PRIMARY` to enable Vulkan/Metal/DX12 automatically.
   - WASM: retain WebGL2 downlevel defaults but ensure compatibility flags remain.
2. Surface format negotiation
   - Configure the surface with the negotiated `surface_format` (prefer sRGB when available).
   - Set the global swapchain format from the negotiated value and propagate consistently.
3. Safe fallback on pipeline creation
   - Replace `expect()` in RenderContext setup with error handling that logs failures and falls back to direct pipeline creation; keep runtime resilient.

Medium Priority (3–5 days)
4. Optional routing alignment
   - Provide an internal utility so RenderContext can call the app helper or a small wrapper to acquire pipelines without further signature changes.
   - Keep current PipelineBuilder path as acceptable for Phase 1; ensure the helper and builder yield consistent results.
5. Stabilize typed PipelineKey
   - Document the key schema; ensure it includes shader identity, bind group layouts, vertex layouts, primitive/multisample state, and target format.
   - Add unit tests for key stability across identical inputs.

Phase 2 Preparation (5–10 days)
6. Invalidation wiring
   - Wire device-lost and shader-reload events to PipelineManager invalidation; rebuild lazily on next use.
7. Quality/performance parameters
   - Parameterize MSAA, culling, and optional depth-stencil for TextureQuad and Mesh pipelines.
8. Observability improvements
   - Add counters and simple timing under a feature flag; provide builder/manager status reporting APIs.

## Timelines and Milestones
- Week 1
  - Complete backend selection and surface format negotiation fixes.
  - Implement safe fallback on pipeline creation; verify on Windows and at least one non-Windows environment.
- Week 2
  - Optional routing alignment (helper vs builder) and PipelineKey stabilization.
  - Begin invalidation wiring; add basic tests for cache and invalidation behavior.

## Acceptance Criteria (Phase 1) — Reaffirmed
- Build succeeds without public API changes beyond internal-only modules.
- Visual output and frame time remain consistent with baseline runs.
- Pipeline color target format is sourced from the negotiated surface/swapchain and used consistently.
- System remains resilient: pipeline creation failures do not crash the app.

## Rollback Strategy
- Revert to legacy creation paths while keeping PipelineManager present; since Phase 1 is additive, rollback is low-risk.

## Notes
- All changes should continue to compile on native targets and WASM with `wasm-pack build -t web`.
- Ensure cross-platform behavior (Windows, macOS, Linux) by avoiding backend hardcoding and honoring surface format negotiation.