## ADDED Requirements

### Requirement: Port Graphics Corrections Into Release Production Renderers

The candidate SHALL port the requested source graphics improvements into the
release's production MPR, projection, DVR, multi-mesh, segmentation, and overlay
paths while preserving published interfaces and separately reviewed behavior.

#### Scenario: A source patch modifies a renderer removed by the release

- **WHEN** a replay patch targets a deleted shader, camera, or mesh implementation
- **THEN** its necessary correction SHALL be applied to the actual release replacement
- **AND** tests SHALL instantiate the production replacement rather than validate only an orphaned old shader
- **AND** release rendering features and public controls SHALL remain available

#### Scenario: Signed CT and one-byte content are uploaded

- **WHEN** canonical signed CT, legacy offset-packed bytes, R8 scalar data, or R8Uint labels are uploaded
- **THEN** content validation and decoding SHALL distinguish their formats and semantic kinds
- **AND** canonical signed packing SHALL preserve the full i16 domain with a sufficiently wide intermediate
- **AND** the published encoding variants and supplied legacy offset interpretation SHALL remain compatible
- **AND** storage decoding SHALL remain independent of display window/level

#### Scenario: Device or attachment format changes

- **WHEN** viewer resources change device or attachment format
- **THEN** every affected release resource SHALL be rebuilt for the selected owner/configuration
- **AND** compatible ordinary resize SHALL NOT rebuild pipelines unnecessarily
- **AND** server compute and viewer resources SHALL remain separate device owners
- **AND** retained public compatibility helpers SHALL NOT become the internal viewer's global format authority

#### Scenario: Shader sampling is adapted for browser and native backends

- **WHEN** implicit derivatives or ray-loop sampling are corrected
- **THEN** the actual production shaders SHALL pass backend validation and independent pixel checks
- **AND** sampler/filter behavior and any proposed display-math change SHALL be reviewed explicitly
- **AND** fixed LOD zero SHALL NOT replace a footprint-preserving correction without an equivalent behavioral proof

### Requirement: Preserve Release State Through Private Lifecycle Improvements

Layout, frame, invalidation, and lifecycle improvements SHALL preserve the release
feature state and public dispatch contracts.

#### Scenario: Layout preparation fails or graphics are rebound

- **WHEN** a fallible layout operation fails or a graphics configuration is replaced
- **THEN** the application SHALL preserve the applicable CPU/control state and rebuild affected GPU state consistently
- **AND** overlay, segmentation, multi-mesh, primitive, and oblique state SHALL be included
- **AND** getters and nonvisual no-ops SHALL NOT introduce continuous idle redraw
