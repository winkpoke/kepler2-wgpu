# Tasks

- [ ] **Dependency Update**
    - [ ] Update `Cargo.toml`: Enable `bytemuck` feature for `glam`.

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
    - [ ] Update `MprView` in `src/rendering/view/mpr/mpr_view.rs` to handle `glam::Mat4`.
    - [ ] Ensure uniform buffer updates (`queue.write_buffer`) correctly handle column-major layout (likely no transpose needed if shader expects standard WGSL matrix).

- [ ] **Verification**
    - [ ] Run `cargo test` and fix compilation errors.
    - [ ] Verify `test_coordinate_system` and other tests in `src/core/coord/mod.rs` (update them to use `glam`).
    - [ ] Verify `GeometryBuilder` tests.
