# Migration to glam for Coordinate Systems

## Problem
The current `Base<T>` and `Matrix4x4<T>` implementations in `src/core/coord/` are redundant custom implementations of linear algebra. They lack SIMD optimizations, require manual maintenance of complex math logic (Gaussian elimination, multiplication), and introduce friction when interoperating with WGPU and other graphics libraries that expect standard memory layouts.

## Solution
Replace the internal math engine with `glam` and refactor `Base` to use `glam` types directly, removing the `Matrix4x4` abstraction.

- **Retain `Base<T>`**: The `Base` struct will remain the primary interface for coordinate systems.
- **Internalize `glam`**:
    - The `Base` struct shall directly contain a `glam` matrix type (e.g., `glam::Mat4` or `glam::DMat4`) instead of the custom `Matrix4x4` struct.
    - The custom `Matrix4x4` struct will be retained to support existing code that relies on it, preventing widespread breakage.
- **Preserve API**: Public methods like `scale`, `translate`, and `to_base` will keep their signatures (or close equivalents) but delegate to `glam` for calculations.

## Impact
- **Performance**: Significant speedup in matrix operations due to SIMD instructions.
- **Maintainability**: Removal of ~200 lines of custom math code.
- **Interoperability**: Direct compatibility with WGPU and other ecosystem crates.
- **Safety**: Reduced risk of numerical errors in custom implementation.
- **Minimal Churn**: By keeping `Base`, we avoid rewriting all call sites that pass `Base` objects (e.g., View logic, Volume logic).
