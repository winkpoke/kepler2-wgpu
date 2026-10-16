# Candidate execution evidence — 2026-10-05

All 13 source commits have been replayed onto v0.17.0 with merge topology
preserved. Implementation and local verification are complete on
`codex/rebase-v017-wgpu`, in `/private/tmp/kepler2-wgpu-v017-rebase`.
The destination `wgpu-upgrade` has been fast-forwarded to the verified candidate
with the two named platform/manual checks explicitly deferred by the user.
[Acceptance and landing](acceptance.md) records the decision. No push was performed.

The user clarified that downstream software relies only on Kepler's viewer and
browser. [Public decisions](public-exceptions.md) preserve that interface,
record the explicit P2/P3 approvals, and retain P4 constructors/helpers.
Historical approval or verification text does not authorize new exceptions.

## Isolation and replay

- Source: `8e308cb3c2e047f530ddbfde690fa45d8e30d1fc`.
- Excluded source base: `603bb96bb774f8a61652e9d14985a2b5046b3e75`.
- Destination/v0.17.0: `a39ca528a55be7f7fe565a7fa7ed20b2251826e4`.
- Completed replay: `dda4ff0dc8f2c89b4064e632e1f9b79672025da6`.
- Backups: `codex/backup-dev-graphics-20261005` and
  `codex/backup-v017-wgpu-20261005`.

[Replay ledger](replay-ledger.json) maps all 13 commits and resolution decisions.
The recreated merge `d625e176a75cc78ccbab7cff7d9cf6946048995c` retains browser
parent `82a258a605e42ac354f5141ccb9523f33f46b672` and toolchain parent
`e8927142c85cf5891699013d691b723afb63d178`. Separate review of the
[combined merge diff](candidate-evidence/recreated-merge-review.log) confirms both
histories, preflight/capability code, portable timing and combined documentation.
[Range-diff](candidate-evidence/range-diff.log) records the ordinary adaptations;
it does not substitute for the merge review.

Release DVR, segmentation, DR overlays, mesh/spine/needle/ball state, exports,
HTTP routes, CLI and local WASM behavior remain. Obsolete source renderers and
opaque assets were not restored. Imported verification is historical and task
lists were not blanket-completed. Cargo 0.17.0 and npm 0.16.0 remain.
Existing `.pendant` worktrees and unrelated untracked Python work remain intact.

Supplemental validation follows the mapped replay: offline HTTP/WebSocket,
independent compute-owner regressions, exact production uniform layouts, current
documentation and evidence. Browser testing also found a reversed private GPU
log-level filter. It now retains warnings/errors and suppresses verbose wgpu/Naga
logging; the regression and final browser rebuild pass.

## Public compatibility

[Baseline manifest](baseline/manifest.json) hashes 101 release source files,
including 87 Rust files, 1,089 inventoried declarations and 163 export/module
edges. Native/WASM evaluated rustdoc JSON captures cfg and generated macros.
Fresh release browser declarations came from the unmodified tag; the historical
snapshot has separate provenance.

The deeper [native](candidate-evidence/api-native-diff.json) and
[WASM](candidate-evidence/api-wasm-diff.json) audits walk re-exports, inherent
methods, fields and trait members, counting aliases separately. Native has
6,117 baseline / 6,361 candidate paths; WASM has 5,809 / 6,055. Both have zero
removals and 32 changed declarations, all classified as P2 errors, P3 context/
callback lifetimes, or P1's upstream Winit PhysicalSize identity. No GLCanvas,
RenderApp, domain or server declarations changed. Raw snapshots and the audit
script are retained in [candidate-evidence](candidate-evidence/README.md).

The audit normalizes rustdoc IDs/upstream internal module moves. It does not
independently compare trait implementations or establish cross-version GPU
object compatibility. External consumer fixtures and explicit conversions cover
the relevant implementations. Low-level graphics users must align wgpu 30.0.1 /
Winit 0.30.13 and apply P2/P3.

Native/WASM external consumers compile with only those graphics adaptations.
Public WindowLevel fields/bounds, exhaustive encoding variants, five-argument
facade, two-argument constructors, custom View defaults, DICOM/sender macros and
browser return shapes remain. Fresh browser application declarations match the
release byte for byte excluding compiler-derived `readonly wasm_bindgen_*`
closure exports. Native CLI and HTTP router files are byte-identical to release.

## Applied production behavior

Canonical signed CT packing is exact across i16 with offset 32768. Published
`HuPackedRg8 { offset }`, `HuFloat`, `DEFAULT_HU_OFFSET = 1100`, and caller offset
interpretation remain. Dimensions/byte counts are checked before upload; float
and packed sampler policies preserve their release differences. R8 scalar and
R8Uint labels stay distinct; AI label restrictions do not narrow generic/DR labels.

Actual MPR/MIP/DVR shaders share decode code. Implicit sampling precedes divergent
MPR bounds; MIP/DVR use footprint LOD. Five layout checks compare every member
offset and struct span for MPR, MIP, DVR, needles and oblique planes. Legacy wrong
size assertions remain visible rather than being edited to force green results.

Format ownership, bounded depth, shared MPR contexts and dirty state cover release
overlays, primitives and cameras. Layouts prepare every replacement before commit.
GPU rebinding preserves CPU volume/labels, meshes, lighting, interaction and
replies. A GPU-only label without a CPU recipe rejects device replacement before
changing the old owner. This does not fix the separate new-volume A/B transaction
issue. ApplicationHandler retains every release event/reply and queued pre-run
command; incomplete frames do not mark presentation complete. Viewer and compute
share a wgpu generation but independent devices. The regression creates a second
GpuContext to represent an independent device owner, destroys it, and verifies
server compute/upload/offline HTTP work; it does not destroy a live App window.

## Verification matrix

Candidate: Rust 1.98.0, aarch64-apple-darwin; wasm-pack 0.15.0. Exact release uses
installed nightly 1.100.0 because of its unstable duration gate. Native release
tests initially fail linking because global ORT DirectML does not match macOS
binaries. Candidate scopes DirectML to Windows and retains native CPU fallback.
For a runnable baseline, a separate release extraction changes only ORT features
and platform guards: [normalization patch](candidate-evidence/release-native-platform-normalization.patch).
Rendering/data behavior and test expectations are unchanged.

| Check | Result / evidence |
| --- | --- |
| Locked native all-target check | Pass; [log](candidate-evidence/native-check.log) |
| Locked WASM library check | Pass; [log](candidate-evidence/wasm-check.log) |
| Optimized browser build after logger correction | Pass; [log](candidate-evidence/wasm-release-build.log) |
| Full native tests with required GPU availability | 443 passed, 18 failed, 19 ignored; [log](candidate-evidence/candidate-full-tests.log) |
| Platform-normalized release tests | 343 passed, same 18 failures, 32 ignored; [comparison](candidate-evidence/test-comparison.json) |
| Production GPU/encoding/resource/redraw checkpoint | 38 passed on Metal; [log](candidate-evidence/production-gpu-tests.log) |
| Independent compute/upload/offline GPU HTTP route | Explicit ignored test run: 1 passed; [log](candidate-evidence/compute-owner.log) |
| Offline router and DTOs | Pass; WS serialization/default commands also pass in full run; [log](candidate-evidence/offline-native-backend.log) |
| Production uniform layouts | 5 passed; [log](candidate-evidence/uniform-layout.log) |
| GPU browser logger | 1 passed; [log](candidate-evidence/browser-logger.log) |
| Native synthetic window/event loop | Normal exit; [log](candidate-evidence/native-smoke.log) |
| Public native/WASM consumer and browser declarations | Pass with approved graphics adaptations; fixtures/logs retained |
| Dependency trees | wgpu 30.0.1, Winit 0.30.13; no native Tokio/axum/ORT/serial/modbus leakage in WASM |
| Python tooling | 47 passed; [log](candidate-evidence/python-tests.log) |
| Current documentation | Zero issues; [log](candidate-evidence/docs-check.log) |
| OpenSpec 1.14 strict validation | 14 passed, zero failures; [log](candidate-evidence/spec-validation.log) |
| Format check | Fails in 35 files, all also fail baseline (61 files); [log](candidate-evidence/format.log) |
| Strict Clippy | Fails: 94 library / 111 library-test errors; [log](candidate-evidence/clippy.log) |
| Staged whitespace check | Production/current docs clean; frozen release bytes and generated log/patch context retain whitespace findings; [record](candidate-evidence/whitespace-check.json) |

The 18 failure names exactly match: WindowLevel/UID/checkerboard assertions,
MetaIO type/2D behavior, camera rotation assumptions and stale MPR/MIP sizes.
The suite is not globally green. Release Clippy reports 160/177 errors but uses a
different nightly; this is not warning-for-warning equivalence. Adaptation
warnings and existing quality debt remain unsuppressed. Scoped files were formatted.

The native window harness queues CT before run, exercises modes 0–4, projections
0–2, window/level, scale, pan, resize, invalid orientation preservation and quit.
Winit emits three pre-run handler diagnostics before normal operation; the log
retains them. Native OS hide/restore could not be performed: the automation
surface cannot bind this bare executable. GPU runtime tests required access to
Metal outside the sandbox; an initial sandbox adapter failure is not a GPU pass.

## Browser runtime acceptance

Final optimized package: Chrome 154 in the Codex in-app browser on macOS/Apple M5.
Only synthetic 16×16×8 signed CT and 16×16×1 DR images were used. The
[harness](candidate-evidence/browser-smoke.html) and
[curated results](candidate-evidence/browser-results.json) are retained.

Fresh WebGPU and forced WebGL2 pages pass startup/queued volume/run return, MPR
layouts, MIP/MinIP/AvgIP, DVR, oblique/pan/zoom, labels, balls/needle, OBJ import and
nonempty export, DR overlay, zero-size/restored resize, replacement canvas/getters
and quit. Final pages each have zero captured console errors. WebGPU selects
Bgra8Unorm; WebGL2 selects Rgba8Unorm through ANGLE Metal. Screenshots were inspected.

MPR replies remain `[400,1500]`. MIP/DVR window getters produce NaN (JSON
`[null,null]`), matching release MPR-only getter semantics. An early DR fixture
incorrectly supplied depth eight to a 2D overlay; depth one corrected its expected
byte-size rejection. A native smoke fixture needed a cfg guard for WASM. Neither
was resolved by changing production behavior. Verbose GPU shader logs initially
caused automation timeouts; the corrected private logger made final runs complete.

Controlled navigator.gpu absence and adapter failure each fall back to WebGL2
before binding a canvas, return MPR getters and quit without console errors.
These injected failures demonstrate fallback, not actual absence of host WebGPU.

## Accepted landing and deferred validation

Linux/Wayland GLES (unavailable on this Mac) and actual OS/native hide or
suspend/restore plus browser document visibility transitions remain unperformed.
Lifecycle unit checks, idle/frame scheduling and zero-size restoration pass but
do not certify those OS transitions. The user approved deferring these named
gates and continuing landing; [acceptance.md](acceptance.md) records the reply.
Task 6.4 is complete through explicit acceptance, while D1/D2 remain unchecked.
ORT models, Windows DirectML, live acquisition/navigation and clinical/physical
correctness are not claimed validated.

Phase D requires explicit acceptance/deferment of outstanding checks. That
condition is now met. `wgpu-upgrade` was rechecked at the release SHA and
fast-forwarded to candidate `41dff9f7cb630700ddb5929edcc369aa1cd8ab73`.
The original untracked proposal was preserved before replacement; unrelated
Python files retained their hashes. Dev, the release tag and backup refs remain.
The final documentation commit records completion without changing tested code.

The user requested stopping when the five-hour quota is exhausted and resuming
later. Honor that instruction without using reset credits to continue.
