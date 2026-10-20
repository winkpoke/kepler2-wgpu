# RTSTRUCT source-plane outlines

The additive `data::structures`, `data::dicom` RTSTRUCT functions and
`rendering::contour` interfaces support original contour display without egui,
window/surface ownership, filesystem scanning, or a full-volume mask.

## Import and association

`read_rtstruct(&[u8])` returns a `StructureSet` preserving original f64 patient
LPS coordinates in millimetres, contour kind, ROI numbers/names/colors and
references. Limits are 32 MiB, 4096 ROIs, 50000 contours and 500000 aggregate
points. Invalid cardinality, nonfinite coordinates, degenerate closed contours,
planarity errors over 0.01 mm, duplicate/undefined ROI numbers and mixed XOR
semantics are errors. Large Explicit-VR Contour Data encoded as UN is interpreted
as decimal text. Errors name the failed attribute or policy without echoing
patient identifiers.

`read_ct_image_with_frame` adds optional frame identity without changing existing
CTImage constructors. `StructureSet::validate_reference` checks ROI frames,
referenced series and explicitly referenced image UIDs against a
`DicomVolumeIdentity`. All ROIs must use the loaded frame. A frame-only match is
possible when no series/image references are provided; hosts must identify that
limited evidence. Missing CT frame identity or geometry should prevent automatic
attachment. The operation does not register data or choose a CT series.

## Host rendering

A host creates one `ContourRenderContext` per device/color-format configuration,
one immutable `ContourBatch` per structure revision, and a separate `ContourView`
for each pane/draw needing independent uniforms. Wrap pipeline/buffer creation in
host validation and out-of-memory scopes. Creation uses the explicit attachment
format and a single-sample pass without depth.

The host supplies a validated `StructurePlane` (top-left image boundary, unit
right/down directions, physical extents), full-image UV origin/span of the clipped
viewport, physical viewport pixels, stroke width in physical pixels, opacity,
and ROI visibility indexed in StructureSet order. `ContourView::update` prepares
uniforms and draw ranges. Set the matching viewport/scissor, draw the grayscale
image, call `ContourRenderContext::draw` in the same pass, then draw UI handles.
Reject invalid view/style input instead of drawing stale ranges. Reuse GPU
geometry across pan/zoom/resize; recreate resources when the owner device or
attachment format changes. Independent uniforms are required because queue
writes precede submission, rather than being snapshots at each draw command.

The shader draws antialiased round triangle strokes with premultiplied alpha.
Coordinates are rebased in f64 before conversion to GPU f32. The pipeline neither
samples nor changes the scalar volume, signed encoding or window/level shader.
ROI IDs are not packed into the fixed label palette, so overlaps remain separate.

Only coplanar original outlines are displayed, within 0.01 mm and with parallel
normals where a normal is defined. POINT, OPEN_PLANAR, CLOSED_PLANAR and
CLOSEDPLANAR_XOR outlines are supported. OPEN_NONPLANAR curves remain in the model
but are not drawn. There is no slab projection, interpolation between source
planes, hole filling, mask/mesh reconstruction, volume measurement, editing,
registration or export in this delivery. This is a prototype, not clinical
qualification.

## Verification

Synthetic serialized importer and reference tests are in
[src/data/dicom/rtstruct_tests.rs](../../src/data/dicom/rtstruct_tests.rs).
Geometry tests are in [structures.rs](../../src/data/structures.rs); shader and
uniform-layout tests are in [contour.rs](../../src/rendering/contour.rs).
The egui host additionally exercises native Metal pixels for source-plane
selection, independent pane uniforms, visibility and clipping, plus native UI
loading, search, Focus, zoom and source-plane navigation. Native and WASM library
type checks do not establish browser runtime or other-platform acceptance.
See the active change for the verification record and outstanding repository
baseline gates.
