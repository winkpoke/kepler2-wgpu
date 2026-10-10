# Rendering Spec Delta

## ADDED Requirements

### Requirement: Browser GPU Backend Selection

Browser startup SHALL prefer usable WebGPU and attempt WebGL2 when WebGPU detection or pre-canvas adapter/device negotiation fails. Selection SHALL finish before binding the application canvas. Failures after binding SHALL report the backend and failing stage without attempting another context type on that canvas.

#### Scenario: Usable WebGPU
- **WHEN** browser WebGPU detection, adapter/device negotiation, and pre-canvas compatibility checks succeed
- **THEN** the app SHALL select BrowserWebGpu
- **AND** normal rendering and controls SHALL remain available

#### Scenario: WebGPU unavailable
- **WHEN** WebGPU detection finds no usable adapter
- **THEN** the app SHALL attempt WebGL2
- **AND** successful startup SHALL report Gl as the selected backend

#### Scenario: WebGPU negotiation failure
- **WHEN** WebGPU is detected but its pre-canvas adapter/device or required-capability negotiation fails
- **THEN** the app SHALL make one WebGL2 initialization attempt
- **AND** diagnostics SHALL identify the failed WebGPU negotiation stage
- **AND** the real canvas SHALL not already have acquired a WebGPU context

#### Scenario: Both candidates fail
- **WHEN** neither browser backend can complete negotiation
- **THEN** startup SHALL return an actionable graphics error identifying the attempted backends and failing stages
- **AND** startup SHALL not retry indefinitely

#### Scenario: Failure after canvas binding
- **WHEN** a surface or pipeline fails after the real canvas has acquired a graphics context
- **THEN** the app SHALL report the selected backend and failing stage
- **AND** it SHALL not attempt to switch that canvas to another API
- **AND** recovery guidance SHALL describe a fresh-page WebGL-only startup

#### Scenario: Explicit WebGL diagnostic mode
- **WHEN** a fresh page starts with `kepler_wgpu_backend=gl` in its query string
- **THEN** startup SHALL attempt only WebGL2 without WebGPU negotiation
- **AND** unknown selector values SHALL warn and retain automatic preference
- **AND** logs SHALL not include unrelated URL parameters

#### Scenario: Native compatibility
- **WHEN** a native application initializes graphics
- **THEN** existing native default selection, backend environment overrides, and validation settings SHALL remain unchanged

### Requirement: Device-Negotiated Volume Capabilities

Volume encoding selection SHALL depend on usable filtered texture formats of the selected device, not platform/backend identity alone. A usable format MUST permit sampled textures, upload destination usage, and filtering under enabled device features. Initialization SHALL reject devices with neither usable R16Float nor packed RG8. Diagnostics SHALL report actual backend, adapter, effective 3D limit, and default encoding.

#### Scenario: Half-float preference
- **WHEN** R16Float permits filtered sampling and upload under the selected device's enabled features
- **THEN** default volume encoding SHALL be R16Float
- **AND** existing half-float rounding semantics SHALL remain unchanged

#### Scenario: Exact packed compatibility path
- **WHEN** R16Float is unusable but RG8 permits filtered sampling and upload
- **THEN** default volume encoding SHALL be PackedI16Rg8
- **AND** signed i16 storage SHALL retain the existing exact offset-binary representation
- **AND** window/level SHALL remain independent of the storage offset

#### Scenario: No usable volume format
- **WHEN** neither R16Float nor packed RG8 supports the required usages and filtering under enabled device features
- **THEN** the candidate SHALL fail negotiation with an actionable capability error
- **AND** a pre-canvas browser WebGPU candidate SHALL permit the bounded WebGL attempt

#### Scenario: Graphics replacement and manual encoding toggle
- **WHEN** graphics are replaced or a user enables float volume textures
- **THEN** the decision SHALL use the destination device's usable formats
- **AND** unsupported float encoding SHALL not be enabled
- **AND** a user's existing packed preference and CPU view interaction state SHALL remain preserved during graphics replacement

### Requirement: Browser Volume Limits and Upload Rejection

Browser initialization SHALL negotiate supported 3D texture limits rather than requiring 1024 voxels per axis unconditionally. Volume uploads MUST reject zero dimensions, axes exceeding the effective device limit, byte-count overflow, or mismatched two-byte-per-voxel input before GPU allocation. Rejections SHALL not alter canonical voxels.

#### Scenario: Lower supported 3D limit
- **WHEN** an otherwise compatible browser adapter supports fewer than 1024 voxels per 3D texture axis
- **THEN** initialization SHALL not fail solely because of that lower limit
- **AND** valid volumes within the negotiated limit SHALL remain uploadable

#### Scenario: Oversized or empty volume dimensions
- **WHEN** a requested volume axis is zero or greater than the effective device 3D limit in voxels
- **THEN** upload SHALL return a dimension/limit error before texture allocation
- **AND** it SHALL not silently clamp, truncate, or resample the volume

#### Scenario: Malformed byte count
- **WHEN** width times height times depth times two bytes overflows or differs from the supplied buffer length
- **THEN** upload SHALL return an actionable validation error before GPU allocation

#### Scenario: Valid upload at the limit
- **WHEN** each axis is nonzero and no greater than the effective device limit and the byte count is correct
- **THEN** validation SHALL permit upload for either usable encoding
- **AND** existing decode and display contracts SHALL remain unchanged

## Verification

Planned focused capability/fallback/upload regressions are not yet implemented. Existing [encoding regressions](../../../../../tests/volume_encoding_tests.rs) and [native pixel regressions](../../../../../tests/rendering_correctness_tests.rs) remain mandatory regression evidence, but do not prove browser behavior.

Manual browser checks and recording criteria are listed in [the design](../../design.md) and tracked separately in [tasks](../../tasks.md). WebGPU, unavailable-WebGPU fallback, forced WebGL, and pre-canvas failed-WebGPU fallback remain unverified until actual browser execution. Follow [the build matrix](../../../../../doc/agents/BUILD.md).
