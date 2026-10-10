# Verification — prefer-browser-webgpu

**Status:** Implementation, native/WASM-library verification, and the packaged
browser build are recorded; browser runtime acceptance remains pending. This
record separates compilation and native tests from browser execution.

## Automated results recorded so far

| Command | Result | Notes |
| --- | --- | --- |
| `cargo +nightly test --locked --lib rendering::core::browser_backend::tests` | PASS — 8 passed | Covers automatic selection, single GL fallback, forced GL, unknown selectors, exhausted candidates, and no retry after binding. |
| `cargo +nightly test --locked --lib rendering::core::capabilities::tests` | PASS — 8 passed | Format legality/preference, enabled features, 3D limits, and rejection coverage. |
| `cargo +nightly test --locked --lib rendering::view::render_content::tests` | PASS — 5 passed | Upload dimensions, limit boundary, checked byte counts, and buffer-length rejection. |
| `cargo +nightly check --locked --all-targets` | PASS — exit 0 | Native library, binary, and test targets compile. Emits pre-existing warnings in test fixtures and unrelated tests. |
| `cargo +nightly check --locked --lib --target wasm32-unknown-unknown` | PASS — exit 0 | Run after the latest post-binding diagnostic edit. Emits unused-import warnings in existing WASM paths. |
| `lens_diagnostics` with `source=lsp`, changed Rust paths, full scan, all severities | PASS — 0 diagnostics | Six changed paths initially; five rechecked after the upload-test cleanup. No unavailable or inconclusive paths. |
| `rustfmt --edition 2021 --check src/application/app.rs src/rendering/core/browser_backend.rs src/rendering/core/capabilities.rs src/rendering/core/graphics.rs src/rendering/view/render_content.rs` | PASS | Changed Rust implementation files are formatted. |
| `cargo +nightly fmt -- --check` | FAIL — exit 1 | Repository-wide formatting differences occur in unmodified files including `tests/coordinate_safety_tests.rs` and `tests/patient_safety_tests.rs`; changed implementation files pass the targeted check above. No unrelated files were reformatted. |
| `cargo +nightly clippy --locked --all-targets -- -D warnings` | FAIL — exit 101 | Strict Clippy reports 35 library and 52 lib-test errors across existing unrelated code (e.g. formatting/doc lints, argument counts, DICOM, coordinate, and view modules). It also flags pre-existing argument counts on unchanged `App::set_render_mode` and `RenderContent::from_bytes` signatures. A new test-only identity-operation lint was fixed and is absent on the rerun. |
| `npm run docs:check` | PASS — 0 issues | Includes browser procedure links. |
| `npm run specs:check` | PASS — 9 passed, 0 failed | All strict spec/change identities validated. |
| `npm run openspec -- validate prefer-browser-webgpu --strict` | PASS | Change validates strictly. |

| `KEPLER_REQUIRE_GPU_TESTS=1 cargo +nightly test --locked --test rendering_correctness_tests --test volume_encoding_tests --test render_resource_lifetime_tests --test redraw_state_tests` | PASS — 32 passed | Required-adapter run on the host's primary backend; no adapter tests were skipped. |
| `WGPU_BACKEND=metal KEPLER_REQUIRE_GPU_TESTS=1 cargo +nightly test --locked --test rendering_correctness_tests --test volume_encoding_tests --test render_resource_lifetime_tests --test redraw_state_tests` | PASS — 32 passed | Backend forced to Metal by the test fixture; adapter model is not printed by passing test output. |

| `npm run docs:check` | PASS — 0 issues | Rechecked after all current documentation edits. |
| `npm run specs:check` | PASS — 9 passed, 0 failed | Strict validation of all specs and changes. |
| `npm run openspec -- validate prefer-browser-webgpu --strict` | PASS | Rechecked after task updates. |
| `git diff --check` | PASS | No whitespace errors. |
| `cargo tree --locked -i wgpu` / `cargo tree --locked -i winit` | PASS — `wgpu 23.0.1`, `winit 0.29.15` | `Cargo.toml` only enables WASM `webgpu` alongside `webgl` and adds required `web-sys` canvas/query types; `Cargo.lock` is unchanged. |
| Final diff review | PASS | No shader/uniform-layout or `src/data/volume_encoding.rs` changes; signed-byte behavior remains covered by passing encoding tests. Native backend selection and unrelated working files are unchanged. |
| `cargo +nightly build --locked --lib --target wasm32-unknown-unknown --release` | PASS — exit 0 | Builds the optimized WASM library; emits existing WASM unused-import warnings. This does not generate the browser JS package. |
| `wasm-pack --version` | PASS — `wasm-pack 0.15.0` | Installed with `cargo install wasm-pack --locked` after the initial package build was blocked. |
| `RUSTUP_TOOLCHAIN=nightly wasm-pack build --target web --out-dir pkg --release -- --locked` | PASS | Browser package generated in `pkg/`; emits existing WASM unused-import and future-incompatibility warnings. |

The required native GPU correctness/resource/redraw suites and source/delta review
are complete. Packaged browser build and browser runtime checks remain pending.

## Browser runtime acceptance — pending

No browser runtime claim is made. The package is built, but no browser runtime was
exercised. The following cases remain open and must be recorded independently
with browser/version, selected backend, format/encoding, maximum 3D limit, method,
observations, and errors:

- [ ] Synthetic signed volume with WebGPU selected: MPR/oblique MPR, MIP/MinIP/AvgIP, mesh, window/level, resize, input, idle/redraw, and restoration.
- [ ] Same synthetic volume with automatic WebGL2 fallback when WebGPU is unavailable.
- [ ] Fresh-page `?kepler_wgpu_backend=gl` on a WebGPU-capable browser.
- [ ] Controlled pre-canvas WebGPU negotiation failure followed by exactly one GL attempt.
- [ ] Exhausted negotiation and post-binding diagnostics, with no same-canvas API switch.
- [ ] Quantitative browser framebuffer parity; the current demo has no GPU readback assertion, so visual inspection alone is insufficient evidence.

See [the reproducible browser procedure](../../../doc/testing/browser_gpu.md).
