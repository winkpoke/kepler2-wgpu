# Viewer lifecycle and rebinding verification

## Scope and authorization

The user requested committing the completed layout refactor and proceeding with lifecycle dispatch and subsequent GPU rebinding recommendations. The preceding layout implementation is committed as `f42fab2`. This change preserves observable behavior and declares `skip_specs: true`; no baseline or pending behavior specs were rewritten or synced.

The private [lifecycle operation](../../../src/application/viewer_lifecycle.rs) owns the existing FrameOwner and explicit shutdown flag. [Winit dispatch](../../../src/application/render_app.rs) supplies borrowed primitive effects after the existing input/active-window gate. App still owns initialized graphics and the existing frame transaction. [AppView](../../../src/application/appview.rs) selects its compatible factory within complete bulk rebinding; App prepares replacement uploads before mutation. Public controls/bootstrap, standalone view methods, shaders, voxel encoding, and dependencies are unchanged.

## Deterministic and GPU evidence

[Lifecycle regressions](../../../src/application/viewer_lifecycle_tests.rs) execute the production event-handling interface with a recording adapter. Eight tests cover repeated resume, complete clean OS redraws, outstanding-request acknowledgement, update-before-render order, completion-time retry sampling, hidden/suspended/zero-sized intervals, animation/recovery deadlines, sticky fatal stop, explicit Quit versus Close behavior, post-shutdown silence, successful replacement ordering, stale-window rejection, retained suspension, and failed replacement preserving routing/outstanding work.

The existing [bulk AppView regression](../../../src/application/appview_tests.rs) now calls the complete rebinding operation without manually replacing the factory. It checks two devices and attachment formats, shared replacement MPR context, subsequent view creation, scale/pan/window state, real grayscale pixels, pending presentation, and compatible context/texture reuse. Existing standalone-view, mixed rendering, encoding, resource-lifetime, fault-notifier, frame-transaction, and redraw tests remain in the full suite. [Test result evidence](evidence/test-results.log) retains suite counts and the live application/lifecycle cases.

A read-only preservation review found no concrete defect and checked exact old/new redraw, shutdown, replacement, input-gate, wait-mode, and factory configuration semantics. Recording tests do not execute Winit callbacks or test pre-run control delivery; they supply pending-work state directly. These limits remain separate from runtime acceptance.

## Automated commands and outcomes

Environment: macOS arm64, Rust 1.98.0 (`rust-toolchain.toml`), Apple M5/Metal for GPU execution. The initial sandbox GPU run failed to find an adapter; approved outside-sandbox synthetic execution passed. No patient data or device acquisition was used.

| Command | Actual outcome |
| --- | --- |
| `rtk rustup show active-toolchain`; `rtk rustc -Vv` | Rust 1.98.0, aarch64-apple-darwin |
| `rtk cargo test --locked --lib application::viewer_lifecycle::tests` | Passed: 8 deterministic lifecycle tests |
| `rtk cargo test --locked --lib application::appview::tests` outside sandbox | Passed: 8 real GPU application tests |
| `rtk proxy env KEPLER_REQUIRE_GPU_TESTS=1 cargo test --locked` outside sandbox | Passed: 377 tests, 0 failed, 23 existing ignored, 28 result groups; 99 library tests |
| `rtk cargo check --locked --all-targets` | Passed; existing warnings remain |
| `rtk cargo check --locked --lib --target wasm32-unknown-unknown` | Passed; 7 existing warnings |
| `rtk wasm-pack build --target web --out-dir pkg --release -- --locked` | Passed; existing wasm-pack bindgen/platform/version notices remain |
| Scoped `rtk rustfmt --edition 2021 --check --config skip_children=true` on all changed Rust files | Passed |
| `rtk cargo fmt -- --check` | Failed: existing formatting in coordinate_safety_tests.rs, patient_safety_tests.rs, volume_integrity_tests.rs |
| `rtk cargo clippy --locked --all-targets -- -D warnings` | Failed: 56 existing errors and 1 warning; no suppressions or unrelated repairs |
| Supplemental `rtk proxy cargo clippy --locked --all-targets --message-format=json` | Failed on existing PatientPosition inherent to_string/Display conflict. Changed-path findings concern unchanged public argument count, borrowed Box, and legacy unwraps; no findings on the added lifecycle, rebinding operation, or new tests |
| `rtk npm run docs:check`; `rtk npm run specs:check` | Passed: 0 documentation issues; 13 spec/change items |
| `rtk python3.11 -m unittest discover -s scripts/tests -p 'test_*.py'` | Passed: 47 maintenance tests |
| `rtk npm run openspec -- validate deepen-viewer-lifecycle --strict --no-interactive`; `rtk git diff --check` | Passed |

## Native acceptance

Recompiled the existing synthetic [native harness](../deepen-view-layout-transitions/evidence/native-smoke-harness.rs) against the current library using `rtk rustc --edition=2021 openspec/changes/deepen-view-layout-transitions/evidence/native-smoke-harness.rs -L dependency=target/debug/deps --extern kepler_wgpu=target/debug/deps/libkepler_wgpu.rlib --extern pollster=target/debug/deps/libpollster-ce257682d1770172.rlib -o /private/tmp/kepler-lifecycle-native-smoke`.

Ran `rtk env RUST_LOG=info KEPLER_WGPU_BACKEND=metal KEPLER_WGPU_VALIDATION=true /private/tmp/kepler-lifecycle-native-smoke` outside the sandbox. The [actual native log](evidence/native-smoke.log) records Apple M5/Metal with validation enabled, R16Float selection and 2048 3D texture limit, missing-volume rejection, synthetic signed-volume load, modified settings/oblique MPR, MIP, generated mesh, mixed MIP/mesh, resize640x480, invalid-orientation rejection, final MPR, Quit, and normal return (exit0).

The three existing Winit pre-run handler diagnostics and two expected request errors remain visible. No fatal GPU/render error was observed. This is control/event-loop smoke evidence; quantitative surface pixels, actual suspend/occlusion callbacks, native window replacement, and comprehensive input/getter/bootstrap acceptance were not measured here.

## Browser acceptance and retained work

On 2026-10-04 the selected Chrome control connection repeatedly failed to load its request-header policy before a test page could open. No test server was started for this attempt. The earlier layout attempt was separately blocked by disabled extension file-URL access during synthetic fixture selection. No permissions were changed or alternate upload path used. Browser/version/backend/rendering/input/resize/replacement/restore outcomes remain unverified. Task3.3 and the previous layout task4.7 remain unchecked; neither change is archived.

The unrelated `upgrade-wgpu-winit/verification.md` edit remains excluded from commits. Its SHA-256 is unchanged: `dce193935455db88d3d63b0f6621e42213fa45d04eec7d3aed2bab208df82c11`. Passing scoped checks do not turn the existing full formatting/Clippy gates into passes.
