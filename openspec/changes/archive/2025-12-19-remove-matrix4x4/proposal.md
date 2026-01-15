# Remove Legacy Matrix4x4

## Why
The `Matrix4x4` struct is a legacy custom implementation of a 4x4 matrix. The codebase has largely moved to using `glam` for linear algebra, which offers SIMD optimizations and better interoperability with WGPU. The existence of `Matrix4x4` creates confusion, requires manual conversion logic, and prevents full adoption of standard graphics math patterns.

## What Changes
1.  **Refactor Camera**: Update `Camera` in `src/rendering/view/mesh/camera.rs` to return `glam::Mat4` for all projection and view matrices.
2.  **Refactor Data Ingestion**: Update `DicomRepo` and `MedicalVolume` to construct matrices using `glam::Mat4` builders (e.g., `from_scale`, `from_translation`) instead of manual array construction with `Matrix4x4`.
3.  **Clean Core API**: Remove `to_matrix4x4` and `from_matrix4x4` from the `GeometricScalar` trait and `Base` struct.
4.  **Remove Code**: Delete the `Matrix4x4` struct definition from `src/core/coord/mod.rs`.

## Impact
- **Performance**: Improved performance in matrix construction and multiplication due to `glam`'s SIMD optimizations.
- **Maintainability**: Reduced code size by removing custom math implementations (inversion, multiplication).
- **Consistency**: Unified math library usage across the application.

## Risks
- **Coordinate System Mismatch**: `Matrix4x4` constructors often assumed specific row/column layouts. `glam` is column-major. Refactoring requires careful verification that matrix construction logic (especially from raw arrays in DICOM/MHA) correctly maps to `glam`'s expectation.
    - *Mitigation*: Verify transformation matrices against known ground truths (e.g., identity, simple translations).
