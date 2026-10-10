# Tasks

These tasks implement the [design](design.md) and its [application](specs/application/spec.md) and [testing](specs/testing/spec.md) contracts after a new request to apply the change. Implementation, automated checks, and native control smoke are recorded in [verification](verification.md). Browser runtime acceptance remains open; full repository formatting and strict Clippy gates remain failed for existing issues.

## 1. Production preparation seam

- [x] 1.1 Add the private transition module and internal request, borrowing existing application state without adding another owner; retain the public control signatures and verify native and WASM-library compilation plus existing successful transition behavior.
- [x] 1.2 Introduce narrow internal upload and view-construction adapters used by production and tests, keeping `AppView`'s concrete factory; verify a headless synthetic fixture drives the production operation and a factory decorator returns a contextual error at a chosen construction step.
- [x] 1.3 Document the live operation and existing ownership in `doc/architecture/ARCHITECTURE.md`; verify the documented entry point, module, and owners match source and local links pass `rtk npm run docs:check`.

## 2. Prepare and commit the complete transition

- [x] 2.1 Add regressions for returned upload failure and view-construction failure after partial candidate preparation, covering old view identities/settings/positions, source volume, orientation selections, mesh cache/flag, and clean versus pending redraw status; verify they expose destructive or partial updates before completing the new commit logic.
- [x] 2.2 Add minimal crate-private `DynamicLayout` support for complete candidate placement/replacement, preserving true parent dimensions and allowing a borrowed retained strategy; verify layout/redraw tests establish that failed preparation does not resize old views and success remains pending until a frame is presented.
- [x] 2.3 Prepare volume rendering content from read-only model state, stage generated or reused mesh content and related orientation/flag updates, and construct all candidates before mutation; verify the failure regressions pass without reloading the model volume or changing existing encoding and mesh-generation policy.
- [x] 2.4 Commit the candidate collection, applicable strategy, orientation selections, mesh cache, and mesh-mode configuration without remaining fallible preparation; verify partial-construction errors leave every staged update inactive and abandoned candidates follow existing resource ownership.
- [x] 2.5 Add real malformed-upload validation and before/after headless pixel checks for failed transitions; verify the old synthetic layout still renders and preserves its previous pending presentation status after the returned error.
- [x] 2.6 Document preparation, commit, returned-error limits, and temporary old-plus-candidate resource coexistence in current architecture/testing docs; verify links and that no device-loss recovery or driver-memory guarantee is claimed.

## 3. Control compatibility and state policy

- [x] 3.1 Route `App::set_render_mode` through the private operation and preserve native/browser names, argument order, units, and return shape; verify MPR → MIP → mesh → mixed → MPR through the production seam with expected orientation, count, positions, strategy, recreated defaults, and pending presentation.
- [x] 3.2 Cover legacy mixed modes, mesh-over-MIP overlap, explicit mesh reuse/regeneration, retained custom strategy, one-cell-to-grid behavior, and unused legacy arguments; verify production-seam tests preserve those valid behaviors.
- [x] 3.3 Reject missing volume, used out-of-range orientation/slot indices, and missing required mesh before commit, replacing misleading deferred-load reporting and the missing-mesh panic with contextual logging; verify unchanged old state, no deferred request, and numeric operation context without patient metadata.
- [x] 3.4 Preserve explicit capture/restore helpers and GPU-rebinding state behavior; verify modified old interaction settings reset only on successful recreation, while existing AppView rebinding/resource-sharing regressions still pass.
- [x] 3.5 Replace detached transition simulations only when matching production checks pass, update the testing guide and `doc/CHANGELOG.md` for the accepted state policy and failure handling, and verify each delta scenario maps to a real test or explicitly pending runtime check.

## 4. Integration verification and acceptance record

- [x] 4.1 Run `rtk cargo check --locked --all-targets`, `rtk cargo check --locked --lib --target wasm32-unknown-unknown`, and `rtk cargo test --locked` with the repository compiler pin; verify native and WASM results and preserve any failing quality gate in the completion record.
- [x] 4.2 Run `rtk cargo fmt -- --check` and `rtk cargo clippy --locked --all-targets -- -D warnings`; verify the scoped change and report current unrelated failures separately without suppression or unrelated reformatting.
- [x] 4.3 Run required-adapter rendering, volume-encoding, resource-lifetime, and redraw suites from `doc/agents/BUILD.md`, plus the new production transition/application tests; verify actual adapter execution and record commands, meaningful outcomes, and any failure rather than treating skipped GPU work as acceptance.
- [x] 4.4 Build the browser package with `rtk wasm-pack build --target web --out-dir pkg --release -- --locked`; verify successful locked packaging and record that this does not establish browser runtime behavior.
- [x] 4.5 Run `rtk npm run docs:check`, supported-interpreter maintenance-script tests, `rtk npm run specs:check`, `rtk npm run openspec -- validate deepen-view-layout-transitions --strict --no-interactive`, and `rtk git diff --check`; verify the source/docs/delta changes remain coherent and the unrelated `upgrade-wgpu-winit/verification.md` edits are preserved.
- [x] 4.6 Perform native synthetic-volume smoke checks using existing controls for all supported arrangements, modified interaction settings, resize, and error reporting; record observed outcomes separately and leave this task open if native runtime acceptance cannot be performed.
- [ ] 4.7 Perform the synthetic signed-volume browser procedure in `doc/testing/browser_gpu.md`, including existing layout controls, resize/input/redraw, and console errors; record browser/version/backend and observed outcomes separately, avoid live Remedy actuation, and leave this task open if browser runtime acceptance cannot be performed.
- [x] 4.8 Write a scoped verification record with actual command results, returned-failure evidence, resource coexistence limits, manual outcomes, and unperformed checks; verify no task or baseline requirement is reported complete solely from planning/spec validation, and do not archive while required acceptance remains incomplete.
