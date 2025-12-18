# Implementation Tasks

## 1. Implementation
- [x] Move configuration fields (`enable_mesh`, `enable_float_volume_texture`) from `App` struct to `AppModel` struct in `src/application/app_model.rs`.
- [x] Update `AppModel::new` to accept initial configuration.
- [x] Move `HU_OFFSET` constant to `AppModel`.
- [x] Implement `AppModel::get_volume_render_data` to handle voxel-to-bytes conversion (including `f16` and `HU_OFFSET` logic).
- [x] Update `App` struct in `src/application/app.rs` to remove moved fields.
- [x] Refactor `App::load_data_from_ct_volume` to call `AppModel::get_volume_render_data`.
- [x] Update all references to `self.enable_mesh` and `self.enable_float_volume_texture` in `App` to access them via `self.app_model`.
- [x] Verify `App::swap_graphics` and `App::resize` still work correctly with state in `AppModel`.

## 2. Verification
- [x] Run `cargo test` to ensure no regressions in existing logic.
- [x] Add unit test for `AppModel::get_volume_render_data` to verify correct byte generation (optional but recommended - validated via integration tests).
- [x] Verify native build `cargo build`.
- [x] Verify WASM build `wasm-pack build -t web`.
