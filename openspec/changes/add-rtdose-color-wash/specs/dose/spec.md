## ADDED Requirements

### Requirement: Quantitative dose import
Kepler SHALL import bounded uncompressed multiframe PHYSICAL RTDOSE in GY or RELATIVE units, retain original unsigned 16/32-bit samples and scaling, and preserve physical grid positions.

#### Scenario: Nonuniform and descending frame positions
- **WHEN** valid relative or axial absolute offsets are supplied, including descending or nonuniform spacing
- **THEN** sample values SHALL retain their physical positions after canonicalization
- **AND** quantitative point sampling SHALL use original samples and f64 interpolation

#### Scenario: Unsupported or malformed input
- **WHEN** dimensions, pixel length, scaling, orientation or offsets are invalid, or pixel semantics are unsupported
- **THEN** import SHALL return an explicit error without CT conversion or truncation

### Requirement: Host-owned dose display
The dose renderer SHALL share the host device and color pass and display an independent GPU color wash below contours and UI handles.

#### Scenario: Slice sampling and independent panes
- **WHEN** a host displays a physical axial, coronal, sagittal or oblique slice
- **THEN** the renderer SHALL interpolate the original dose grid using physical frame distances
- **AND** out-of-grid pixels SHALL be transparent
- **AND** pane uniforms SHALL remain independent

#### Scenario: Display versus quantitative precision
- **WHEN** the GPU uses an approximate R32Float display copy
- **THEN** authoritative integer samples and scaling SHALL remain available unchanged
- **AND** deterministic synthetic pixels SHALL agree with independent reference interpolation within two 8-bit color levels away from domain edges

### Requirement: Verified dose association
Hosts SHALL permit dose metadata inspection independently and require validated CT geometry and matching Frame of Reference before overlay.

#### Scenario: Mismatch or source replacement
- **WHEN** CT geometry or identity is invalid, references conflict, or the source changes during loading
- **THEN** dose SHALL NOT be overlaid on the CT
- **AND** source replacement SHALL clear attached dose resources
