# mesh_3d imports fix

Time: 2025-11-11T16-02-46

Summary:
- Fixed incorrect imports in `src/rendering/view/mesh/mesh_3d.rs` to use `dicom_object::open_file` and `dicom_core::Tag` instead of the non-declared `dicom` crate path.
- Added `ndarray` and `ndarray-stats` to `Cargo.toml` to satisfy `Array3` and `QuantileExt` usage.

Rationale:
- The project declares `dicom-object` and `dicom-core` crates. Importing via `dicom::object` and `dicom::core` leads to unresolved crate errors.
- `mesh_3d.rs` relies on `ndarray::Array3` and `ndarray_stats::QuantileExt` for volume processing; adding the dependencies ensures build consistency across native and wasm targets.

Example (no_run):
```rust
// Correct imports (no_run)
use dicom_object::open_file;
use dicom_core::Tag;
use ndarray::Array3;
use ndarray_stats::QuantileExt;
```

Notes:
- This change is internal and does not alter UI/visual output.
- The file system operations in `mesh_3d.rs` are for development; avoid running them in wasm builds.