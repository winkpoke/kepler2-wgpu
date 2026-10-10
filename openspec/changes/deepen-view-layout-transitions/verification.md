# View-layout transition verification

## Scope and environment

Implementation uses the private production operation in [view_layout_transition.rs](../../../src/application/view_layout_transition.rs), called by existing `App::set_render_mode`. The headless fixture borrows real `AppModel`/`AppView` state and injects returned preparation failures into that same sequence. Public controls, shaders, voxel encoding, and mesh extraction are unchanged.

Local environment: macOS arm64, Rust 1.98.0 from `rust-toolchain.toml`, Apple M5/Metal for headless GPU execution. The sandbox reports no Metal adapter; synthetic GPU checks require the approved outside-sandbox execution. No patient data or live acquisition was used.

## Regression development

- Extracted the live sequence with its early layout mutations and added `returned_failures_preserve_all_committed_state_and_pixels` before removing those mutations.
- The first GPU run reproduced upload failure clearing the previous grid into an empty one-cell layout. After removing that early mutation, the constructor-failure run reproduced clearing the old views before the replacement mesh view failed after five successful candidate constructions.
- Moving layout replacement after all preparation made the preservation regression pass for clean and pending states. It checks old view identities, settings/rotation/positions, volume identity/data, orientation selections, cache identity/content, mesh flag, presentation state, and old pixels. Weak references also establish that abandoned candidate content is released.
- The original two-sample fixture hit an existing Gaussian mesh-filter indexing panic in `mesh_processing.rs`. Generation fixtures now use an eight-sample extent. Mesh-filter repair and panic/device-loss rollback remain outside the approved change; no extraction behavior was changed to make the regression pass.

## Automated checks

| Check | Outcome |
| --- | --- |
| `rtk rustup show active-toolchain`; `rtk rustc -Vv` | Rust 1.98.0, aarch64-apple-darwin |
| `rtk cargo check --locked --all-targets` | Passed; existing warnings remain |
| `rtk cargo check --locked --lib --target wasm32-unknown-unknown` | Passed; existing WASM warnings remain |
| `rtk proxy cargo test --locked --lib application::appview::tests -- --nocapture` | Passed: 8 application tests, including 5 production transition regressions, Apple M5/Metal |
| `rtk proxy env KEPLER_REQUIRE_GPU_TESTS=1 cargo test --locked` | Passed: 369 tests across 28 result groups, 23 existing ignored tests; includes 91 library tests and all required rendering/encoding/resource/redraw suites |
| `rtk cargo fmt -- --check` | Failed: existing formatting in coordinate_safety_tests.rs, patient_safety_tests.rs, volume_integrity_tests.rs |
| Scoped `rtk rustfmt --edition 2021 --check --config skip_children=true` on all changed Rust files | Passed |
| `rtk cargo clippy --locked --all-targets -- -D warnings` | Failed: 56 existing denied findings and 1 warning, including unchanged public argument counts and existing core/acquisition/rendering/data code; no suppressions added |
| `rtk wasm-pack build --target web --out-dir pkg --release -- --locked` | Passed; existing unused imports and wasm-pack install/version notices remain |
| `rtk npm run docs:check`; `rtk npm run specs:check` | Passed: 0 documentation issues; 12 spec/change items |
| `rtk python3.11 -m unittest discover -s scripts/tests -p 'test_*.py'` | Passed: 47 maintenance tests |
| `rtk npm run openspec -- validate deepen-view-layout-transitions --strict --no-interactive`; `rtk git diff --check` | Passed; unrelated verification file SHA-256 unchanged |

## Contract-to-evidence map

| Contract | Production evidence |
| --- | --- |
| Success modes, existing defaults, orientation selections, pixel parent dimensions, presentation | `successful_modes_commit_defaults_orientations_cache_and_pending_frame` |
| Upload or partial construction error leaves committed state intact; candidate release | `returned_failures_preserve_all_committed_state_and_pixels` |
| Missing volume/mesh, invalid used selection, real malformed upload | `malformed_upload_and_invalid_prerequisites_leave_old_layout_intact` |
| Legacy modes, mesh-over-MIP precedence, custom strategy, reuse/regeneration and fallback | `mixed_precedence_custom_strategy_and_mesh_generation_retain_legacy_policy` |
| Failed switches from MIP/mesh preserve settings and clean/pending work | `failures_from_single_mip_or_mesh_preserve_settings_and_pending_work` |
| GPU rebinding retains existing CPU state/shared resources | Existing AppView and rendering correctness regressions |

Tests are in [appview_tests.rs](../../../src/application/appview_tests.rs). Detached mode/error-label simulations were removed after production checks covered their intended transition behavior. Remaining public ViewManager checks retain their separate limitations.

## Native smoke evidence

The [temporary harness](evidence/native-smoke-harness.rs) was compiled against the built production library with `rtk rustc --edition=2021 /private/tmp/kepler-native-transition-smoke.rs -L dependency=target/debug/deps --extern kepler_wgpu=target/debug/deps/libkepler_wgpu.rlib --extern pollster=target/debug/deps/libpollster-ce257682d1770172.rlib -o /private/tmp/kepler-native-transition-smoke`, then run using `rtk env RUST_LOG=info KEPLER_WGPU_BACKEND=metal /private/tmp/kepler-native-transition-smoke`. Its [native log](evidence/native-smoke.log) records all requested controls, the two expected rejected requests, resize to 640x480, and normal return. This source is verification evidence, not a new application entry point or dependency.

The [constructor regression before the fix](evidence/constructor-failure-before-fix.log) and [final eight application tests](evidence/application-tests.log) retain the failure/pass evidence. The eight-sample generation fixture avoids the existing small-volume filter panic without modifying that filter.

## Manual acceptance and limits

A temporary native harness outside the repository uses the compiled production library and existing GLCanvas controls with the same 16x16x8 signed synthetic volume as the browser procedure. The first native Metal run completed missing-volume rejection, altered MPR settings/oblique orientation, MIP, generated mesh, mixed MIP/mesh, resize, invalid-orientation rejection, final MPR, and quit with normal return. It emitted existing Winit pre-run handler diagnostics plus the two expected validation errors. A second run through a temporary app bundle enabled GPU validation; a visible four-MPR layout with the synthetic signed gradient was inspected. Per-mode quantitative native surface pixels and broader lifecycle acceptance were not measured by this smoke check.

Browser package serving works at localhost:8001; localhost:8000 was already occupied and was preserved. Chrome opened the rebuilt demo, but synthetic file selection was blocked because the ChatGPT extension's "Allow access to file URLs" permission is disabled. That setting was not changed and no alternative upload path was used. Browser rendering, controls, backend selection/fallback, and input/resize/restoration remain unverified; task 4.7 stays open. WASM packaging and native pixels do not establish those browser outcomes.

Old and candidate GPU resources coexist temporarily. One bounded candidate is discarded on a returned error or committed on success. Content weak-reference checks and existing resource suites establish ownership behavior, not driver-memory leak freedom or peak-memory measurements. Existing mesh-cache identity/invalidation and asynchronous fatal GPU handling retain their earlier policies. Concurrent mutation, clinical validity, and performance improvement are not established by these tests.

Unrelated edits to `upgrade-wgpu-winit/verification.md` are preserved; initial SHA-256: `dce193935455db88d3d63b0f6621e42213fa45d04eec7d3aed2bab208df82c11`.

Supplemental `rtk proxy cargo clippy --locked --all-targets --message-format=json` also fails on the existing `PatientPosition::to_string`/`Display` conflict in `image_info.rs:239`. Diagnostics on changed files identify only the unchanged public 16-argument control and existing borrowed-box layout interface; no diagnostics identify the new transition module or its test additions. This does not turn the failed full lint gate into a pass.
