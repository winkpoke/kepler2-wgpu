## MODIFIED Requirements
### Requirement: Dual Orthogonal MPR Rendering

The multi-planar reconstruction (MPR) view SHALL support rendering two orthogonal CT slices simultaneously within a single canvas using overlay composition, where the axial slice is the primary layer and the sagittal slice is blended on top.

#### Scenario: Overlay composition in one canvas
- **WHEN** dual orthogonal MPR mode is activated
- **THEN** the renderer SHALL draw both axial and sagittal slices in the same full-canvas output
- **AND** the renderer SHALL NOT split the canvas into side-by-side or multi-viewport regions
- **AND** both slices SHALL remain spatially consistent in patient coordinates

#### Scenario: Axial base with sagittal transparency
- **WHEN** the final pixel color is composed for dual orthogonal MPR mode
- **THEN** the axial slice SHALL be evaluated as the base layer
- **AND** the sagittal slice SHALL be blended on top using alpha/transparency composition
- **AND** sagittal overlay opacity SHALL use a safe default and MAY be adjusted by user-controlled opacity settings

#### Scenario: Transfer function and window-level continuity
- **WHEN** the renderer evaluates intensity-to-color mapping for dual orthogonal overlay
- **THEN** each slice SHALL apply transfer function and window/level mapping before blend composition
- **AND** blend composition SHALL preserve visibility of both slices without breaking existing tissue contrast interpretation

#### Scenario: Minimal pipeline disruption and performance
- **WHEN** dual orthogonal overlay mode is implemented
- **THEN** the shader and pipeline changes SHALL be minimal and compatible with existing uniform/buffer layout constraints
- **AND** uniform layouts SHALL preserve 16-byte alignment compatibility across native and wasm builds
- **AND** the implementation SHALL avoid additional synchronization or extra passes that materially degrade render-loop performance
