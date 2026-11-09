## Interface

The host supplies bounded Part 10 byte slices from one selected series. The
builder checks cumulative input and voxel budgets. Finishing rechecks the
selected series UID and returns an i16 volume, separate LPS affine, frame and
instance identity, and optional window/level. Existing CT interfaces stay
source compatible. The host is responsible for filesystem access and cancellation.

## Geometry and intensities

Sort by the projection of Image Position (Patient) onto the slice normal.
Require common dimensions, spacing, frame and orientation, distinct planes,
and origins within 0.01 mm of the regular stack. Preserve original voxel
integers, sign-extend stored bits, and reject values outside i16 or nonidentity
modality mappings. MR display values have arbitrary signal units.
