# Verification — 2026-10-16

Environment: Apple M5 / Mac17,2, 24 GiB RAM, macOS 27.0 (26A428),
Rust 1.98.0. Native GPU execution uses Metal outside the restricted sandbox.
Fixtures are synthetic; no patient data was saved or committed.

## Passed focused checks

- `cargo test --locked --test nifti_import_tests`: 16 passed, including
  exact resulting CTVolume scalars/geometry, both byte orders, zero-slope policy,
  qform/qfac and sform provenance/selection, true inverse, shear rejection,
  offsets/singleton dimensions, truncation, gzip limits/concatenated members,
  native/bytes agreement, rectangular voxel access and MHA/MHD compatibility.
- `cargo test --locked --test nifti_rendering_tests`: 2 passed on Metal.
  The rotated/reflected asymmetric phantom verifies all three MPR orientations,
  screen/world round trips, slice clipping planes and an actual rendered mesh
  marker. Exact texel selection isolates geometry from interpolation precision.
  DVR is compared with an independently specified physical model.
- Existing `rendering_correctness_tests`: 14 passed; `volume_encoding_tests`:
  11 passed, including storage and filtering contracts.
- Existing `dicom_tests`: 8 passed. `coordinate_precision_tests`: 5 passed,
  4 already-ignored tests retained. Focused MPR unit tests: 7 passed; core
  geometry: 1 passed. These scoped runs total 64 passes, 4 ignored.
- Native `cargo check --locked --lib` passed. WASM
  `cargo check --locked --lib --target wasm32-unknown-unknown` passed with
  existing unused-import warnings. `wasm-pack build --target web --dev -- --locked`
  produced a temporary browser package.
- New importer, geometry and regression files pass scoped rustfmt checks.
- `npm run specs:check`: 15 items passed. `npm run docs:check`: 0 issues.

Kepler commands use an isolated CARGO_TARGET_DIR under /private/tmp. An earlier
run shared the egui target directory across different lockfiles and produced
incompatible cached glam/wgpu artifacts during concurrent builds. Final runs
isolate the repositories; the actual egui dependency graph contains one WGPU
30.0.1 generation. This cache failure was not a new dependency or source defect.

## Browser and app integration

A local Chrome harness imported both `.nii` and `.nii.gz` through the exported
WASM parse_common_files_wasm API using empty legacy slope/intercept info. Export
back to MHA confirmed all eight expected scaled i16 values, scanner LPS origin,
and orientation. The live browser MPR displayed the synthetic volume and its
public centre query returned (-99.75, -199.625, 301), matching the independent
fixture calculation. This covers one browser/backend session, not a cross-browser
matrix or browser DVR acceptance.

The egui app's 90 tests cover both extension dispatches, Kepler-volume GPU
eligibility, generic unknown-unit/HU readouts and existing display behavior.
Strict app Clippy, app formatting, all-target/all-feature check and release build
pass. GPU-required app verification uses KEPLER_VIEWER_REQUIRE_GPU=1.
The native release test app launched with Kepler GPU active. The test window
became user-controlled, so automated native file-picker interaction was stopped;
native picker acceptance remains manual. No screenshot of user data was saved.
Synthetic browser evidence is in the egui target/nifti-check directory.

## Broad baseline gates still fail

- Full `cargo check --locked --all-targets` cannot compile existing
  medical_imaging_tests: 11 references to retired MhaParser/MhdParser names.
  The new MetaimageParser compatibility regression passes using the current API.
- Full native library suite: 105 passed, 9 failed, 3 ignored. Existing failures
  concern window-level clamp expectations, DICOM UID expectations, camera/default
  mesh rotations and stale MIP/MPR uniform-size assertions. No uniform ABI was
  changed here. Existing application/layout GPU regressions passed on Metal.
- Full strict Kepler Clippy reports 90 existing diagnostics across unrelated
  legacy code. Newly introduced importer warnings were corrected; this is not a
  claim that the full lint gate passes.
- Full cargo fmt --check still reports unrelated legacy formatting. Formatting
  was applied to changed regions rather than reformatting the repository.

No clinical validation, production-readiness claim, arbitrary typed-storage
support, resampling, Windows/Linux runtime verification, large-volume throughput
benchmark or total RSS claim is made by these checks.
