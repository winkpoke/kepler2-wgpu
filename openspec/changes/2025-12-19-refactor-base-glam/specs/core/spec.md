# Core Specification: Coordinate Systems

## Purpose
Defines the fundamental coordinate system types and transformations used throughout the application.

## Requirements

### Requirement: Glam-based Base Coordinate System
The `Base` struct, representing a coordinate system (basis), MUST use `glam::Mat4` for its internal matrix representation.
- **Type**: `Base` (concrete, standardized on `f32`).
- **Matrix**: `glam::Mat4` (column-major).
- **Operations**: Geometric operations (scaling, translation, basis conversion) MUST use `glam`'s optimized methods.

### Requirement: Row-Major vs Column-Major
- **Internal**: `glam::Mat4` is column-major.
- **External/Legacy**: Any legacy row-major data sources (e.g., raw arrays from DICOM or tests) MUST be explicitly converted/transposed when loading into `Base`.
- **Shaders**: WGPU/WGSL expects column-major matrices by default. The `Base.matrix` can be written directly to uniform buffers without transposition (unless the previous implementation was pre-transposing for row-major logic).
