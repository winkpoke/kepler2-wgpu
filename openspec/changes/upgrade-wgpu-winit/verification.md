# Verification record

> Source-branch verification history retained by the v0.17.0 replay. It does not
> certify this candidate; see [candidate execution](../rebase-dev-graphics-onto-v017/execution.md).

Status: approved lifetime adaptation implemented; automated graphics/build verification passes. Dispatch coverage and manual native/browser acceptance remain incomplete; global format/strict-Clippy gates still fail with baseline findings. This file begins with the observed pre-upgrade baseline. Add implementation results below in dated sections; never replace failed/unavailable baseline entries with later passing results.

## Pre-upgrade baseline — 2026-10-01

### Environment and dependency resolution

- Toolchain selected by `rust-toolchain.toml`: `1.98.0-aarch64-apple-darwin`; `rustc 1.98.0 (88d9e12ae 2026-08-18)`, host `aarch64-apple-darwin`.
- `cargo tree --locked -i wgpu@23.0.1`: `kepler-wgpu -> wgpu 23.0.1`.
- `cargo tree --locked -i winit@0.29.15`: `kepler-wgpu -> winit 0.29.15`.
- `WGPU_BACKEND` and `KEPLER_WGPU_BACKEND` were unset. The required-adapter GPU tests passed, but they did not print/store adapter identity in this run; exact hardware/backend is **unverified**, not inferred from the host OS.
- Manifest declares wgpu 23 in common and WASM dependencies, with WASM `webgl` and `webgpu`; Winit 0.29.15 requests `rwh_05`.

### Commands and outcomes

| Command | Outcome | Evidence / limits |
| --- | --- | --- |
| `cargo check --locked --all-targets` | PASS (exit 0) | Completed on pinned toolchain; emitted existing unused/dead-code warnings in tests/fixtures and a future-incompatibility notice for `block 0.1.6`. |
| `cargo check --locked --lib --target wasm32-unknown-unknown` | PASS (exit 0) | Completed; emitted existing unused-import warnings in `app.rs`, `geometry.rs`, `fileio.rs`, `lib.rs`, and `dicom_repo.rs`. |
| `KEPLER_REQUIRE_GPU_TESTS=1 cargo test --locked --test rendering_correctness_tests --test volume_encoding_tests --test render_resource_lifetime_tests --test redraw_state_tests` | PASS (exit 0) | 33 tests passed: redraw 3, resource lifetime 8, rendering correctness 12, volume encoding 10. Required-adapter checks did not report adapter identity; see above. |
| `cargo fmt -- --check` | FAIL (exit 1) | Existing formatting diffs in `tests/coordinate_safety_tests.rs`, `tests/patient_safety_tests.rs`, and `tests/volume_integrity_tests.rs`; no formatter was run in write mode. |
| `cargo clippy --locked --all-targets -- -D warnings` | FAIL (exit 101) | Existing strict-Clippy findings across core timing/coordinates, acquisition, application, DICOM/medical metadata, rendering, and test targets (including dead code, argument-count, docs, and iterator lints). The baseline library failed with 39 errors and library-test compilation with 56; do not attribute all findings to this change. |

At the time of the baseline, no dependency or source implementation had been changed. OpenSpec planning files and this change's task tracking/verification record were the only changed paths.

## Implementation verification

### Initial contract regression and dependency resolution — 2026-10-01

- Added `src/core/error.rs` tests for an application-owned, actionable surface failure and ordinary rendering-error display. The first targeted run failed to compile because `SurfaceFailure` and `SurfaceFailureKind` did not exist (expected red result); after introducing those owned types and removing the `From<wgpu::SurfaceError>` conversion, `cargo test --locked --lib core::error::migration_tests` passed (2 tests).
- Updated common and WASM dependency declarations to wgpu 30.0.1 and Winit 0.30.13 with `rwh_06`. Restored the baseline lockfile before re-resolving offline; Cargo reported 97 package changes required by the selected dependency graph. The diff includes platform and transitive upgrades needed by wgpu/Winit, rather than a blanket `cargo update` of the whole graph.
- `cargo tree --locked -i wgpu@30.0.1` and `cargo tree --locked -i winit@0.30.13` show only the application; querying wgpu 23.0.1 finds no package. The WASM-targeted feature tree retains both `webgl` and `webgpu`; native default features retain Vulkan, GLES, Metal, and DX12 backends.
- Inspected downloaded wgpu 30.0.1 definitions for `CurrentSurfaceTexture`, `SurfaceStatus`, surface config, error-scope guards, and queue presentation. Exact semantics: acquired textures are presented with `Queue::present` after submission; statuses now include `Success`, `Suboptimal`, `Timeout`, `Occluded`, `Outdated`, `Lost`, and `Validation`; the device-loss notification remains a separate path.
- `cargo check --offline --all-targets` after dependency resolution currently **fails as expected during the unfinished migration**: 63 library and 76 library-test errors. Main categories include removed Winit `WindowBuilder` / old event-loop construction, wgpu surface/error and adapter/device API changes, descriptors/pipeline layouts, and removed texture-copy/polling APIs. These are migration work, not claimed passing checks.
- `cargo check --locked --lib --target wasm32-unknown-unknown` also currently fails during the same unfinished API migration (68 errors), including the corresponding shared graphics/pipeline issues. No post-upgrade GPU rendering tests have yet been run.

### Frame/descriptor work and compatibility blocker — 2026-10-01

- Added test-first frame transaction regressions in `src/application/frame.rs`, covering unit success, suboptimal reconfiguration retained across the public wrapper, skipped owned classifications, zero-size no-op, failed drawing, and recovery/validation failure. `cargo test --locked --lib application::frame::tests` initially exited 101 with missing frame seam types plus outstanding migration errors. Added the production driver/transaction and connected `App::render` to it; **green execution is still blocked**, so tasks 2.1–2.3 remain open.
- Added test-first owner scheduling/lifecycle regressions in `src/application/redraw.rs`. `cargo test --locked --lib application::redraw::tests` exited 101 with missing `FrameOwner` (expected red) and outstanding migration errors. The owner implementation has not been added; tasks 3.5/4.1 remain open.
- Adapted shared instance/device/adapter descriptors, SDR surface color configuration, optional pipeline slots, immediate size, attachment/multiview fields, texel-copy names, sampler mipmap type, and checked headless polling/scope popping. Retained instance/adapter for selected-backend surface recreation. These adaptations have not yet received GPU or WASM runtime verification; fatal callbacks and checked recovery handling are still incomplete.
- Added the scoped initial Winit window-construction bridge and contextual event-loop/window startup errors. The owned `ApplicationHandler`, browser active replacement creation, and frame-result consumer remain unfinished.
- Latest `cargo check --locked --lib`: **FAIL, exit 101, seven errors**. Four reference removed surface-error variants in the still-unmigrated event-loop owner. Three are E0597 at `PassContext::new` in mesh/slice/MIP pass execution.
- **New compatibility blocker:** public `PassContext<'a>` stores `&'a mut wgpu::RenderPass<'a>`, tying borrow and encoder lifetimes. wgpu 30.0.1 uses `PhantomDrop` to prevent shortening the encoder lifetime, making these existing calls invalid. The direct adaptation separates borrow and encoder lifetimes (`PassContext<'pass, 'encoder>`); explicitly annotated downstream callers would need another lifetime argument. This is an additional Rust API change beyond the approved rendering-error migration. Paused before changing this public type; request approval and update migration artifacts/guidance if accepted. No unsafe workaround or silent API expansion was applied.
- Active LSP probes returned no errors for the frame/error files after adding `From<SurfaceFailure>` (old unresolved-conversion findings were marked stale). A seven-path probe reported four unrelated informational Rust-2024 let-chain suggestions in `App`; **these LSP results do not override the Cargo compilation failures above**.
- No tasks completed this continuation: progress remains **3/32**. No post-upgrade rendering tests, packaging, global lint/format gate, native-window acceptance, or browser acceptance have passed.

### Approved lifetime, frame, GPU-fault, and lifecycle continuation — 2026-10-01

The user explicitly approved `PassContext<'pass, 'encoder>`. Proposal, design,
rendering delta, current [caller migration](../../../doc/rendering/wgpu30-migration.md),
and changelog record the additional Rust signature change. The implementation
preserves the encoder guard, without unsafe casts. Downstream-style `Renderable`,
unit-success `App::render`, and explicitly annotated pass callbacks compile.

Implemented the private frame transaction and owner policy, retained suboptimal
reconfiguration across the public wrapper, and connected native `run_app` / browser
`spawn_app` through `ApplicationHandler<UserEvent>`. Replacement windows use the
active loop and stale window events are filtered. Exactly one justified scoped
deprecation expectation remains in the GPU-ready initial window bridge. Initial
canvas attachment failure now returns contextual startup failure; replacement
attachment failure logs and stops that replacement request. Exported GLCanvas
controls and RenderApp startup signatures were not changed.

Persistent typed GPU faults register uncaptured/device-loss callbacks, wake an
idle owner through the existing quit transport, and stop recovery. Expected device
destruction is ignored. Native notification uses shared synchronization; browser
wakeups use a thread-local registry, not an unsafe Send wrapper. Old graphics detach
their wake transport when dropped. Initial/recovery/draw boundaries check faults.
Initial/headless validation scopes retain/pop guards; synchronous recovery uses
callback checks, including asynchronous browser delivery. General device recovery
and bound-canvas backend switching remain unsupported.

Test-first evidence:

- Previously red frame/error and owner tests now pass in the 86-test library suite.
- New fault tests initially failed to compile without `FaultNotifier`; notification,
  pending-fault delivery, typed kinds, and detached transports now pass.
- Mesh regression initially failed because an unattached renderer returned success;
  it now returns contextual graphics failure without clearing pending drawing.
- PassExecutor regression initially failed because injected pass context was erased;
  it now retains the first failing pass's message (`synthetic pass validation`) and
  submits/presents no partial frame. Relevant suite: `render_resource_lifetime_tests`.
- Active-window filter test initially failed to compile without the production
  filter; the green policy test retains suspension/resume eligibility when the
  active identity changes. It is **not** actual browser canvas replacement evidence.
- New-limit test initially failed without the compatibility policy. Newly added
  aggregate bind-group/vertex/buffer limits and the replacement varying-variable
  limit are capped at adapter support; all existing requested per-stage limits and
  feature requirements are retained. This avoids rejecting an otherwise usable
  adapter merely for unused wgpu 30 defaults. Synthetic native/browser tests pass.

### Final automated checks — 2026-10-01

Same pinned Rust 1.98.0/aarch64-apple-darwin as the baseline. Offscreen adapter:
`Apple M5`, `IntegratedGpu`, `Metal`, vendor/device 0, driver strings empty;
subgroups 4–64. No performance/clinical conclusion follows from these checks.

| Command/check | Result | Evidence / limits |
| --- | --- | --- |
| `cargo check --locked --all-targets` | PASS, exit 0 | Final native library/binary/test targets; existing warnings retained. |
| `cargo check --locked --lib --target wasm32-unknown-unknown` | PASS, exit 0 | Final library; native Tokio remains target-guarded and absent from WASM feature tree. |
| `cargo test --locked --lib` | PASS, exit 0 | 86 passed, no ignored tests; includes real offscreen application tests plus injected frame/owner/fault cases. |
| `cargo test --locked` | PASS, exit 0 | 364 passed, 23 ignored across 28 reported suites. Ignored fixtures/doctests remain ignored, not claimed verified; no live hardware connections or prep/exposure were run. |
| `KEPLER_REQUIRE_GPU_TESTS=1 cargo test --locked --test rendering_correctness_tests --test volume_encoding_tests --test render_resource_lifetime_tests --test redraw_state_tests -- --nocapture` | PASS, exit 0 | Redraw 4, resources 9, rendering correctness 12, encoding 9: **34 total**, no failures/skips. Apple M5/Metal identity printed. Original signed-pixel assertions, row padding, WGSL algorithms, and tolerances unchanged. |
| Active LSP probe of 28 changed Rust paths | No errors/warnings | 26 clean, two files with five informational Rust-2024 let-chain suggestions. No unsupported/unavailable/failed probes. These optional edition suggestions are not changes required by this upgrade. |
| `cargo fmt -- --check` | FAIL, exit 1 | Only original coordinate/patient/volume safety/integrity test formatting remains. Changed Rust files formatted separately without reformatting those unrelated baseline files. |
| `cargo clippy --locked --all-targets -- -D warnings` | FAIL, exit 101 | Same 39 library and 56 library-test errors as baseline. No full strict-lint pass claimed; compilation stops before some integration lint checks. |
| `wasm-pack build --target web --out-dir pkg --release -- --locked` | PASS, exit 0 | Final release package. Generated d.ts confirms `get_render_app(): Promise<RenderApp>`, `get_glcanvas(): GLCanvas`, `run(): Promise<void>`, div-targeted replacement and async getters used by existing pages. Generated pkg is ignored/not committed; packaging is not browser acceptance. |
| `npm run docs:check` | PASS, exit 0 | No link/identity issues, including new migration guide registered in current-doc scope. |
| `npm run docs:test` | FAIL, exit 1 | Default Python 3.9 lacks `tomllib`; 30 reported cases include one import failure. Retained as environment failure rather than concealed. |
| `python3.11 -m unittest discover -s scripts/tests -p 'test_*.py'` | PASS, exit 0 | BUILD.md's supported interpreter fallback: 47 tests. |
| `npm run specs:check` | PASS, exit 0 | 11 current specs/changes validated strictly. |
| `npm run openspec -- validate upgrade-wgpu-winit --strict --no-interactive` | PASS, exit 0 | Upgrade delta validation. |
| `git diff --check` and dependency/export/dispatch audit | PASS, exit 0 | One effective wgpu 30.0.1 and Winit 0.30.13; browser webgpu/webgl retained. No removed or added dispatched UserEvent variants, new GPU features, shader/pixel-tolerance changes, unsafe lifetime bypass, or generated artifacts. |

Temporary full logs: `/tmp/kepler-{native,wasm,unit,full,required-gpu,package,fmt,clippy}-final.log`;
prior red evidence includes `/tmp/kepler-{frame,owner,fault,window,limit}-red.log`
and `/tmp/kepler-pass-context-red.log`. Summary tables above retain outcomes when
these machine-local files disappear.

**Baseline count correction:** the initial table recorded 33 tests / encoding 10.
The retained pre-upgrade `/tmp/kepler-agent-gpu-tests.log` reports 3/8/12/**9**
(32 total), and baseline source has nine encoding tests. Current source retains
every baseline test; the added downstream-signature and mesh-error regressions
produce 4/9/12/9 (34). Preserve the original entry as recorded, but its numerical
count was a bookkeeping error, not a removed/disabled test.

One attempted focused command used nonexistent `render_pass_uniform_tests` and
exited 101 before executing tests. It was corrected to BUILD.md's
`rendering_correctness_tests` and `KEPLER_REQUIRE_GPU_TESTS` invocation above;
the failed invocation is not GPU evidence.

### Remaining acceptance and implementation coverage

Refreshed OpenSpec apply instructions confirm **23/32 tasks complete**, nine open.
This is implementation progress, not accepted completion or archive readiness.

Lifecycle/frame policy, layout getters/no-op redraw, and active-ID filtering have
focused coverage. Exhaustive production **control dispatch/pre-run order**, scalar,
coordinate, and rotation result-shape regressions still need an initialized owner
seam; isolated scheduler/getter tests and the unchanged-arm audit do not satisfy
that coverage requirement. Tasks 4.1, 4.4, and 4.5 remain open rather than narrowed
or claimed complete from source inspection. At this checkpoint, native-window and
browser checks 6.1–6.6 were unperformed. Subsequent shader-only browser evidence
below does not establish complete viewer/browser acceptance.

Run native checks only with synthetic volume injection, not `src/main.rs`'s
hard-coded patient-data paths. Browser procedure is documented in
[browser_gpu.md](../../../doc/testing/browser_gpu.md), including repeat replacement,
implemented getters, WebGPU, independent automatic fallback and forced GL, and
bounded failure injection. None is established by packaging/offscreen tests.

Rollback remains an atomic dependency/source rollback (no forced old lockfile with
new descriptors or mixed error contracts); browser rollback requires a fresh page
and rebuilt pkg, not reuse of a bound canvas. Keep this change and overlapping
`prefer-browser-webgpu` / `optimize-render-resource-lifetimes` acceptance open until
each has its own reviewed evidence. No archive or agreed acceptance exception.

### Browser WGSL uniformity regression and fix — 2026-10-01

User-supplied browser runtime log reports successful `BrowserWebGpu` startup,
R16Float selection, and four-view volume loading, followed by the exact rejection
`'textureSample' must only be called from uniform control flow` in MPR. The
pipeline/command-buffer failures cascade from the invalid shader. Fatal
notification/quit is observed, not a surface-acquisition problem. User browser
version and visual output remain unspecified; no patient data copied into fixtures.
This is failed runtime evidence, not completion of task 6.2.

Reproduced the same failure using the **actual production shader bodies plus the
shared decoder** on real Chrome WebGPU, without any volume/patient data:
`node /tmp/kepler-wgsl-repro/probe.mjs red` exited 1. Chrome
**154.0.8037.58** rejected both MPR and MIP with the uniform-control-flow message.
MIP additionally reports its helper/threshold/ray-loop call chain. A temporary
localhost-only CDP harness used isolated profiles and inspected
`GPUShaderModule.getCompilationInfo()` plus validation error scopes. An initial
`--dump-dom` attempt timed out/published PENDING; it was not a passed browser check.
A first follow-up saw cached old WGSL; no-store fetches and fresh profiles removed
that false result. The final footprint-based probe below fetched current sources.

Root cause: MPR sampled after a varying-coordinate bounds return; MIP sampled
inside varying-coordinate bounds and ray control. Native Naga 30's analyzer sets
`DISABLE_UNIFORMITY_REQ_FOR_FRAGMENT_STAGE = true`, so even all native validation
flags cannot reproduce the browser's derivative-uniformity rejection.

Rejected fixes, kept visible:

- Explicit fixed LOD zero compiles on WebGPU but is **not equivalent filtering**:
  the production sampler has linear mag / nearest min filters even with one mip.
  A new native minification regression failed with pixel 38 versus expected 51
  (existing two-byte tolerance), exposing the change. Fixed LOD zero was removed.
- `textureSampleGrad` with uniform ray-start gradients compiles on Chrome, but
  wgpu/Naga 30's Metal lowering incorrectly emits `metal::gradient2d` for float3
  gradients/texture3d, causing native production pipeline failure. No dependency
  fork, native-test suppression, or backend-specific shader divergence was added.

Final correction:

- MPR performs its original implicit sampling **before** the bounds return.
  Out-of-bounds pixels still return opaque black; sampler/filter selection is intact.
- MIP computes derivatives of `ray_origin + t_start * ray_dir` in uniform fragment
  control before ray misses and loops. Orthographic direction and step size are
  uniform, so logical steps add no spatial derivative. Derivatives are scaled by
  level-zero texture dimensions; `log2(max(max(length(dx), length(dy)), 1e-6))`
  supplies explicit, footprint-based LOD to ray sampling. This avoids both divergent
  implicit derivatives and the broken Metal 3D-gradient translation, preserving
  nearest minification / linear magnification for the existing single-mip volume.
- Signed decode, texture formats/sampler descriptors, uniform layout, voxel bytes,
  row padding, window/level, ray stepping/aggregation/thresholds, boundary return
  values, Rust/JavaScript APIs, and existing pixel tolerances were not changed.

Persistent regressions (no new dependencies):

- `production_volume_shaders_preserve_derivatives_outside_divergent_control_flow`
  parses real production WGSL via wgpu's Naga re-export, validates it, and guards
  sampling/derivative placement before divergent control plus footprint-based MIP
  LOD. The initial regression was red on the original implicit sampling; its final
  placement/filter variant was also observed red against the rejected fixed-LOD fix.
- `volume_minification_retains_nearest_filter_in_mpr_and_projection_modes` pins
  native 8-voxel-to-4-pixel minification in both encodings and MPR/MIP/MinIP/AvgIP;
  it was red for the fixed-LOD attempt and green for the final correction.
- `mpr_mixed_bounds_keep_black_background_and_filtered_in_volume_pixels` covers
  fragment quads mixing inside/outside voxels, both encodings, and exact opaque
  black outside. Existing linear magnification/interpolation and signed mode
  assertions continue passing without widened bounds.

Final verification after the footprint correction:

| Command/check | Outcome | Limits |
| --- | --- | --- |
| `node /tmp/kepler-wgsl-repro/probe.mjs footprint-green` | PASS, exit 0 | Chrome 154.0.8037.58, both actual production shader modules, zero compilation errors and null validation errors. Shader compilation only: not full RenderApp/page/pixel/lifecycle acceptance. |
| `node /tmp/kepler-wgsl-repro/probe.mjs footprint-pixels` | PASS, exit 0 | Fresh Chrome WebGPU profile; additionally creates the production MIP pipeline and renders/maps synthetic packed signed 8×1×1 voxels into 4×4 RGBA8. MIP/MinIP/AvgIP each produce grayscale `[51,102,153,204]`, exactly the independent nearest-minification expectation (≤2-byte bound); null validation scopes. Does not exercise the Rust owner/page or establish all-browser/full-volume parity. |
| `cargo test --locked --test volume_encoding_tests production_volume_shaders_preserve_derivatives -- --nocapture` | PASS, exit 0 | One focused production-structure regression. |
| `KEPLER_REQUIRE_GPU_TESTS=1 cargo test --locked --test rendering_correctness_tests --test volume_encoding_tests --test render_resource_lifetime_tests --test redraw_state_tests` | PASS, exit 0 | **37** passed: redraw 4, resources 9, rendering 14, encoding 10. Apple M5/Metal. |
| `cargo test --locked` | PASS, exit 0 | **367** passed, 23 ignored; no live hardware testing. |
| `cargo check --locked --all-targets` | PASS, exit 0 | Final native targets. |
| `cargo check --locked --lib --target wasm32-unknown-unknown` | PASS, exit 0 | Final WASM library. |
| `wasm-pack build --target web --out-dir pkg --release -- --locked` | PASS, exit 0 | Final corrected package rebuilt; generated output remains ignored. Requires a full browser reload after the prior viewer quit. |
| `cargo fmt -- --check` | FAIL, exit 1 | Original coordinate/patient/volume safety/integrity formatting only; changed tests formatted. |
| `cargo clippy --locked --all-targets -- -D warnings` | FAIL, exit 101 | Same baseline 39 library / 56 library-test findings. No global quality-gate pass claimed. |
| Active LSP probes of two changed Rust tests and four updated Markdown paths | No errors/warnings | All six checks confirmed; only 18 auxiliary spelling suggestions misinterpret the graphics acronym LOD (level of detail) as LOAD. |
| `npm run docs:check` | PASS, exit 0 | No current-doc/delta link issues. |
| `python3.11 -m unittest discover -s scripts/tests -p 'test_*.py'` | PASS, exit 0 | 47 supported-interpreter maintenance regressions. |
| `npm run specs:check` and strict upgrade validation | PASS, exit 0 each | Delta/current spec syntax only; no manual task marked complete. |
| `git diff --check` | PASS, exit 0 | Final localized shader/test/docs edits; no added dependency or public API change. |

Machine-local logs: `/tmp/kepler-wgsl-browser-{red,footprint-green,footprint-pixels}.json`,
`/tmp/kepler-wgsl-{regression-red,uniform-flow-red,minification-red}.log`,
`/tmp/kepler-wgsl-{regression,gpu,native,wasm,full,package,fmt,clippy}-final.log`.
The temporary harness is explicitly a throwaway diagnostic, not a new standing
browser test runner. Chrome/server processes were terminated after each probe.
Current encoding/migration guidance and changelog explain the filtering constraint.
OpenSpec task progress remains **23/32**: full viewer re-test, other browsers/GL,
control dispatch coverage, and manual acceptance remain open.

### Review correction: native display ownership — 2026-10-02

The native instance descriptor now retains `Arc<Window>` as its display-handle
provider. wgpu 30's GLES backend selects its window-system display during instance
creation; omitting it selected a surfaceless EGL display on Mesa/Wayland. The
retained window supplies the same display used by initial and recovered surfaces.
The field is native-only; browser and headless initialization are unchanged.
Caller guidance and the changelog record the correction.

| Command/check | Outcome | Limits |
| --- | --- | --- |
| `cargo check --locked --all-targets` | PASS, exit 0 | Native targets; existing warnings remain. |
| `cargo check --locked --lib --target wasm32-unknown-unknown` | PASS, exit 0 | WASM library; existing warnings remain. |
| `cargo test --locked --lib` | PASS, exit 0 | 86 passed, desktop GPU access. |
| `cargo test --locked --test rendering_correctness_tests --test volume_encoding_tests --test render_resource_lifetime_tests --test redraw_state_tests` | PASS, exit 0 | 37 passed across four offscreen/policy suites; desktop GPU access. |
| `npm run docs:check` / `npm run specs:check` | PASS, exit 0 | Zero documentation issues; 11 specs/changes validated. |
| `python3.11 -m unittest discover -s scripts/tests -p 'test_*.py'` | PASS, exit 0 | 47 maintenance-script tests. |
| `rustfmt --edition 2021 --check src/rendering/core/graphics.rs` / `git diff --check` | PASS, exit 0 | Changed Rust file and diff whitespace. |
| `cargo fmt -- --check` | FAIL, exit 1 | Existing coordinate/patient/volume test formatting only. |
| `cargo clippy --locked --all-targets -- -D warnings` | FAIL, exit 101 | Existing repository lint failures remain; no full lint pass claimed. |

An initial scoped formatting invocation used edition 2024 and reported unrelated
edition-style changes; rerunning with the repository's edition 2021 passed without
changing those lines. Native Linux/Wayland presentation was not exercised on this
macOS host. Existing offscreen tests and compilation do not establish GLES window
startup; verify `KEPLER_WGPU_BACKEND=gl` on Wayland as part of native acceptance.
No manual acceptance task was marked complete.
