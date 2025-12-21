# Tasks

- [x] **Dependency Update**
    - [x] Update `Cargo.toml`: Enable `bytemuck` feature for `glam`.

- [x] **Refactor `Base` Struct**
    - [x] Modify `src/core/coord/base.rs` to use `glam::Mat4` and remove generic `<T>`.
    - [x] Re-implement `to_base`, `get_scale_factors`, `scale`, `translate` using `glam` methods.
    - [x] Update `Debug` implementation.
    - [x] **Critical**: Ensure `Matrix4x4` (row-major) data is correctly mapped to `Mat4` (column-major). Use `Mat4::from_rows_slice` if source data is row-major.

- [x] **Refactor `Vector3`**
    - [x] Replace `Vector3<T>` with `glam` types (`Vec3`, `DVec3`, `IVec3`, `UVec3`) depending on usage.
    - [x] Remove `Vector3` struct from `src/core/coord/mod.rs` or make it a compatibility alias if needed.

- [x] **Update `GeometryBuilder`**
    - [x] Remove `to_glam` and `from_glam` helpers in `src/core/geometry.rs`.
    - [x] Update `build_uv_base`, `build_transverse_base`, `build_coronal_base`, `build_sagittal_base`, `build_oblique_base` to populate `Base.matrix` directly with `glam::Mat4`.

- [x] **Update Data Models**
    - [x] Update `CTVolume` in `src/data/ct_volume.rs` to use concrete `Base`.
    - [x] Update `Geometry` struct in `src/data/ct_volume.rs` to use `glam::Mat4` (was `Matrix4x4<f32>`).

- [x] **Update Rendering Views**
    - [x] **MprView** (`src/rendering/view/mpr/mpr_view.rs`)
        - [x] Replace `Base<f32>` fields with `glam::Mat4`.
        - [x] Replace `[f32; 3]` pan with `glam::Vec3`.
        - [x] Update `screen_coord_to_world` and `set_center_at_point_in_mm` to use `glam`.
        - [x] Ensure uniform buffer updates handle column-major layout correctly.
    - [x] **MeshView** (`src/rendering/view/mesh/mesh_view.rs`)
        - [x] Refactor `update_uniforms` to use `glam::Mat4`.
        - [x] Remove manual matrix construction and multiplication logic.
        - [x] Ensure consistent coordinate system handling (orthographic projection).

- [x] **Verification**
    - [x] Run `cargo test` and fix compilation errors.
    - [x] Verify `test_coordinate_system` and other tests in `src/core/coord/mod.rs` (update them to use `glam`).
    - [x] Verify `GeometryBuilder` tests.
