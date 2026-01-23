## ADDED Requirements
### Requirement: MIP View Spin
The MIP view SHALL support 3D rotation via roll, yaw, and pitch inputs.

#### Scenario: User Rotates MIP View
- **WHEN** the user inputs a roll, yaw, or pitch angle
- **THEN** the MIP view updates the internal rotation state
- **AND** the rendered image reflects the rotated volume

### Requirement: MIP Shader Rotated Ray Casting
The MIP shader MUST support rotated orthographic rays by applying a rotation transform to ray generation and MUST compute correct ray-box intersection for non-axis-aligned rays.

#### Scenario: Rotated Ray Casting
- **WHEN** rotation parameters are provided via uniforms
- **THEN** the shader computes ray-box intersection for the rotated ray against the unit volume box
- **AND** the sampling coordinates remain within [0,1]^3 for valid hits
- **AND** the shader returns black output when the ray does not intersect the volume

### Requirement: MIP Uniform Layout Compatibility
The MIP uniform buffer layout MUST remain compatible between Rust and WGSL, and MUST maintain 16-byte alignment requirements on all targets, including wasm.

#### Scenario: Uniform Layout Validation
- **WHEN** the MIP uniforms are updated to include rotation parameters
- **THEN** the uniform struct size remains a multiple of 16 bytes
- **AND** runtime binding uses the full struct size without undefined padding reads
