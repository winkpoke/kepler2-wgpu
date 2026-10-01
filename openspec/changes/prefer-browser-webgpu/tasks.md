# Tasks

## 1. Device capabilities and validated uploads

- [x] 1.1 Add failing deterministic capability regressions covering R16Float preference, RG8 fallback, no usable format, missing upload/sample usage, absent filtering, and device-enabled feature legality; verify the failures precede implementation and the focused tests pass afterward.
- [x] 1.2 Implement a device-owned capability snapshot in rendering graphics initialization and use it in App startup, graphics replacement, and float toggles; verify focused tests and existing rendering/resource rebinding regressions preserve packed preference and CPU view state.
- [x] 1.3 Add test-first browser limit negotiation and volume-upload validation for supported limits below 1024, zero/oversized axes, boundary acceptance, checked byte-count overflow, and mismatched buffers; implement checks before allocation and verify focused tests plus existing volume encoding tests.
- [x] 1.4 Update current rendering and encoding guidance for selected-device format legality and upload errors; verify links with `npm run docs:check` and preserve the documented signed-storage/window-level contract.

## 2. Browser selection and bounded fallback

- [x] 2.1 Add test-first orchestration regressions via an internal attempt seam for WebGPU success, unavailable detection, adapter/device/capability failure followed by GL success, both attempts failing, forced GL, unknown selectors, and no retry after real-canvas binding; verify the tests fail before implementation and pass afterward.
- [x] 2.2 Implement WASM-only detection, disposable candidate surfaces, pre-canvas adapter/device/capability negotiation, and a single GL retry; verify WASM-library compilation and the orchestration tests while preserving the native surface-first path and existing native backend overrides.
- [x] 2.3 Explicitly enable both existing WASM wgpu features and implement the pre-start `kepler_wgpu_backend=gl` query override without logging the full URL; verify locked native/WASM builds, selector regressions, and an unchanged dependency-version/lockfile diff.
- [x] 2.4 Add actual-backend/capability/encoding startup summaries, stage-labeled negotiation errors and fallback reasons, and safe post-binding error diagnostics; verify error-path tests cover backend/stage context and do not retry on a bound canvas.
- [x] 2.5 Update rendering/build guidance and changelog for preference, fallback boundaries, forced GL, and manual browser verification; deliver a synthetic-volume browser procedure and record pending runtime checks explicitly; verify `npm run docs:check` and strict change validation.

## 3. Integrated verification

- [x] 3.1 Run `cargo +nightly check --locked --all-targets`, `cargo +nightly check --locked --lib --target wasm32-unknown-unknown`, active LSP diagnostics for changed Rust paths, formatting and strict Clippy; record exit statuses and distinguish scoped results from pre-existing global failures in verification.md.
- [x] 3.2 Run required-adapter native rendering correctness, volume encoding, resource-lifetime, and redraw regressions using the build matrix; record backend and outcomes in verification.md, without labeling missing adapters as successful tests.
- [x] 3.3 Run `npm run docs:check`, `npm run specs:check`, and `npm run openspec -- validate prefer-browser-webgpu --strict`; review the final diff for unchanged dependency versions, shader/uniform layouts, signed bytes, native selection, and unrelated work; record results in verification.md.

## 4. Manual browser acceptance

- [ ] 4.1 Build the browser package and verify synthetic signed-volume MPR/oblique MPR, MIP/MinIP/AvgIP, mesh, window/level, resize, input, redraw/idle, and restoration with WebGPU selected; record browser/version, backend, encoding, device limit, numerical fixture expectations, and observed errors in verification.md.
- [ ] 4.2 Repeat the same synthetic study and interactions with WebGPU disabled/unavailable, confirming automatic Gl selection; record independent fallback evidence and compare expected image values using the existing per-encoding tolerances.
- [ ] 4.3 Verify fresh-page forced GL on a WebGPU-capable browser, controllably failed pre-canvas WebGPU negotiation followed by GL, exhausted negotiation, and post-binding failure diagnostics without a same-canvas API switch; record the method, stage-labeled logs, outcomes, and cleanup of any test-only fault injection in verification.md.
