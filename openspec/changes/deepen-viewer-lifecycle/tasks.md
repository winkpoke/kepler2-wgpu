# Tasks

The user authorized proceeding with the lifecycle and subsequent rebinding recommendations. Observable behavior is preserved; specs are intentionally skipped. Runtime acceptance is separate from deterministic tests and compilation.

Implementation, automated checks, preservation review, and native smoke are recorded in [verification](verification.md). Browser acceptance remains open; existing full formatting and strict Clippy failures remain visible.

## 1. Live lifecycle sequence

- [x] 1.1 Add the private lifecycle operation and primitive production/recording adapters, route existing dispatch, and retain frame/scheduler/input/public compatibility; verify deterministic production-interface tests cover redraw order, hidden/suspended/zero-size suppression, bounded scheduling, sticky fatal stop, Quit/Close distinctions, active-window replacement, failed replacement, and post-shutdown silence, plus native/WASM compilation.
- [x] 1.2 Document live ownership and test limits in architecture/testing references; verify documented source anchors and local links with docs checks.

## 2. Complete AppView rebinding

- [x] 2.1 Make AppView select its compatible replacement factory and rebuild existing views in one operation, migrate App callers and the existing bulk rebinding test; verify actual GPU context sharing, two-device/format state and pixels, pending presentation, compatible reuse, and unchanged standalone view behavior.
- [x] 2.2 Document the rebinding interface and retained device/format ownership in current rendering/architecture guides; verify source agreement and documentation links.

## 3. Integration acceptance

- [x] 3.1 Run locked native/WASM checks, full required-GPU tests, WASM packaging, scoped formatting, full formatting/strict Clippy, docs/spec/maintenance checks, and change validation; record actual outcomes and keep unrelated full-gate failures visible.
- [x] 3.2 Run synthetic native control smoke for mode changes, resize, error logging, and normal Quit through the changed dispatch; record actual adapter/runtime results and their limits separately.
- [ ] 3.3 Execute the synthetic browser lifecycle/layout procedure, record browser/backend/results/errors, and complete the previous layout-change browser task only with runtime evidence; leave both tasks open if browser control/upload prevents execution.
- [x] 3.4 Write verification evidence and review the scoped diff, public compatibility, and unrelated-file hash; verify the record matches actual tests and unperformed checks, without archiving incomplete acceptance.
