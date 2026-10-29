# Plan beam arrangement rendering

`data::beam::BeamOverlayGeometry` accepts explicit patient LPS isocenter, an
orthonormal collimator basis (+Z toward source), SAD in mm, optional jaw limits
and up to 2048 isocenter-plane aperture edges. The owner supplies association
and delivery semantics; no RTPLAN interpretation or egui dependency enters this API.

`rendering::beam::BeamOverlayContext` owns the MPR pipelines and three pane buffer
sets. Submit each complete visible batch with `prepare`, then `render` on the
host pass. The context validates all input before upload, clears prior ranges
on rejected preparation, and uses aligned dynamic UBO offsets and separate line
ranges per beam. The cap is 128 beams per pane; unused panes remain empty.

`BeamViewport` supplies a validated physical slice, clipping UVs and physical
pixel size. Divergent field boundaries use actual slice intersections. A parallel
off-slice axis may be shown as a dashed projection when explicitly enabled;
its caption and patient offset belong to the host. The selected axis is brighter
and thicker; visibility remains host-owned. Optional arc source directions draw
one projected, schematic orientation guide at a bounded radius. This curve is
not a true source path, slice contour, delivery interpolation or collision model.

`union_axis_aligned_outlines` returns an exact union boundary in one fixed beam
frame. It preserves disconnected openings and removes internal edges. Input is
capped at 8192 outlines, 2048 edges/outline and one million total edges; work is
capped at four million edge-row visits and output at 16384 edges. Rejection is
explicit. The host must ensure a common gantry/couch/collimator/isocenter frame.
`BevOverlayScene::envelope` draws a distinct dashed geometric coverage outline.
No fluence or MU weighting is inferred.

The existing [BEV renderer](bev.md) remains the sole qualitative DRR/MLC/contour
pipeline and shares the host CT allocation. These additions use WGPU 30 and no
new runtime dependency. Beam buffers grow on demand and are capped near 15.1 MiB
for three maximum batches, excluding CPU caches and transient upload staging.

Regression evidence: `cargo test --locked --test beam_arrangement_tests --test
bev_rendering_tests` verifies independent beam/pane uploads, stale-batch rejection,
recorded union semantics and existing projection/cache/encoding behavior. The egui
host additionally retains native/oblique/clipped aperture pixel regressions and
seven-beam selection/delivery tests. See the active
[verification record](../../openspec/changes/add-plan-beam-arrangement/verification.md).
