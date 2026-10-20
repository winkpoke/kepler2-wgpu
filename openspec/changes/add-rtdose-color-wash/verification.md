# Verification — 2026-10-20

## Passing scoped evidence
- `cargo test --offline --locked --lib dose_`: five model/parser/shader tests.
  Synthetic serialized unsigned 16/32-bit grids cover LE, implicit LE and BE,
  scaling, original u32 values, relative and legacy absolute offsets, descending
  frame order, nonuniform Z and rejection of malformed/unsupported metadata.
- Native Metal viewer regression suite: 110 passed, one fixture generator ignored.
  Independent physical ramp pixels cover native planes and clipping, simultaneous
  nonzero-angle oblique panes, visibility/cutoff and GPU lifetime/budget rejection.
  Point/worker tests cover metadata-only import, frame mismatch, stale results,
  source replacement and out-of-domain distinction.
- Final native release UI shows synthetic CT plus dose in three panes, PLAN/Gy,
  maximum 21.5 Gy, legend and crosshair 15.75 Gy. Visibility and opacity controls
  work. GPU oblique mode retains dose. Manual nonzero angle entry was not
  established; native GPU pixel tests exercise nonzero angles.
- WASM library compiles with eight pre-existing unused-import warnings. Viewer
  formatting, strict Clippy, release/no-default builds pass; no new dependencies.
- Documentation and strict OpenSpec checks pass.
- Viewer size: 6,183,120 → 6,233,328 bytes (+50,208, 0.81%). Five alternating
  first-UI medians: 142.498 → 131.775 ms, with baseline outlier retained. This
  interactive-session measurement does not establish a speedup. Raw samples and
  hashes are under viewer `target/rtdose-check/performance.json`.

## Whole-library baseline limitations
Broader library tests: 120 passed, nine failures in untouched existing
window/level, export UID, camera/mesh rotation/default and MIP/MPR layout tests,
three ignored. Whole-library Clippy retains its existing inherent-to-string
error and other unrelated diagnostics. These are not passing gates. New dose
module diagnostics are checked separately.

## Unperformed acceptance
Browser/Windows/Linux runtime, large dose import/frame time, peak RSS, idle CPU,
vendor interoperability and clinical qualification remain unverified.
