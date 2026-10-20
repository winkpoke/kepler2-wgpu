## ADDED Requirements

### Requirement: Original structure geometry and association
Kepler SHALL preserve original f64 patient LPS coordinates in mm, contour types, ROI numbers/names/colors and DICOM references. Input SHALL be limited to 32 MiB, 4096 ROIs, 50000 contours and 500000 points. Nonfinite, invalid-cardinality, degenerate closed, nonplanar planar and mixed XOR contours SHALL be rejected. Planarity tolerance SHALL be 0.01 mm.

#### Scenario: Serialized RTSTRUCT import
- **WHEN** a valid synthetic RTSTRUCT is decoded
- **THEN** geometry and references SHALL be preserved without GPU allocation or rasterization

#### Scenario: Association mismatch
- **WHEN** an ROI frame, referenced series or explicitly referenced image conflicts with the loaded CT
- **THEN** association SHALL fail before drawing

### Requirement: Source-plane contour overlay
Kepler SHALL expose an egui-independent single-sample GPU renderer using the host device and explicit color format. Geometry SHALL be shared across panes and uniforms SHALL be per pane. Stroke width SHALL be explicit in physical screen pixels, with alpha blending and clipping to the host image viewport.

#### Scenario: Matching physical plane
- **WHEN** a visible contour is parallel to the displayed plane and within 0.01 mm
- **THEN** its original outline SHALL be drawn with the ROI color

#### Scenario: Another MPR plane
- **WHEN** the plane is displaced or rotated away from the contour
- **THEN** that contour SHALL be omitted without implying reconstruction

#### Scenario: Host resize
- **WHEN** size changes on the same device and format
- **THEN** transforms SHALL update without recreating pipelines or uploading contour geometry
