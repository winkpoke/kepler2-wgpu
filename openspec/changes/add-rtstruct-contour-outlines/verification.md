# Verification - 2026-10-19

## Feature evidence

- Actual Kepler serialized RTSTRUCT suite: six tests pass, including UN coordinate
  decoding and the 32 MiB predecode limit. Contour geometry, shader and uniform
  layout filters: five tests pass (one also matches an existing ball-contour test).
- Viewer suite with `KEPLER_VIEWER_REQUIRE_GPU=1`: 150 passed, one existing live-Pi
  test intentionally ignored. Metal contour pixels exercise physical plane
  matching, clipping, empty nonmatching slices, ROI visibility and independent
  per-pane uniforms. Reindexed native plane mapping and stale generation/source
  replacement are covered separately.
- Viewer strict all-target/all-feature Clippy passes; targeted new Kepler modules
  have no strict-Clippy diagnostics. Viewer release and no-default-feature builds
  pass. New files and viewer formatting pass.
- Kepler WASM library check passes with eight existing unused-import warnings.
  This is not browser runtime acceptance.
- Native macOS UI exercised with a generated 64x64x9 CT, a green XOR ring and an
  overlapping red ROI: normal Open routing, Structures path entry, reference
  association status, colored outlines, independent visibility, ROI search,
  source-plane Focus, scrolling away/returning, zoom, and coexistence with Copilot.
  Pan drag was attempted but not established by the automation; clipping/pan
  projection is covered by pixel/geometry regressions, not a claimed manual pass.
- No new dependencies or lockfile changes. `cargo tree -d` and `cargo tree -i wgpu`
  retain one WGPU generation (30.0.1).
- `npm run docs:check` and `npm run specs:check`: pass, including this active change.

## Outstanding repository gates

`cargo test --locked --offline --lib` has nine failures in untouched existing
window/level clamping, DICOM export UID expectations, camera/mesh defaults and
rotation, MIP uniform-size and MPR uniform-size tests. The final result is 115 passed, nine failed, three ignored.
The new importer/geometry/shader tests pass. These existing expectations were not
changed to make the gate appear green.

Strict Kepler all-target Clippy fails in existing application/core/rendering/
server/acquisition code; machine-readable diagnostics show none in the new
modules. Whole-repository formatting also reports existing differences, starting
in src/acquisition/process.rs. No unrelated formatting/lint cleanup was made.

## Performance and platform limits

The host executable grew from 6,766,352 to 6,782,928 bytes (+16,576 bytes, 0.24%).
Five alternating first-UI samples had medians 151.183 ms before and 145.539 ms
after, retaining both outliers; no startup speedup is claimed. Raw samples and
SHA-256 hashes are in the host target/rtstruct-check/performance.json.

Final host release-size/startup results are recorded in the viewer's
`docs/performance.md` and `docs/rtstruct-verification.md`. No claim is made about
large contour interaction throughput, peak RSS, idle CPU measurements, browser
runtime, Windows/Linux runtime, reconstruction accuracy, or clinical acceptance.
The change remains active; baseline sync/archive is not being asserted.
