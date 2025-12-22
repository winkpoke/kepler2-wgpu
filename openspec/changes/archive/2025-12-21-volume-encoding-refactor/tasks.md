# Tasks: Semantic Volume Encoding Refactor

- [x] **Core Data Structures**
    - [x] Create `src/data/volume_encoding.rs` with `VolumeEncoding` enum.
    - [x] Update `src/data/mod.rs` to export the new module.

- [x] **Render Content & Model**
    - [x] Update `RenderContent` struct to store `VolumeEncoding`.
    - [x] Implement `RenderContent::decode_parameters()` helper.
    - [x] Update `RenderContent` constructors to accept `VolumeEncoding`.
    - [x] Update `AppModel::get_volume_render_data` to return `(Vec<u8>, VolumeEncoding)` instead of `bool`.
    - [x] Remove `AppModel::HU_OFFSET` (moved to `VolumeEncoding::DEFAULT_HU_OFFSET`).

- [x] **View Factory & Window Level**
    - [x] Update `DefaultViewFactory` to use `VolumeEncoding` when creating `RenderContent`.
    - [x] Update `WindowLevel` constants to allow bias range `[-2048, 2048]` (fixing the 1024 clamp bug).

- [x] **Application Logic (Pending Fixes)**
    - [x] Update `App::load_data_from_ct_volume` in `src/application/app.rs`.
        - Handle the `VolumeEncoding` return type from `AppModel`.
        - Use `VolumeEncoding::DEFAULT_HU_OFFSET` instead of `AppModel::HU_OFFSET`.

- [x] **View Implementations (Pending Fixes)**
    - [x] Update `MipView::update` in `src/rendering/view/mip/mod.rs`.
        - Use `render_content.decode_parameters()` to populate uniforms.
        - Remove hardcoded `1100.0` and `Rg8Unorm` checks.
    - [x] Update `MprViewWgpuImpl::new` in `src/rendering/view/mpr/mpr_view_wgpu_impl.rs`.
        - Use `render_content.decode_parameters()` to populate uniforms.
        - Remove hardcoded `1100.0` and `Rg8Unorm` checks.

- [x] **Verification**
    - [x] Verify `cargo check` passes.
    - [x] Verify no magic number `1100.0` remains in view code.
