# Refactor Matrix Column Major Tasks

## 1. Implementation
- [x] 1.1 Modify `Matrix4x4` struct definition (Rename `data` -> `columns`, add `#[repr(C)]`)
- [x] 1.2 Implement Constructors (`from_rows`, `from_cols`)
- [x] 1.3 Update Mathematical Methods (`multiply`, `apply`, `inv`, `transpose`)
- [x] 1.4 Migration & Fixes
    - [x] Replace struct literal initializations in tests with `from_rows`
    - [x] Update `matrix.data` access to `matrix.columns`
    - [x] Fix index access patterns (swap `[row][col]` to `[col][row]` where needed)
    - [x] Update `GeometryBuilder` and `MprView` integration
- [x] 1.5 Verification
    - [x] Run `cargo test` to ensure mathematical correctness
    - [x] Verify WGPU rendering

## 2. Documentation
- [x] 2.1 Update doc comments in `Matrix4x4`
- [x] 2.2 Add note in `doc/CHANGELOG.md`
