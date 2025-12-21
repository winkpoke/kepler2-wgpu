# Implementation Tasks

## 1. Implementation
- [x] Refactor Graphics Delegation:
    - [x] Add `graphics()` and `graphics_mut()` to `App`.
    - [x] Remove `device()`, `queue()`, `surface()`, `surface_config()`, `surface_config_mut()`, `adapter()`, `pass_executor_mut()` from `App`.
    - [x] Update internal usages in `App` to use `self.graphics().field`.
    - [x] Update external usages in `render_app.rs` and other modules to use `app.graphics().field`.
- [x] Simplify View Property Setters:
    - [x] Implement `apply_to_mesh_view<F>(&mut self, f: F)` helper in `App`.
    - [x] Refactor `set_mesh_rotation_enabled`, `set_mesh_scale`, `set_mesh_pan`, `reset_mesh`, `set_mesh_rotation_speed` to use the helper.
- [x] Error Handling in Data Loading:
    - [x] Change `load_data_from_ct_volume` return type to `Result<Arc<RenderContent>, KeplerError>`.
    - [x] Propagate errors from `RenderContent::from_bytes` and view creation.
    - [x] Handle `Result` in `load_data_from_repo` and `set_mesh_mode_enabled`.
- [x] Consolidate Layout Construction (Optional/Phase 2):
    - [x] Extract `rebuild_layout` logic.

## 2. Verification
- [x] Verify `cargo check` passes after API changes.
- [x] Verify `cargo test` passes.
- [x] Verify WASM build `wasm-pack build -t web`.
- [x] Manual verification:
    - [x] Check Mesh rotation/scaling controls in UI.
    - [x] Check volume loading success path.
