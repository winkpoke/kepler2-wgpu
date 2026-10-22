## ADDED Requirements

### Requirement: Source looking physical beam projection
Kepler SHALL project patient LPS points from an explicitly supplied source onto
the collimator isocenter plane in millimetres, rejecting invalid geometry.

#### Scenario: Perspective magnification
- **WHEN** a point is halfway between source and isocenter
- **THEN** its projected lateral coordinates SHALL be twice its lateral displacement
- **AND** points on or behind the source plane SHALL have no projection.

### Requirement: Physical voxel mapping
Kepler SHALL retain anisotropic, reflected and oblique voxel-center geometry and
the host's normalized texture permutation without assuming display and native axes match.

#### Scenario: Reflected native upload
- **WHEN** display X is reversed relative to the resident texture
- **THEN** ray sampling SHALL address the corresponding native voxel centers.

### Requirement: Bounded qualitative CT projection
Kepler SHALL integrate nonnegative HU-derived relative-density length over the
entire physical ray/CT intersection on the host's existing GPU texture and device.

#### Scenario: Step budget
- **WHEN** the requested sample count exceeds the configured bounded budget
- **THEN** the step length SHALL increase to cover the full intersection
- **AND** no CT segment SHALL be omitted because the loop budget was reached.

#### Scenario: Independent contrast
- **WHEN** only the contrast coefficient changes
- **THEN** the integral SHALL be reused and only display mapping SHALL be repeated.

### Requirement: Original geometry projection semantics
Kepler SHALL distinguish projected original contour segments from reconstructed
silhouettes and preserve closed/crossed leaf blocking for presentation geometry.

#### Scenario: Contour point admission
- **WHEN** projected input exceeds the configured point limit
- **THEN** an explicit error SHALL be returned without silently truncating the ROI.
