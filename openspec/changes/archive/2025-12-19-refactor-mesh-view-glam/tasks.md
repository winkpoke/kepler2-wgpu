## 1. Implementation
- [x] 1.1 Add `glam` imports (`Mat4`, `Vec3`, `Quat`) to `src/rendering/view/mesh/mesh_view.rs`
- [x] 1.2 Refactor `MeshView::update_uniforms` to use `glam` for Model matrix (translation * rotation * scale)
- [x] 1.3 Refactor `MeshView::update_uniforms` to use `glam` for View matrix
- [x] 1.4 Refactor `MeshView::update_uniforms` to use `glam` for Projection matrix (matching existing ortho parameters)
- [x] 1.5 Verify matrix multiplication order matches `wgpu` expectations (column-major)
- [x] 1.6 Verify rendering output remains correct
