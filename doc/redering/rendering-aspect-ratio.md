# Aspect-Ratio Safe Rendering for Slice/MIP

This change introduces aspect-preserving fitting for slice-based views (MPR) and MIP so content is correctly scaled and centered for any container aspect ratio.

## Behavior
- Preserves the physical in-plane aspect by letterboxing/pillarboxing into the view rectangle.
- Maintains spatial relationships and prevents stretching while respecting optional uniform padding.
- 1:1 cases remain visually identical to previous behavior.

## Supported Ratios
- Any width:height ratio is supported. Internally, effective content aspect is clamped to [1e-4, 1e4] for numerical stability.
- Zero or negative dimensions are considered invalid and skipped; callers receive a safe 1×1 fallback viewport when necessary.

## Performance
- CPU-only arithmetic and a single viewport state change. No shader changes. Overhead is negligible for both native and WebAssembly builds.

## Code Example (no_run)
```rust
#![no_run]
use kepler_wgpu::rendering::view::layout::compute_aspect_fit;

let container = (1600u32, 900u32);  // 16:9 view
let content_physical = (4.0f32, 3.0f32); // in-plane width:height
let padding = 0u32;

if let Some(fit) = compute_aspect_fit(container.0, container.1, content_physical.0, content_physical.1, padding) {
    // Set viewport to (fit.x, fit.y, fit.w, fit.h) relative to the view origin
}
```

## Visual Examples
- 16:9 content in 4:3 container → horizontal letterboxing
- 4:3 content in 16:9 container → vertical pillarboxing
- 1:1 content in 1:1 container → full-bleed, unchanged

## Notes
- MPR uses physical in-plane size from volume dimensions × voxel spacing per orientation.
- MIP uses the texture extent (width/height); if voxel spacing is anisotropic, a future enhancement can pass spacing for physically accurate fitting.
