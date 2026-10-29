## ADDED Requirements

### Requirement: Batched plan beam geometry
Kepler SHALL draw up to 128 supplied conventional beam geometries in one MPR pane
batch, with independent uniforms and aperture ranges for each beam and pane.

#### Scenario: Selection among seven beams
- **WHEN** one of seven supplied beams is selected
- **THEN** all seven SHALL retain their supplied directions
- **AND** selection SHALL change only its display emphasis.

### Requirement: Physical slice intersections
Kepler SHALL intersect divergent aperture boundaries with the supplied physical
slice plane and SHALL distinguish an off-slice isocenter indicator from an on-slice ring.

#### Scenario: Clipped oblique image
- **WHEN** an oblique MPR image is clipped or zoomed
- **THEN** beam geometry SHALL follow the same physical plane and image UV mapping.

### Requirement: Invalid batch rejection
Kepler SHALL reject invalid geometry or excessive batch sizes and clear the affected
pane's previous draw ranges without rendering a partial batch.

#### Scenario: Oversized batch
- **WHEN** more than 128 beams are submitted
- **THEN** preparation SHALL return an error and that pane SHALL draw no stale geometry.

### Requirement: Recorded aperture union semantics
Kepler SHALL form a bounded exact union of axis-aligned recorded aperture outlines
in a supplied common isocenter frame, preserving disconnected openings and rejecting
invalid or excessive geometry without returning a partial result.

#### Scenario: Disjoint and overlapping openings
- **WHEN** two recorded openings are disjoint and a third overlaps one of them
- **THEN** the union SHALL retain the gap and omit internal overlap boundaries
- **AND** its display SHALL be a geometric outline rather than a fluence map.

### Requirement: Explicit off-slice projection
Kepler SHALL distinguish an enabled parallel off-slice axis projection using dashed
strokes from the solid in-plane axis and actual crossing markers.

#### Scenario: Browsing away from isocenter
- **WHEN** the slice moves parallel to a beam axis and off-slice projection is enabled
- **THEN** the projected axis SHALL be dashed
- **AND** disabling off-slice projection SHALL omit it.
