# Execution tasks

Implementation is authorized. The 13-commit replay and available candidate checks
are complete. User clarification and approvals are recorded in
[public-exceptions.md](public-exceptions.md). Details and
verification limits are recorded in [execution.md](execution.md).

## 1. Baseline and public interface protection

- [x] 1.1 Recheck the three pinned SHAs, branch/worktree states, and untracked files.
- [x] 1.2 Create backup refs and a separate candidate from the source tip; leave existing worktrees intact.
- [x] 1.3 Copy/hash release API evidence with provenance and audit externally reachable Rust items, target guards, exported macros, and generated senders.
- [x] 1.4 Generate/reference browser bindings and create unchanged-consumer compile/API fixtures.
- [x] 1.5 Capture release HTTP/WebSocket/CLI contracts and native/WASM/GPU/quality baseline outcomes.
- [x] 1.6 Obtain explicit decisions for P1–P4; record exact signatures, alternatives, and downstream migrations. Do not infer approval from old integration documents.
- [x] 1.7 Record medical/spec/test discrepancies and any required behavioral decisions before changing their expectations.

## 2. Replay and resource foundation

- [x] 2.1 Prepare a merge-preserving interactive rebase with phase stops and a commit/resolution ledger covering all 13 source commits.
- [x] 2.2 Adapt `685e419` without renaming published encoding variants or changing legacy raw-upload offset interpretation.
- [x] 2.3 Adapt `57d4cee` and `d6c044a`; preserve release data/export APIs and exercise its actual production shaders.
- [x] 2.4 Adapt `1edaf80` with approved/additive format interfaces and independent BGRA/RGBA ownership.
- [x] 2.5 Adapt `947b4fc`; include all release visual state in invalidation/resource bounds.
- [x] 2.6 Pass encoding, pipeline, format/resource, and redraw checkpoint checks, documenting baseline limitations.

## 3. Tooling, browser, and source merge

- [x] 3.1 Adapt `6136c46` while preserving release feature/spec content and package identity/version.
- [x] 3.2 Recreate and adapt the `b254673` browser branch and `34bc2aa` toolchain branch.
- [x] 3.3 Cover all release unstable timing calls with portable helpers; preserve units and fractional timing.
- [x] 3.4 Recreate `37a027e` and separately review its merge-specific evidence/resolutions.
- [x] 3.5 Pass locked stable native and WASM-library compile checks; validate docs/spec/tooling syntax.

## 4. Graphics upgrade and release features

- [x] 4.1 Reconcile native/WASM dependencies and lockfile for source wgpu/Winit versions; preserve the compute alias/public paths and independent device owners.
- [x] 4.2 Adapt `6ff5a0e` under approved public graphics exceptions, with GPU-ready construction, handler/proxy behavior, and native display handles.
- [x] 4.3 Port descriptor/error-scope/presentation/recovery changes to all release production paths.
- [x] 4.4 Transfer deleted-path fixes to MPR/DVR/shared camera/release mesh code; do not resurrect old renderer interfaces or opaque assets.
- [x] 4.5 Verify CT/R8 scalar/R8Uint label separation and matched production Rust/WGSL layouts.
- [x] 4.6 Pass native/WASM, generated binding, public compile/API, and actual production GPU checkpoint tests.
- [x] 4.7 Run compute-owner and mocked/offline HTTP/DTO checks, preserving the release router, CLI, exports, and local WASM behavior.

## 5. Layout and lifecycle

- [x] 5.1 Adapt `f42fab2` through private preparation/commit operations with the published release facade and layout semantics intact.
- [x] 5.2 Test partial preparation failures without lost views, orientation, mesh, overlay, or presentation state.
- [x] 5.3 Adapt `918e36b` and preserve every release event/reply through the lifecycle owner.
- [x] 5.4 Rebind all release resources on device/format change and test CPU state, overlay/primitive state, getters, pre-run commands, replacement events, and idle behavior.
- [x] 5.5 Carry `8e308cb` as historical evidence and write a separate candidate verification report.

## 6. Acceptance and landing

- [x] 6.1 Review range-diff, recreated merge resolutions, destination diff, cleanly applied patches, and source-to-candidate commit mapping.
- [x] 6.2 Require no unapproved public API differences; verify unchanged release consumer calls and return/serialization semantics.
- [x] 6.3 Run the full candidate verification matrix and compare quality failures against the exact baseline.
- [x] 6.4 Complete required synthetic native/browser/GL/GLES/manual checks, or obtain an explicit decision for a named unavailable gate.
- [x] 6.5 Review specs/changelog/migration docs without blanket-completing imported historical tasks.
- [x] 6.6 Recheck the destination and fast-forward `wgpu-upgrade` only after accepted implementation gates; leave dev/tags/other worktrees unchanged.

## Explicitly deferred validation

The user approved deferring these gates and continuing landing on 2026-10-05.
Task 6.4 is complete through its explicit-decision alternative, not a platform
pass. These checks remain unperformed:

- [ ] D1 Run Linux/Wayland GLES acceptance on a suitable host.
- [ ] D2 Exercise actual native OS hide/suspend/restore and browser document visibility transitions.

See [acceptance.md](acceptance.md) for the decision and landing record. Existing
baseline test and quality failures remain documented in execution.md.
