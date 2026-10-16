# Design

Approved scope: the owner's 2026-10-16 instruction implements the preceding
reviewed recommendation in both repositories. No additional permission is needed
for the proposed geometry/scalar policies.

The data/medical_imaging NIfTI module owns validation, bounded gzip expansion,
scaling, i16 conversion and qform/sform resolution. Its additive complete-import
API returns CTVolume plus provenance and diagnostics. Legacy APIs are retained.
Native filesystem access is cfg-gated; WASM file handling supplies bytes to the
same importer. The egui app contains only file dispatch and generic display glue.

The authoritative selected affine maps voxel centres to LPS axes, mm when known.
Scanner preference, ambiguity tolerance, zero-slope behavior, rounding,
saturation, non-finite rejection, singleton dimensions and offset normalization
are specified in doc/fileio/nifti.md. General inverse correctness is independent
of the initial rendering restriction on shear. Unknown space/units never gain
patient-mm labels. CPU storage remains i16, independently of display settings.

Core UV bases include the half-voxel boundary; MPR centres and projected extents
use the full affine. DVR/UV meshes share a normalized physical model. Public
content-only factory signatures remain unchanged. Existing MHA/DICOM constructors
and their scalar policies are not broadened.

Alternative rejected: keep app-side format fixes, which leaves native/WASM
behavior divergent. Arbitrary voxel storage and resampling are separate designs.
Rollback can restore old callers without deleting legacy APIs, but would also
restore the documented correctness defects and is not a supported fallback.
