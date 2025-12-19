## 1. Implementation
- [ ] 1.1 Add `use glam::Vec3;` to `src/rendering/view/mip/mod.rs`.
- [ ] 1.2 Change `MipView.pan` type from `[f32; 3]` to `Vec3`.
- [ ] 1.3 Update `MipView::new` to initialize `pan` with `Vec3::ZERO`.
- [ ] 1.4 Update `MipView::update` to access `.x` and `.y` components of `pan`.
- [ ] 1.5 Refactor `MipView::set_pan` to use `Vec3::new` and `Vec3::clamp` for vector-based logic.
- [ ] 1.6 Verify compilation and run tests.
