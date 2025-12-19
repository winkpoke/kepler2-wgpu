# Refactor Geometry to use Glam

## Problem
The `src/core/geometry.rs` module currently relies on a custom `Matrix4x4` implementation for geometric transformations. This custom implementation:
1.  Lacks SIMD optimizations found in industry-standard crates.
2.  Requires manual maintenance of matrix math logic.
3.  Uses a row-major memory layout, whereas modern graphics standards (and WGPU) often favor column-major or specific alignment that libraries like `glam` handle efficiently.
4.  Creates friction when interfacing with other graphics code that likely uses standard math libraries.

## Solution
Replace the internal matrix math in `src/core/geometry.rs` with the `glam` crate (`Mat4`, `Vec3`).
- Introduce helper functions to convert between the legacy `Matrix4x4` (preserved in `Base` struct for now to limit scope) and `glam::Mat4`.
- Rewrite `build_*_base` methods to use `glam` for matrix construction and multiplication.
- Ensure all transformations remain mathematically equivalent.

## Impact
- **Performance**: Potential speedups due to `glam`'s SIMD optimizations.
- **Maintainability**: Reduced custom math code; leveraging a robust community crate.
- **Correctness**: `glam` is widely tested, reducing the risk of bugs in custom matrix math.

## Constraints
- **WASM Compatibility**: The solution MUST work seamlessly in WebAssembly environments. `glam` is chosen partly because it supports WASM (with and without SIMD, depending on target features).

## Risks
- **Coordinate System Mismatch**: `Matrix4x4` is row-major, while `glam` is column-major. Careful conversion (transposition) is required to ensure correctness.
- **Scope Creep**: This proposal strictly limits `glam` usage to the implementation of `GeometryBuilder`. `Base<T>` and `CTVolume` will continue to hold `Matrix4x4` to avoid a codebase-wide refactor at this stage.
