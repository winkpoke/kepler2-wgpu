# Tasks

- [x] Add `glam` dependency to `Cargo.toml` with `features = ["bytemuck", "serde"]` <!-- id: 0 -->
- [x] Retain `Matrix4x4` struct in `src/core/coord/mod.rs` for compatibility <!-- id: 1 -->
- [x] Update `Base<T>` in `src/core/coord/base.rs` to directly store `glam::Mat4` or `glam::DMat4` <!-- id: 2 -->
- [x] Implement conversion traits/methods between `Base` (glam) and `Matrix4x4` if needed <!-- id: 7 -->
- [x] Re-implement `Base::scale`, `Base::translate`, and `Base::to_base` using `glam` operations while keeping signatures compatible <!-- id: 3 -->
- [x] Remove legacy custom math logic (Gaussian elimination, manual multiplication) <!-- id: 4 -->
- [x] Verify `geometry.rs` and other consumers still compile with minimal changes <!-- id: 5 -->
- [x] Verify all tests pass with new implementation <!-- id: 6 -->
