# Plan beam arrangement verification — 2026-10-24

The owner authorized plan-wide beam presentation and explicitly required rendering
to live in Kepler. `data::beam` and `rendering::beam` now own bounded intersection,
union and MPR batch rendering. Existing BEV rendering remains reused. The viewer
owns import/association, delivery semantics and egui controls/callbacks.

- Native Metal `cargo test --locked --offline --test beam_arrangement_tests --test
  bev_rendering_tests`: 5 passed. Distinct beam/pane buffers, stale batch clearing,
  disconnected union, invalid input, signed CT projection and cache behavior execute.
- Viewer all-feature suite with `KEPLER_VIEWER_REQUIRE_GPU=1`: 147 passed, 10 ignored;
  opt-in synthetic generator executed separately. Existing native/oblique/clipped
  divergent aperture and dual-layer pixel tests stay green; the new seven-beam
  pixel regression preserves every direction under changed selection.
- Viewer formatting, strict all-target/all-feature Clippy, no-default types and
  size-optimized release build pass. No dependency/lockfile change; only WGPU 30.0.1.
- WASM library type check passes with eight existing unrelated unused-import warnings.
  Browser runtime was not tested. Changed Kepler Rust files pass targeted rustfmt.
- Specification validation: 20 passed. Documentation validation: zero issues.
- Global strict Kepler Clippy remains failed in unrelated existing files. JSON
  diagnostics captured by the host contain no warnings/errors rooted in the changed
  beam/overlay source files. Full diagnostics are retained in the host's
  target/plan-arrangement-check/kepler-clippy.jsonl. No unrelated lint cleanup was made.
- Native synthetic host review verified seven visible beams, Beam 3 selection,
  Segment 8 / CP 14–15, raw CP access, isolate/hidden selection, Expand/Restore,
  separate dashed envelope and an arc with a fixed 0–90 CW guide while BEV showed
  CP 1 / gantry 45. Screenshots remain in the host's target/plan-arrangement-check/.

Performance and complete host validation are recorded in the viewer's
docs/performance.md and docs/plan-review-verification.md, with exact binary hashes,
alternating startup arrays and all outliers retained. Startup is still synthetic
MPR and excludes first-use BEV latency. Buffer/target bounds are not measured peak RSS.

The change remains active because whole-repository lint gates are not green.
No source was copied from Kepler into the host. No clinical commissioning, machine
delivery, dose/fluence calculation, collision model, 3D source path, Windows/Linux
or browser runtime acceptance is established by these synthetic/native checks.
