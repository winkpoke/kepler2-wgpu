# core Specification

## Purpose
Provides fundamental data structures, error types, and mathematical utilities for the application, including coordinate systems and geometry transformations.

## Requirements

### Requirement: Standardized Math Library
The core geometry subsystem MUST use `glam` as the primary linear algebra library for calculations.
The `Base<T>` struct MUST be retained as the public interface for coordinate systems, and it MUST directly contain a `glam` matrix (e.g. `glam::Mat4` or `glam::DMat4`) instead of any custom matrix type.
The `Matrix4x4` struct is no longer supported and MUST NOT be used.

#### Scenario: Matrix Construction
Given a request to build a geometric base (e.g., Transverse, Coronal)
When the geometry builder computes the transformation matrix
Then it should use `glam::Mat4` for intermediate operations
And the result should be numerically equivalent to the defined physical-to-screen transformation.

### Requirement: Glam-based Base Coordinate System
The `Base` struct, representing a coordinate system (basis), MUST use `glam::Mat4` for its internal matrix representation.

#### Scenario: Base Structure Definition
- **WHEN** the `Base` struct is instantiated or used
- **THEN** it MUST contain a `glam::Mat4` field (column-major)
- **AND** geometric operations (scaling, translation, basis conversion) MUST use `glam`'s optimized methods.

### Requirement: Camera Matrices
The `Camera` struct MUST return `glam::Mat4` for `view_matrix`, `projection_matrix`, and `view_projection_matrix`.

#### Scenario: Camera Matrix Access
- **WHEN** accessing matrices from the `Camera` struct
- **THEN** the return type MUST be `glam::Mat4`.

### Requirement: Volume Orientation
Volume generation logic (DICOM, MHA) MUST construct orientation and transform matrices using `glam` directly.

#### Scenario: Constructing Orientation Matrix
- **WHEN** constructing a base matrix from voxel spacing and direction vectors (e.g. from DICOM)
- **THEN** use `Mat4::from_scale`, `Mat4::from_cols`, or `Mat4::from_cols_array` to build the matrix directly.

### Requirement: GeometricScalar Trait
The `GeometricScalar` trait MUST NOT expose conversion methods to/from `Matrix4x4`.

#### Scenario: Trait Implementation
- **WHEN** using the `GeometricScalar` trait
- **THEN** it MUST NOT provide methods to convert to or from the legacy `Matrix4x4` type.
