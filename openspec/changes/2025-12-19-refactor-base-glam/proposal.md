# Refactor Base and Vector3 to use Glam

## Problem
Following the initial refactor of `GeometryBuilder` to use `glam` internally (see `2025-12-18-refactor-geometry-glam`), the `Base` struct in `src/core/coord/base.rs` still relies on the custom, row-major `Matrix4x4<T>` implementation. Additionally, a custom `Vector3<T>` struct is used. This leads to:
1.  **Inefficiency**: Constant conversion between `Matrix4x4` and `glam::Mat4` in `GeometryBuilder` and potentially other future call sites.
2.  **Complexity**: Maintenance of custom matrix and vector math logic (inversion, multiplication, etc.) that duplicates standardized libraries.
3.  **Type Overhead**: The generic `Base<T>` and `Vector3<T>` add complexity while practically only `f32` is needed for WGPU rendering.

## Solution
Fully migrate the `Base` struct to use `glam::Mat4` and remove its generic parameter `T`. Replace `Vector3` with `glam::Vec3`.

1.  **Update `Base` Struct**: Change `pub matrix: Matrix4x4<T>` to `pub matrix: glam::Mat4`. Remove `<T>` generic and standardize on `f32`.
2.  **Replace `Vector3`**: Replace usage of `Vector3<T>` with `glam::Vec3` (for f32) or other appropriate `glam` types.
3.  **Remove Custom Math**: Replace manual implementation of `to_base`, `scale`, `translate`, etc., with `glam` methods (`inverse`, `mul_mat4`, `from_scale`, `from_translation`).
4.  **Update Consumers**:
    *   `GeometryBuilder`: Remove `to_glam`/`from_glam` adapters. Directly assign `glam` matrices.
    *   `CTVolume` & `Geometry`: Update to hold `Base` (concrete type) or `glam::Mat4` instead of `Base<f32>`/`Matrix4x4<f32>`.
    *   `MprView`: Update matrix access and uniform updates to use `glam` layouts (column-major).
5.  **Deprecation**: The custom `Matrix4x4` and `Vector3` structs in `src/core/coord/mod.rs` can eventually be deprecated or removed if no longer used.

## Impact
- **Performance**: Zero-cost abstractions for matrix operations; removal of conversion overhead.
- **Code Quality**: Reduced lines of code; usage of industry-standard math types.
- **WGPU Alignment**: `glam` types (`Mat4`, `Vec3`) are directly compatible with WGPU uniform buffers (via `bytemuck` feature).

## Risks
- **Coordinate System Mismatch**: `Matrix4x4` was row-major. `glam::Mat4` is column-major.
    - *Mitigation*: Existing tests in `GeometryBuilder` must pass. We must carefully verify that matrix construction (which often assumes row-major data in `Matrix4x4::from_array`) is correctly transposed or reconstructed using `Mat4::from_cols_array` / `Mat4::from_rows_array` as appropriate.
- **API Breakage**: This is a breaking change for any code accessing `Base.matrix` or `Vector3`.
