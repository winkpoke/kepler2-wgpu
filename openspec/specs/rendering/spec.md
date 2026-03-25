# Rendering Specification

## Purpose

Defines requirements for GPU rendering, shader pipelines, view correctness, and performance characteristics of medical imaging visualization.
## Requirements
### Requirement: Rendering Testing

The rendering subsystem SHALL have comprehensive test coverage for GPU operations, view management, and visual correctness to ensure reliable medical imaging visualization.

#### Scenario: GPU Pipeline Creation Validation
- **WHEN** a WGPU device or pipeline is created
- **THEN** it SHALL be validated for success
- **AND** creation failures SHALL return clear, actionable errors
- **AND** this SHALL only apply to native builds (#[cfg(not(target_arch = "wasm32"))])

#### Scenario: Texture Management Validation
- **WHEN** textures are created, updated, or destroyed
- **THEN** operations SHALL be tested for correct behavior
- **AND** dimension mismatches SHALL be caught
- **AND** GPU memory SHALL be properly cleaned up on destruction
- **AND** format conversions SHALL preserve data integrity

#### Scenario: View Manager State Consistency
- **WHEN** views are added, removed, or switched
- **THEN** view manager SHALL maintain consistent internal state
- **AND** concurrent operations SHALL be thread-safe
- **AND** removed views SHALL be fully cleaned up (no memory leaks)
- **AND** view counts SHALL always be accurate
- **AND** active view switching SHALL work without state loss

#### Scenario: Visual Correctness Validation
- **WHEN** window/level settings are applied
- **THEN** output SHALL preserve visual fidelity
- **AND** clamping SHALL prevent invalid values
- **AND** aspect ratios SHALL be preserved (no stretching)
- **AND** extreme values SHALL be handled gracefully

#### Scenario: GPU Error Handling
- **WHEN** GPU operations fail (allocation, shader compilation, surface creation)
- **THEN** errors SHALL be properly propagated to user
- **AND** error messages SHALL include relevant details (dimensions, memory limits)
- **AND** no GPU resources SHALL be leaked after errors
- **AND** system SHALL NOT crash or enter undefined state

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
- **WHEN** the MIP uniforms are updated (including adding rotation or mode parameters)
- **THEN** the uniform struct size remains a multiple of 16 bytes
- **AND** runtime binding uses the full struct size without undefined padding reads

### Requirement: MPR View Transformations

The MPR view SHALL apply pan/scale transforms consistently and SHALL upload transforms to the GPU in a deterministic layout.

#### Scenario: Column-major matrix upload
- **WHEN** the MPR transform matrix is uploaded as `[f32; 16]`
- **THEN** it SHALL be treated as column-major by both CPU and shader code
- **AND** the rendered slice SHALL remain stable across native and wasm targets

### Requirement: Performance Benchmarks

The rendering subsystem SHALL be measured against defined baselines and SHALL detect significant regressions.

#### Scenario: Performance baselines
- **WHEN** critical operations are executed (DICOM parsing, volume rendering, frame timing)
- **THEN** performance SHALL be measured against defined baselines
- **AND** DICOM parsing SHALL complete in < 10ms for 512×512 images
- **AND** volume rendering SHALL achieve ≥ 60 FPS (16ms per frame) for 512×512 views
- **AND** memory usage SHALL be tracked and bounded (< 2GB for typical volumes)
- **AND** regressions SHALL be detected (>50% performance degradation)

### Requirement: MinIP and AvgIP Rendering Modes
The MIP view shader SHALL support Minimum Intensity Projection (MinIP) and Average Intensity Projection (AvgIP) in addition to Maximum Intensity Projection (MIP).

#### Scenario: User Selects MinIP Mode
- **WHEN** the user selects MinIP mode (mode 1)
- **THEN** the shader SHALL compute the minimum intensity along the ray segment intersecting the volume
- **AND** the initial minimum intensity SHALL be initialized to a large positive value (e.g., +infinity) to support signed data (HU)
- **AND** the final pixel value SHALL be the minimum encountered value, preserving negative values
- **AND** rays not intersecting the volume SHALL return the background color
- **AND** the shader SHALL apply configurable lower and upper intensity thresholds to filter contributions based on tissue density (e.g., excluding background air or specific tissue ranges)

#### Scenario: User Selects AvgIP Mode
- **WHEN** the user selects AvgIP mode (mode 2)
- **THEN** the shader SHALL compute the average intensity along the ray segment intersecting the volume
- **AND** the sum of intensities SHALL be divided by the number of valid samples (arithmetic mean)
- **AND** the final pixel value SHALL be the computed average, preserving signed values
- **AND** rays not intersecting the volume SHALL return the background color
- **AND** the shader SHALL apply configurable lower and upper intensity thresholds to filter contributions based on tissue density (e.g., excluding background air or bone)

#### Scenario: User Selects MIP Mode
- **WHEN** the user selects MIP mode (mode 0)
- **THEN** the shader SHALL compute the maximum intensity along the ray (existing behavior)
- **AND** the initial maximum intensity SHALL be initialized to a small negative value (e.g., -infinity) to support signed data (HU)
- **AND** the final pixel value SHALL be the maximum encountered value, preserving negative values
- **AND** the shader SHALL apply configurable lower and upper intensity thresholds to filter contributions based on tissue density

### Requirement: 3D Volume Rendering View
The 3D view SHALL support GPU volume rendering of CT data using ray-marched sampling through the volume texture.

#### Scenario: Render volume in 3D view
- **WHEN** the user activates volume rendering in the 3D view
- **THEN** the renderer SHALL sample the volume texture along orthographic rays
- **AND** the output SHALL respect the configured step size and opacity settings
- **AND** rays that miss the volume SHALL render the background color

### Requirement: Volume Rendering Controls
The 3D view SHALL expose minimal controls for volume rendering that influence ray-march behavior without blocking the render loop.

#### Scenario: Adjust volume rendering parameters
- **WHEN** the user updates step size or opacity window parameters
- **THEN** the renderer SHALL update uniforms without reallocating the volume texture
- **AND** the change SHALL take effect on the next frame

