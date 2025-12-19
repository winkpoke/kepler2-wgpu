# Change: Refactor MIP View to use glam

## Why
The current `MipView` implementation uses raw `[f32; 3]` arrays for geometric state (pan), which lacks type safety and misses out on SIMD optimizations. Consistent with the broader project goal of adopting `glam` for all geometry, this refactor improves code quality and performance.

## What Changes
- Refactor `MipView` to use `glam::Vec3` for the `pan` field.
- Update `update` method to access vector components.
- Update `set_pan` to use `glam` vector operations for clamping.
- Ensure zero-cost abstraction with `#[repr(C)]` compatibility where needed (though `glam` handles this well).

## Impact
- **Affected Specs**: `rendering`
- **Affected Code**: `src/rendering/view/mip/mod.rs`
- **Breaking Changes**: None (internal state change only).
