# BEV review verification — 2026-10-21

Owner-approved BEV implementation and ownership split are implemented. Algorithms
and all image/aperture/leaf/contour GPU rendering live in Kepler; the egui host
owns the superficial workspace, source associations and controls. Existing viewer
RTPLAN parser/IEC model migration remains a separate scope.

- `cargo test --offline --locked --lib data::bev`: 4 passed.
- `cargo test --offline --locked --test bev_rendering_tests`: 3 passed with native
  Apple M5 Metal. Both signed encodings, whole-depth integration with a one-step
  budget, independent contrast/cache behavior, air/missing rays, rotated beam,
  target bounds/resource sharing and physical aperture pixels execute.
- `cargo check --offline --locked --lib --target wasm32-unknown-unknown`: pass;
  existing unrelated unused-import warnings remain. Browser runtime is unverified.
- Targeted rustfmt for all new BEV Rust files: pass. Global rustfmt and strict
  all-target Clippy remain failed in unrelated existing files. Diagnostic capture
  found no BEV-file warnings/errors; no unrelated fixes were applied.
- `npm run specs:check`: 19 items passed. `npm run docs:check`: zero issues.
- Host viewer required-GPU all-feature suite: 146 passed, 9 explicit fixture/local
  probes ignored; the synthetic native fixture generator was executed separately.
  Viewer format, strict Clippy, no-default types and release build pass.
- Native host reviewed imported synthetic CT + RT objects, shared CP stepping,
  dual MLC presentation, contour projection, independent contrast, BEV + MPR,
  Expand/Restore and explicit isocenter navigation. No Windows/browser runtime,
  large-study interaction/RSS, or clinical commissioning acceptance is inferred.

Performance: Apple M5 / Mac17,2 / 24 GiB, macOS 27.0 26A428. The size-optimized
host release grew 6,383,072 → 6,416,368 bytes (+33,296, 0.52%). Sequential five-run
warm-cache first-frame medians were 125.939 → 124.695 ms. These samples are not a
controlled speedup/regression result. The BEV UI was open in the after cohort.
An earlier concurrent-check after cohort (149.283 ms median) is retained as
confounded exploratory data. Detailed arrays/logs are in host target/bev-check.

The change stays active: whole-repository lint/format gates are not all green.
