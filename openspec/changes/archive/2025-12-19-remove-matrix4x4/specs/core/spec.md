# Core Specification

## REMOVED Requirements
### Requirement: Legacy Matrix Support
The `Matrix4x4` struct is no longer supported. All matrix operations MUST be performed using `glam` types (`Mat4`, `DMat4`).

## MODIFIED Requirements
### Requirement: Camera Matrices
The `Camera` struct MUST return `glam::Mat4` for `view_matrix`, `projection_matrix`, and `view_projection_matrix`.

### Requirement: Volume Orientation
Volume generation logic (DICOM, MHA) MUST construct orientation and transform matrices using `glam` directly.
- **Scenario**: constructing a base matrix from voxel spacing and direction vectors.
- **Then**: Use `Mat4::from_scale`, `Mat4::from_cols`, or `Mat4::from_cols_array` to build the matrix.

### Requirement: GeometricScalar Trait
The `GeometricScalar` trait MUST NOT expose conversion methods to/from `Matrix4x4`.
