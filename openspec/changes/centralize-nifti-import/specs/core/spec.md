## ADDED Requirements

### Requirement: Complete NIfTI scalar volume import
Kepler SHALL own single-file NIfTI-1 parsing, validation, conversion and geometry
for native and WASM callers, producing canonical i16 CTVolume data and diagnostics.

#### Scenario: Supported scalar image
- **WHEN** a supported endian/scalar image has positive spatial dimensions and only singleton extra dimensions
- **THEN** the importer SHALL validate payload extent and bounded input/expansion/output before construction
- **AND** apply declared scaling, ties-away rounding and saturating i16 conversion without a -1024 floor
- **AND** zero slope SHALL ignore the intercept and non-finite scaled voxels SHALL fail

#### Scenario: Single-file offset normalization
- **WHEN** a finite representable vox_offset is below 352
- **THEN** the importer SHALL use byte 352
- **AND** unaligned integral byte positions SHALL not fail solely for lack of 16-byte alignment

#### Scenario: Resolved geometry
- **WHEN** valid qform/sform transforms are declared
- **THEN** Kepler SHALL preserve both codes/matrices and resolve the documented scanner preference
- **AND** retain reference-space identity after RAS-to-LPS conversion and known-unit conversion to mm
- **AND** conflicting scanner forms beyond 0.01 mm at voxel-centre corners SHALL fail
- **AND** singular, non-finite or selected sheared render geometry SHALL fail explicitly

#### Scenario: Unknown reference frame
- **WHEN** spatial units or orientation are unknown, or the selected space is aligned/atlas/template
- **THEN** the result SHALL distinguish those properties from scanner patient LPS millimetres
- **AND** callers SHALL NOT need to parse NIfTI headers to decide display labels

### Requirement: Affine-consistent volume view geometry
MPR and DVR source-aware views SHALL preserve the volume affine through centring,
texture sampling, and associated world/UV overlay transforms.

#### Scenario: Rotated or reflected source
- **WHEN** source geometry rotates or reflects voxel axes
- **THEN** MPR centre SHALL be the full affine of (dimensions-1)/2
- **AND** texture centre (index+0.5)/dimensions SHALL map to the same voxel centre
- **AND** screen picking and overlay coordinates SHALL agree with rendered sampling
- **AND** source-aware DVR and UV-space meshes SHALL share one normalized physical model
