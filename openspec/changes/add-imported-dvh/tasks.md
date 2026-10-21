## Implementation
- [x] Add validated cumulative histogram model and f64 query semantics.
- [x] Decode optional source histograms with independent diagnostics and bounds.
- [x] Add synthetic model/parser/reference regression tests.
- [x] Integrate source-aware chart and metrics in the host viewer.

## Verification
- [x] Run focused library tests, native and WASM type checks.
- [x] Run relevant lint/format and document baseline failures separately.
- [x] Run OpenSpec and documentation validation.
- [x] Verify viewer tests, native UI and release-size/startup measurements.

## Evidence and limits
- Ten focused DVH tests pass. Viewer native Metal suite: 135 passed, eight explicit
  fixture/local-data probes ignored; strict viewer Clippy/no-default-feature checks pass.
- Exact 15-curve local export association/metrics probe passes without logging identifiers.
- Kepler native library and WASM library type checks pass (WASM retains eight existing warnings).
- Full library tests: nine failures reproduced on clean HEAD; candidate initially
  127 passed versus baseline 120 passed, both nine failed/three ignored. Ten focused
  DVH tests include three additional cases added after that full-suite comparison.
- Strict Clippy: candidate and clean HEAD both have the same 90 errors, no candidate-only diagnostics.
- All-targets types fail in eleven legacy MhaParser/MhdParser uses in medical_imaging_tests,
  reproduced on clean HEAD. Scoped Rust 2021 formatting passes; unrelated global differences remain.
- OpenSpec: 18 passed. Documentation: zero issues.
- Viewer release: 6,349,904 bytes, +33,296 (0.53%). Five-launch medians:
  baseline 103.221 ms, candidate 102.776 ms; no performance change established.
- Native synthetic UI verified MPR/expanded layouts, %/cm³, source/estimate labels,
  metrics, curve visibility and independent ROI selection. No clinical commissioning
  or local contour-volume/DVH recalculation is claimed.
