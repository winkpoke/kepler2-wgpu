## Context
The 3D view currently centers on mesh rendering, which limits soft-tissue inspection. Volume rendering provides intensity-driven visualization that aligns with clinical tools like 3D Slicer while retaining GPU performance constraints across native and wasm targets.

## Goals / Non-Goals
- Goals: Add a minimal, performant volume rendering path in the 3D view using existing volume textures.
- Goals: Preserve orthographic projection and cross-platform compatibility.
- Non-Goals: Full cinematic transfer-function editor or advanced global illumination.

## Decisions
- Decision: Implement a ray-marched volume rendering shader in WGSL using a configurable step size and opacity window.
- Decision: Keep orthographic projection and reuse existing volume texture bindings to avoid new data paths.
- Alternatives considered: Full mesh-based iso-surface extraction (higher CPU cost, slower iteration).

## Risks / Trade-offs
- Risk: Increased GPU cost at high step counts → Mitigation: default conservative step size and allow tuning.
- Risk: Performance variance on wasm GPUs → Mitigation: cap samples per ray and avoid CPU-GPU sync in render loops.

## Migration Plan
1. Add new pipeline and uniforms behind a view toggle.
2. Default to existing mesh rendering until user selects volume mode.
3. Validate on native and wasm builds.

## Open Questions
- Should default opacity curve be linear or window/level-driven for CT?
