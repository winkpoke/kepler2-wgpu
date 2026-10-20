# RTDOSE grid and color wash

`data::dicom::read_rt_dose(bytes)` imports an uncompressed multiframe PHYSICAL
RTDOSE. Limits: 256 MiB input, 64 × 1024 × 1024 voxels; native LE/BE 16/32-bit
unsigned samples, GY or RELATIVE units. Invalid dimensions, bit layouts, pixel
length, scaling, geometry or monotonic offsets return errors. Single-frame,
compressed, effective/error/coded data are explicitly unsupported.

`data::dose::DoseGrid` owns original integers and f64 scaling. `sample_lps`
returns a quantitative interpolated point or None outside the sample-center
domain. DoseGeometry uses LPS mm and explicit physical offsets; the importer
normalizes the relative/axial absolute conventions and descending frame order
together with pixel frames. Grid maximum is never a prescription reference.

`rendering::dose::{DoseRenderContext,DoseTexture,DoseView}` accepts a host device
and attachment format. The host wraps construction in allocation/validation
scopes, sets viewport/scissor, and draws in its single-sample color pass without
depth, after CT and before contours. DoseTexture contains an approximate
R32Float copy and frame-offset buffer; original integers remain authoritative.
DoseView gets a shared `data::slice::SlicePlane` in patient coordinates and
independent style/UV parameters per pane. It rebases in f64 before f32 uniforms.
The shader performs physical Z bracketing and explicit neighbor interpolation,
requiring no optional float32-filtering feature. Invalid updates disable drawing.

The host owns file I/O, CT association, source revision, UI controls and memory
admission. Match Frame of Reference plus validated geometry before overlay; do
not equate CT and dose series IDs or dimensions. Keep metadata inspection
available independently. Clear attached resources when changing CT. The viewer
counts encoded CT, dose texture and dose staging against a 2 GiB admission
policy, which is not total GPU/process memory. No whole-volume CT-grid resampling
is needed. Isodose lines, DVH, summation, calculation and registration are deferred.

Verification is recorded in the [change](../../openspec/changes/add-rtdose-color-wash/verification.md).
