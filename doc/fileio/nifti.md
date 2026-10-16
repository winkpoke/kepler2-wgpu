# NIfTI-1 import contract

Kepler owns NIfTI interpretation. Native `open_nifti` and cross-platform
`import_nifti_bytes` return `ImportedNifti` with a resident canonical i16
`CTVolume`, geometry provenance, and conversion diagnostics. Callers select
files, schedule work, wrap results, and display diagnostics; they do not reparse
headers or reinterpret transform/scaling fields.

## Accepted input and limits

Single-file NIfTI-1 (`n+1\0`) `.nii` and `.nii.gz` scalar images are supported.
Rank 2..7 is accepted when every active dimension is positive and dimensions
4..7 are singleton. Non-singleton time/vector dimensions, paired hdr/img,
NIfTI-2, complex/RGB, and unsupported scalar types return errors. Datatype and
bitpix must agree. Checked payload extent is validated before construction;
extra bytes after the declared payload are not treated as extra voxels.

Finite single-file vox_offset is interpreted as an integer byte offset and
normalized to at least 352. Alignment to 16 bytes is advisory. NaN/infinity,
unrepresentable offsets, overflow, and truncated payloads are errors.

Defaults limit input, expanded stream, and converted voxels to 512 MiB each.
These are per-buffer limits, not a total process RSS guarantee; native gzip can
retain input + expanded bytes + output concurrently. Limits are configurable.
Gzip expansion is checked in bounded chunks, including concatenated members
and checksum/end-of-stream validation. Raw byte imports borrow input; native
input is read once and gzip is decompressed once. GPU limits remain separate.

## Scalars

Supported stored types: u8, i8, i16, u16, i32, f32, f64 in either byte order.
Conversion decodes the stored value, computes slope*x+intercept in f64, rounds
ties away from zero, then saturates to i16. Zero slope ignores the intercept
and disables scaling. Active non-finite scaling and non-finite scaled voxels
are errors. Rounded/clipped voxel counts are reported. No -1024 floor or
window/level adjustment is applied. NIfTI values are not automatically HU.

The legacy NiftiParser signatures remain available with bounded input and
validated payloads. Their MedicalVolume payload is unscaled; new rendering
callers must use the complete importer. Legacy i8 is expanded into Int16 since
the public PixelType API has no i8 variant. MedicalVolume::new and MHA/DICOM
scalar conversion policies are unchanged.

## Geometry and space identity

Both original RAS qform/sform matrices and codes are retained. Qform includes
quaternion rotation, pixdim, qfac reflection, and qoffset. Sform is a full affine
and is never multiplied by pixdim. Declared metres/micrometres convert to mm.
Known transforms convert RAS axes to LPS by negating world X/Y, including origin.
That axis conversion does not change reference-space identity.

Selection policy for this medical viewer: prefer a scanner-coded qform, then a
scanner-coded sform, then another valid sform, then qform. Two scanner-coded
transforms must agree within 0.01 mm at all voxel-centre corners (0.01 declared
units when units are unknown); otherwise import fails as ambiguous. This is a
documented viewer policy, not a universal NIfTI precedence rule. Aligned,
Talairach, MNI, template, unknown, and unrecognized spaces retain their identity.
Scanner classification reflects the header declaration, not physical validation.

CTVolume::base maps integer voxel centres to the selected world frame. A true,
checked inverse is retained. Unknown form codes use spacing-only voxel axes;
unknown units remain unknown, never relabelled mm. Only scanner-coded geometry
with known units is exposed by patient_lps_affine. Patient posture stays unknown.
Selected shear is explicitly unsupported for rendering import; metadata parsing
retains the full affine and its coordinate helper uses a true inverse. Singular
or non-finite declared transforms fail before volume construction.

Texture coordinates (i+0.5)/N map to voxel centre i, so the UV-to-world base is
A * translate(-0.5) * scale(N). MPR centres use A*((N-1)/2), and plane extents
use projected physical box edges. Initial and updated sampling use one inverse.
MPR overlays and picking use the same world/screen bases. DVR views supplied
with a CTVolume retain axis direction/relative spacing in a uniformly normalized,
centred camera frame; UV-space meshes receive the same model. The public
content-only factory remains compatible and requires its canonical source owner
to attach geometry through MeshView::set_volume_geometry.

## Verification

- [Import regressions](../../tests/nifti_import_tests.rs): public import results,
  endian/scaling, transforms/inverses, malformed input and resource limits.
- [Rendered regressions](../../tests/nifti_rendering_tests.rs): asymmetric
  rotated/reflected phantom through production MPR, overlay coordinates and DVR.
- [Core geometry](../../src/core/geometry.rs): centre and half-voxel consistency.
- [Browser procedure](../testing/browser_gpu.md): browser runtime acceptance is
  separate from a successful wasm32 library check.

References: [official header](https://raw.githubusercontent.com/NIFTI-Imaging/nifti_clib/master/nifti2/nifti1.h)
and [qform/sform documentation](https://nifti.nimh.nih.gov/nifti-1/documentation/nifti1fields/nifti1fields_pages/qsform.html/document_view.html).
