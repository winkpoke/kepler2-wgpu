## Scope and Assumptions

* "TSC" is not present in the codebase; we apply the enhancement to slice/volume rendering views that exhibit the same behavior: MPR and MIP. This covers all user-visible "TSC-like" rendering.

* Keep existing behavior for 1:1 layouts. No breaking changes.

## Goals

* Correctly scale and position content for any aspect ratio (landscape, portrait, arbitrary).

* Maintain visual integrity, spatial relationships, and margins/padding.

* Validate inputs, guard numerical issues, and handle pathological aspect ratios.

* Add focused unit tests for layout math.

* Document supported ratios, performance notes, and examples.

## High-Level Design

* Introduce a small, pure layout helper that computes a letterboxed/pillarboxed inner viewport rectangle that preserves content aspect ratio.

* Use actual in-plane physical aspect (voxel dimensions × voxel spacing) for medical accuracy.

* Both MPR and MIP set the render pass viewport to this inner rectangle; content is neither stretched nor distorted.

* Keep the existing full-screen quad/shaders unchanged.

## Files To Update

* src/rendering/view/layout.rs

  * Add AspectFit API (pure functions and a small result struct) with input validation and edge-case handling.

* src/rendering/view/mpr/mpr\_view\.rs

  * Use AspectFit to compute an inner viewport from the view area and the in-plane physical size (based on orientation).

  * Replace direct `set_viewport(x, y, width, height)` with the fitted rectangle.

* src/rendering/view/mip/mod.rs

  * Use AspectFit based on volume physical X/Y extent to compute an inner viewport and apply it.

* tests/layout\_aspect\_fit.rs (new)

  * Unit tests for AspectFit across 16:9, 4:3, 1:1, extreme and invalid dimensions.

* doc/rendering-aspect-ratio.md (new)

  * Behavioral changes, supported ranges, performance considerations, visual examples and "no\_run" sample snippets.

## Implementation Details

1. AspectFit helper (layout.rs)

* Add:

  * struct `AspectFitResult { x: f32, y: f32, w: f32, h: f32, scale: f32 }`

  * fn `compute_aspect_fit(container_w: u32, container_h: u32, content_w: f32, content_h: f32, padding: u32) -> Option<AspectFitResult>`

* Behavior:

  * Validate inputs: return None for zero/negative content or container sizes; clamp padding so `2*padding < min(container_w, container_h)`; clamp w/h to at least 1 for WGPU safety (consistent with OneCellLayout min 1 pixel).

  * Compute container inner rect = `(padding .. w - padding, padding .. h - padding)`; compute `content_aspect = content_w / content_h`; compute `container_aspect = inner_w / inner_h` using f32 with EPS checks.

  * If content\_aspect > container\_aspect → pillarbox; else → letterbox. Center the fitted rect, return scale and fitted x/y/w/h.

  * Handle extreme aspect ratios by capping effective aspect at `[1/10_000, 10_000]` to avoid infinities; log at debug when capped.

  * Use f32 throughout with `is_finite` guards.

1. MPR: apply fitted viewport

* Reference point for current viewport: src/rendering/view/mpr/mpr\_view\.rs:273

* Determine in-plane physical content size based on `orientation` used when constructing `base_screen`:

  * Transverse: content\_w = dim\_x \* spacing\_x; content\_h = dim\_y \* spacing\_y

  * Coronal: content\_w = dim\_x \* spacing\_x; content\_h = dim\_z \* spacing\_z

  * Sagittal: content\_w = dim\_y \* spacing\_y; content\_h = dim\_z \* spacing\_z

  * Oblique: approximate with transverse unless a more accurate dynamic basis is available; keep backward-compatible by using screen base scale factors

* Compute `AspectFitResult` using current `self.dim` as container and a configurable `padding` (default 0 or small default like 4 px); set viewport to `(self.pos.x + fit.x, self.pos.y + fit.y, fit.w, fit.h)`.

* Leave transform/shaders unchanged; content is sampled within the fitted viewport without stretching.

1. MIP: apply fitted viewport

* Reference point for current viewport: src/rendering/view/mip/mod.rs:383

* Compute content physical X/Y using volume metadata available when creating the view (CTVolume.dimensions and voxel\_spacing). If only the `RenderContent` is present, we can retrieve extent via `texture.size()` and approximate spacing as 1.0 for backward compatibility.

* Apply `compute_aspect_fit` with the view’s `dimensions`; set the fitted viewport on the render pass as above.

1. Validation and Edge Cases

* Guard zero/negative dimensions and non-finite values; return a minimal 1×1 safe rect or skip drawing if both container dims are invalid.

* Clamp extreme aspect ratios to safe bounds during fit math to avoid numeric precision issues.

* Ensure all math is f32; cast to f32 for WGPU viewport calls.

1. Unit Tests

* tests/layout\_aspect\_fit.rs

  * 1:1 content in 1:1 container → fitted rect equals inner container; scale=1.

  * 16:9 content in 4:3 container → letterbox vertical; widths match; centered Y.

  * 4:3 content in 16:9 container → pillarbox horizontal; heights match; centered X.

  * Very wide (100:1) and very tall (1:100) content in square container → extreme capping still yields a non-zero, centered rect.

  * Zero/negative container or content dims → returns None; callers handle by skipping render or using 1×1.

  * Padding larger than half min(container\_w, container\_h) → clamped and still valid.

1. Documentation (doc/rendering-aspect-ratio.md)

* Supported aspect ratios: effectively any, with numeric safety caps at 1e-4 to 1e4.

* Performance: This is CPU-only arithmetic and a single viewport state change; no shader changes; no measurable GPU cost. Works in native and WebAssembly targets.

* Visual examples: screenshots or diagrams for 1:1, 16:9 in 4:3, and 4:3 in 16:9; code snippets marked with `no_run`.

## Backward Compatibility

* When container and content ratios are equal, the fitted viewport equals the full view rect; behavior matches existing 1:1 path.

* Default padding set to 0 (or use a small default as a configurable parameter); if 0, the result is visually identical to existing behavior for equal aspect cases.

## Logging and Build Targets

* Add DEBUG logs in aspect-fitting helper for odd inputs/clamps; default INFO logging remains unchanged.

* No new dependencies; works for native and wasm. No perspective projection is introduced.

## Acceptance and Verification

* Build and run unit tests via `cargo test`.

* Manual visual verification on common display ratios (16:9, 4:3, ultrawide portrait) to confirm proper letterboxing/pillarboxing in MPR and MIP.

## Pointers for Implementation

* Current viewport calls for modification:

  * MPR: src/rendering/view/mpr/mpr\_view\.rs:273

  * MIP: src/rendering/view/mip/mod.rs:383

* Layout base code for helper addition:

  * src/rendering/view/layout.rs (suitable place for a general-purpose fit utility)

