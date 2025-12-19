# Tasks

- [ ] **Dependency Update**
    - [x] Update `Cargo.toml`: Enable `bytemuck` feature for `glam`.

- [ ] **Refactor `Base` Struct**
    - [ ] Modify `src/core/coord/base.rs` to use `glam::Mat4` and remove generic `<T>`.
    - [ ] Re-implement `to_base`, `get_scale_factors`, `scale`, `translate` using `glam` methods.
    - [ ] Update `Debug` implementation.
    - [ ] **Critical**: Ensure `Matrix4x4` (row-major) data is correctly mapped to `Mat4` (column-major). Use `Mat4::from_rows_slice` if source data is row-major.

- [ ] **Refactor `Vector3`**
    - [ ] Replace `Vector3<T>` with `glam` types (`Vec3`, `DVec3`, `IVec3`, `UVec3`) depending on usage.
    - [ ] Remove `Vector3` struct from `src/core/coord/mod.rs` or make it a compatibility alias if needed.

- [ ] **Update `GeometryBuilder`**
    - [ ] Remove `to_glam` and `from_glam` helpers in `src/core/geometry.rs`.
    - [ ] Update `build_uv_base`, `build_transverse_base`, `build_coronal_base`, `build_sagittal_base`, `build_oblique_base` to populate `Base.matrix` directly with `glam::Mat4`.

- [ ] **Update Data Models**
    - [ ] Update `CTVolume` in `src/data/ct_volume.rs` to use concrete `Base`.
    - [ ] Update `Geometry` struct in `src/data/ct_volume.rs` to use `glam::Mat4` (was `Matrix4x4<f32>`).

- [ ] **Update Rendering Views**
    - [ ] **MprView** (`src/rendering/view/mpr/mpr_view.rs`)
        - [ ] Replace `Base<f32>` fields with `glam::Mat4`.
        - [ ] Replace `[f32; 3]` pan with `glam::Vec3`.
        - [ ] Update `screen_coord_to_world` and `set_center_at_point_in_mm` to use `glam`.
        - [ ] Ensure uniform buffer updates handle column-major layout correctly.
    - [ ] **MeshView** (`src/rendering/view/mesh/mesh_view.rs`)
        - [ ] Refactor `update_uniforms` to use `glam::Mat4`.
        - [ ] Remove manual matrix construction and multiplication logic.
        - [ ] Ensure consistent coordinate system handling (orthographic projection).

- [ ] **Verification**
    - [ ] Run `cargo test` and fix compilation errors.
    - [ ] Verify `test_coordinate_system` and other tests in `src/core/coord/mod.rs` (update them to use `glam`).
    - [ ] Verify `GeometryBuilder` tests.
