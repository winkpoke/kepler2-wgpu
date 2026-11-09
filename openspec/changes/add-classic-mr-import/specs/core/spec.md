## ADDED Requirements

### Requirement: Classic MR series import

Kepler SHALL accept a bounded explicitly selected classic MR series of uncompressed
little-endian single-frame MONOCHROME2 images with 16-bit allocated samples and
values representable as i16. It SHALL return original DICOM frame, series and
instance identity and a validated voxel-centre transform in patient LPS mm.

#### Scenario: Reversed oblique inputs

- **WHEN** valid slices from one stack arrive in reverse or arbitrary order
- **THEN** the importer orders them by position projected onto their slice normal and preserves their stored intensities and instance identities

#### Scenario: Invalid stack or unsupported encoding

- **WHEN** the input exceeds a budget or includes incompatible geometry, duplicate planes, unsupported encoding or values outside the supported range
- **THEN** the importer returns an error and supplies no assembled volume
