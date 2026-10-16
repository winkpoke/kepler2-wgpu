# Data Layer Overview (src/data)

This folder contains medical imaging data structures, decoding/loading logic, and format-specific modules.

## Main Areas
- CT volume representation:
  - `src/data/ct_volume.rs`
- DICOM support:
  - `src/data/dicom/*` (repo, file IO helpers, patient/study/series structures, export helpers where applicable)
- Medical imaging formats (non-DICOM):
  - `src/data/medical_imaging/formats/*` (e.g., MHA/MHD parsing)
  - `src/data/medical_imaging/metadata/*` (image/volume/pixel metadata)
  - `src/data/medical_imaging/validation.rs` (sanity checks at boundaries)
- Encoding/transfer helpers:
  - `src/data/volume_encoding.rs`

## Boundaries and responsibilities
- Parsing/decoding should validate inputs at module boundaries and return typed errors (no panics for malformed input).
- Rendering should receive prepared/validated data (e.g., volume dimensions, spacing, pixel type) rather than raw file bytes.

## Typical flow (high level)
1. Load bytes from disk (native) or browser-provided sources (wasm).
2. Decode into typed metadata + pixel buffers.
3. Validate volume invariants (dimensions, spacing, pixel stride).
4. Pass to rendering/view layer as ready-to-use volume/texture inputs.