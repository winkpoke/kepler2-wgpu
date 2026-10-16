## ADDED Requirements
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
