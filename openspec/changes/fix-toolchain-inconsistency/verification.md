# Toolchain implementation verification

## Status

Implementation and local verification record updated 2026-10-01. Stable Rust 1.98.0 passes native/WASM checks, library tests, locked release builds, and required local GPU suites. Coverage mapping is valid but both unchanged coverage thresholds fail; full formatting and strict Clippy retain baseline failures. Hosted GitHub Actions and manual browser/native interaction remain unperformed; this record does not claim CI is green.

## Pre-change environment

- Git revision: `6136c46f1eb8ed177d883690931965175169cbe5`.
- Host: `aarch64-apple-darwin`.
- Installed channels: stable and nightly; default/active channel: nightly. No `RUSTUP_TOOLCHAIN` or `CARGO_TARGET_DIR` environment override was reported.
- Stable: `rustc 1.98.0 (88d9e12ae 2026-08-18)`, LLVM 22.1.8.
- Nightly: `rustc 1.100.0-nightly (e7769602a 2026-08-24)`, LLVM 23.1.0.
- Nightly targets include native and `wasm32-unknown-unknown`.

## Baseline checks

| Command | Exit | Evidence |
| --- | --- | --- |
| `cargo +stable check --locked --all-targets` | 101 | Dependencies compiled/checked; library and library-test compilation rejected `src/lib.rs:1` with E0554: feature attributes cannot be used on stable. This is a source feature-gate failure, not a dependency-fetch or missing-target failure. |
| `cargo +nightly check --locked --all-targets` | 0 | Native library, binary, and test targets check successfully; existing warnings remain. |
| `cargo +nightly check --locked --lib --target wasm32-unknown-unknown` | 0 | WASM library checks successfully; eight existing unused-import warnings remain. |
| `cargo +nightly test --locked --lib` | 0 | 46 passed, 0 failed, 0 ignored. Includes GPU-backed application tests; no adapter failure, but this run did not log adapter/backend identity. |
| `cargo +nightly fmt -- --check` | 1 | Existing differences in `tests/coordinate_safety_tests.rs`, `tests/patient_safety_tests.rs`, and `tests/volume_integrity_tests.rs`. |
| `cargo +nightly clippy --locked --all-targets -- -D warnings` | 101 | 36 library and 53 library-test errors, including `inherent_to_string_shadow_display` at `image_info.rs:239`, `empty_line_after_doc_comments` in timing/pipeline, `too_many_arguments`, `items_after_test_module`, and test float precision/range findings. No suppressions or unrelated repairs made. |

Raw baseline logs: `/tmp/kepler-toolchain-baseline-{stable,native,wasm,lib,fmt,clippy}.log` (ephemeral, not durable CI artifacts). Baseline quality failures are recorded as planned; they do not establish a new compiler failure or authorize unrelated repairs.

## Timing regressions before gate removal

`cargo +nightly test --locked --lib core::timing::tests`: exit 0; 5 passed, 0 failed, 0 ignored, 44 filtered out. Tests explicitly dispatch through `DurationExt`, retaining zero, 0.25 ms, 1500.0 ms, and multi-second fractional values. The fractional multi-second case uses an absolute tolerance of 1e-9 milliseconds. Raw log: `/tmp/kepler-toolchain-timing-nightly.log`.

The pre-change stable rejection is the compiler-level reproducer; the conversion regressions characterize the already-working helper before renderer wiring changes.

## Initial pause: stable compiler exposed additional timing callers

The first gate-removal attempt revealed additional f32 callers; the following failed checks were recorded before the approved correction:

| Command | Exit | Evidence |
| --- | --- | --- |
| `cargo +stable check --locked --all-targets` | 101 | E0658 for three `as_millis_f32()` calls: `src/application/app.rs:1104`, `src/rendering/view/mesh/mesh_view.rs:737`, and `src/rendering/view/mesh/performance.rs:173`. Eleven renderer f64 calls now use the trait but emit `unstable_name_collisions` warnings recommending fully qualified trait dispatch. |
| `cargo +stable test --locked --lib core::timing::tests` | 101 | Same three f32 compilation errors; tests did not execute. |
| Active LSP diagnostic probe of changed Rust files | clean response | No findings reported for timing/lib/render_pass; this limited probe does not override the actual Cargo compilation failures in other modules. |

The scope correction was explicitly approved: three additional source files, f32 conversion regressions, and explicit dispatch at all fourteen callers. Proposal/design/task 2.2 were updated before continuing. The ephemeral native/timing logs were subsequently overwritten by successful corrected runs; the initial error identities are preserved in the table above. No patient data was opened and no application/device operation was run.

## Corrected timing and stable candidate

- Before caller changes: `rustup run nightly rustc --edition=2021 --test src/core/timing.rs -o /tmp/kepler-toolchain-timing-tests` and the resulting standalone module test executable both exited 0; 7 tests passed. This directly exercised the additional f32 regressions without re-enabling the crate feature gate.
- `cargo +stable check --locked --all-targets`: exit 0; existing unrelated warnings only, no duration method collision warnings.
- `cargo +stable test --locked --lib core::timing::tests`: exit 0; 7 passed, 0 failed, 0 ignored, 44 filtered out.
- `cargo +stable check --locked --lib --target wasm32-unknown-unknown`: exit 0; eight pre-existing unused-import warnings.
- `cargo +stable test --locked --lib`: exit 0; 51 passed, 0 failed, 0 ignored.
- Compiler for all corrected checks: Rust 1.98.0, commit `88d9e12ae`, LLVM 22.1.8. Default channel remains nightly; these checks explicitly selected the installed stable channel.
- Source scan found no remaining feature attributes or method-syntax floating-millisecond calls under `src/`/`tests/`. Public timing methods, clock behavior, and application lockfile remain unchanged.

Raw corrected logs: `/tmp/kepler-toolchain-stable-{native,timing,wasm,lib}.log`.

## Repository compiler pin

`rustup toolchain install 1.98.0 --profile minimal --component rustfmt --component clippy --component llvm-tools-preview --target wasm32-unknown-unknown` exited 0. The repository `rust-toolchain.toml` now selects the exact release with these components/target. Plain `rustc -Vv` reports 1.98.0; `rustup show active-toolchain` reports the repository file override while the machine default remains nightly.

Plain `cargo check --locked --all-targets`, `cargo test --locked --lib core::timing::tests`, and `cargo check --locked --lib --target wasm32-unknown-unknown` all exited 0; timing tests: 7 passed, 0 failed, 0 ignored. Raw logs: `/tmp/kepler-toolchain-pinned-{native,timing,wasm}.log`. `Cargo.toml` and `Cargo.lock` have no diff.

## Documentation and shared setup

- Current BUILD/RENDERING/headless guidance, README build-reference link, and changelog updated for the compiler pin. `npm run docs:check` and `git diff --check` exited 0; archives and overlapping change artifacts have no diff.
- Shared Rust setup delegates TOML validation to `scripts/ci_toolchain.py`; eight configuration/CLI regressions passed with `python3.11 -m unittest discover -s scripts/tests -p test_ci_toolchain.py`. They initially failed because the resolver had not been implemented (red-before-green).
- Installer revision `02cb101ec7c40f2c49e1d9714d64511d8e1b74de` verified with `git ls-remote` against dtolnay/rust-toolchain master; cloned action metadata was read to verify explicit toolchain/components/targets inputs and its minimal installation profile. This immutable installer revision does not determine the compiler release.
- TOML tooling/tests require Python >=3.11, documented in BUILD. The host default `python3` is 3.9.6; `python3.11` is 3.11.14 and resolves stdlib `tomllib`. A Pyright missing-import diagnostic from its older interpreter was marked false-positive, not suppressed in source. Direct resolver execution returned the exact repository compiler/components/target.

All four existing Rust workflows now use shared setup before tool installation/cache restoration. The release checkout was aligned to the repository's immutable v4 revision, removing its outdated v3 runner. Coverage environment exports were grouped without changing values to satisfy shell validation. Configuration tests now include workflow adoption: 9 passed (initially four workflow subtests failed before rewiring). `actionlint` 1.7.12 and `npm run docs:check` exited 0.

Added `.github/workflows/rust-checks.yml` with five independent matrix jobs (native types, library tests, WASM library types, formatting, strict Clippy), fail-fast disabled, shared exact setup, compiler-aware cache keys, and native/software-Vulkan prerequisites where needed. Configuration tests passed 10/10 after a missing-workflow red test; actionlint and whitespace checks exited 0. Hosted execution is not claimed.

## Locked release builds

- `cargo build --release --locked`: exit 0 on Rust 1.98.0 (52.57 seconds); one pre-existing dead-code warning. Raw log: `/tmp/kepler-toolchain-release-native.log`.
- Installed wasm-pack 0.15.0 with `cargo install wasm-pack --locked --root /tmp/kepler-toolchain-build-tools` (exit 0); no global executable was replaced.
- `wasm-pack build --release --target web -- --locked`, with the temporary tool's bin directory prepended to PATH: exit 0. Generated ignored `pkg/` artifacts, using wasm-bindgen CLI 0.2.100 to match the project dependency. Raw log: `/tmp/kepler-toolchain-release-wasm.log`. This is build evidence, not browser execution.
- Release workflow uses both locked project commands, locked wasm-pack installation, and native Linux build prerequisites. The release regression initially failed on missing flags; after correction all 11 setup/workflow tests and actionlint passed.
- `Cargo.toml`/`Cargo.lock` remain unchanged. wasm-pack itself is not version-pinned by this change; its observed version is retained here, consistent with the stated reproducibility boundary.

BUILD now documents shared fail-closed setup, installer/compiler pin distinction, independent platform checks, and locked release commands, with links to actual files. Docs checks, all 11 setup/workflow tests, actionlint, and whitespace checks exited 0. Local compiler identity is recorded above; hosted GitHub Actions has not been run and remains unperformed.

## Coverage CLI compatibility smoke test

- Installed cargo-llvm-cov 0.6.23 under Rust 1.98.0 using `cargo install cargo-llvm-cov --version 0.6.23 --locked --root /tmp/kepler-toolchain-coverage-tools` (exit 0). The existing global executable was not replaced.
- `cargo llvm-cov test --locked --lib --json --output-path coverage/smoke/coverage.json core::timing::tests`: exit 0; 7 passed, 0 failed, 0 ignored. The explicit `test` subcommand is needed for positional filtering in this CLI release; an initial top-level filter invocation was rejected before building and corrected after reading its help.
- Verified all seven timing test function mappings have execution count 1. Timing source-line summary: 47/56 covered lines, including inline test code. This is a focused smoke scope, not repository coverage or a passed medical/overall gate.
- Raw instrumented data and revision/compiler/CLI provenance are retained locally under ignored `coverage/smoke/`; log: `/tmp/kepler-toolchain-coverage-smoke.log`. The historical nightly mapping anomaly did not reproduce with this stable pairing.

## Coverage CLI pin and CI provenance

- Added `scripts/ci-tool-versions.toml` pinning the verified `cargo-llvm-cov 0.6.23`; shared TOML resolver validates the exact release and emits the CI output. The setup action exposes it to both coverage workflows; both install exactly that version with `--locked`. The existing Cargo dev-dependency and `Cargo.lock` remain unchanged.
- Expanded resolver/workflow checks to cover exact/missing/malformed/floating coverage CLI declarations, local version output, no output on failed validation, and both jobs' single instrumented invocation/provenance/artifact conditions. `python3.11 -m unittest discover -s scripts/tests -p test_ci_toolchain.py`: 18 passed. `actionlint`, docs check, Python formatting check, and whitespace check pass.
- Coverage workflows record revision, full compiler and CLI versions, default-feature/native/backend configuration, scope JSON, lavapipe `vulkaninfo --summary`, complete test log, exit/count outcome, and upload artifacts with `if: always()`. Coverage gate exports JSON in its one test invocation and derives LCOV from the saved profile; report job similarly derives HTML from its one LCOV test invocation.
- Verified on the pinned compiler/CLI that `cargo llvm-cov test --locked --lib --lcov ... core::timing::tests` executes 7/7 tests and that `cargo llvm-cov report --html` succeeds from the same saved profile. Logs/reports: `/tmp/kepler-toolchain-coverage-{lcov,html}-smoke.log`, `/tmp/kepler-toolchain-coverage-smoke.lcov`, and `/tmp/kepler-toolchain-coverage-html/`.

## Coverage methodology documentation

`doc/testing/coverage.md` now documents installation from the exact shared declaration, Python >=3.11 resolver invocation, stable compiler and coverage commands, one instrumented run, artifact provenance, and limits. The existing `scripts/coverage_summary.py`, `doc/testing/coverage-scope.json`, 45% overall / 80% medical thresholds, trend-pending warning, and `implement-test-coverage-plan` artifacts are unchanged (`git diff --exit-code` verified). `python3.11 -m unittest discover -s scripts/tests -p test_coverage_summary.py`: 14 passed; resolver/workflow tests: 18 passed; docs checks and whitespace validation pass.

## Committed-pin plain-Cargo checks

With no `+toolchain` override or `RUSTUP_TOOLCHAIN`, `rustc -Vv` reports Rust 1.98.0 and `rustup show active-toolchain` names the repository `rust-toolchain.toml` override. All of these plain-Cargo commands exited 0:

| Command | Result |
| --- | --- |
| `cargo check --locked --all-targets` | Native library, binary, and test targets compile/check; pre-existing warnings remain. |
| `cargo test --locked --lib` | 51 passed, 0 failed, 0 ignored; includes application/rendering GPU-backed tests. |
| `cargo check --locked --lib --target wasm32-unknown-unknown` | WASM library checks; eight pre-existing unused-import warnings remain. |

Raw logs: `/tmp/kepler-toolchain-final-{native,lib,wasm}.log`. This is native macOS test evidence; a specific adapter/backend was not logged. The CI software-Vulkan setup is configured but hosted Actions has not run.

## Full native and GPU suites

- `KEPLER_REQUIRE_GPU_TESTS=1 WGPU_BACKEND=metal cargo test --locked -- --nocapture`: exit 0. Across 28 test/doc-test summaries, 327 passed, 0 failed, 23 ignored (existing ignored tests/doc-tests; not disabled by this change). This ran all four requested GPU integration targets along with the rest of native targets. Log: `/tmp/kepler-toolchain-full-native-gpu.log`.
- Explicit required GPU run: `KEPLER_REQUIRE_GPU_TESTS=1 WGPU_BACKEND=metal RUST_LOG=info cargo test --locked --test rendering_correctness_tests --test volume_encoding_tests --test render_resource_lifetime_tests --test redraw_state_tests -- --nocapture`: exit 0; 32 passed, 0 failed, 0 ignored. Logs identify adapter `Apple M5`, device type `IntegratedGpu`, backend `Metal`; detailed results in `/tmp/kepler-toolchain-focused-gpu.log`.
- Synthetic offscreen pipeline/pixel assertions ran with existing tolerances; no tests were disabled, ignored by command filters, or modified to pass. No patient data/live device operations used.

## Full native coverage measurement

`KEPLER_REQUIRE_GPU_TESTS=1 WGPU_BACKEND=metal RUST_LOG=info cargo llvm-cov test --locked --json --output-path coverage/full/coverage.json`: test command exit 0; 327 passed, 0 failed, 20 existing ignored across 27 summary lines. Full compiler/tool/feature/scope/revision and adapter metadata, raw run log, outcome, coverage JSON, and Python summary are retained locally under ignored `coverage/full/`. The adapter log identifies Apple M5 IntegratedGpu on Metal. The JSON includes 57 measured source files and timing unit-function mappings.

`python3.11 scripts/coverage_summary.py coverage/full/coverage.json --output coverage/full/summary.json --overall-min 45 --medical-min 80`: **exit 1, valid below-threshold measurement** — overall 4,554/10,564 (43.11%, minimum 45%); medical 910/1,407 (64.68%, minimum 80%). The failing measurements are preserved, not turned into a passing gate; thresholds, scope, and denominator were not changed. Existing `test_coverage_summary.py` regressions (14 passed) exercise malformed inputs and threshold failure behavior. Coverage trend remains pending; this is not a comparable historical baseline.

## Formatting and strict Clippy quality gates

- `cargo fmt -- --check`: exit 1, with pre-existing formatting diffs only in `tests/coordinate_safety_tests.rs`, `tests/patient_safety_tests.rs`, and `tests/volume_integrity_tests.rs` (same three files as the pre-change baseline). No files were formatted or unrelated formatting applied. Log: `/tmp/kepler-toolchain-final-fmt.log`.
- `rustfmt --edition 2021 --check` on all six changed Rust files: exit 0. Log: `/tmp/kepler-toolchain-scoped-fmt.log`.
- Pinned-toolchain `cargo clippy --locked --all-targets -- -D warnings`: exit 101; 40 library and 57 library-test errors, in existing code. Log: `/tmp/kepler-toolchain-final-clippy.log`. The pre-change nightly baseline was already 36 library and 53 library-test errors. The added timing test and trait-dispatch lines did not introduce Clippy diagnostics; errors in changed files point to pre-existing functions/comments (e.g. `App::set_render_mode`, `PassPlan`, and render-pass methods), not inserted lines.
- As specified in the design's risks/trade-offs, these strict quality failures remain visible and blocking; the new strict Clippy CI job is intentionally not softened with `continue-on-error` or suppressions. No unrelated lint/format cleanup was applied.

## Final documentation, OpenSpec, and diff validation

- `npm run docs:test` with the host's default Python 3.9.6: exit 1 because `tomllib` is unavailable. Rerun using Python 3.11.14 as the `python3` on PATH (as documented for the resolver): exit 0, 47 tests passed. Logs: `/tmp/kepler-toolchain-final-docs-test.log` and `/tmp/kepler-toolchain-final-docs-test-py311.log`.
- `npm run docs:check`: exit 0, 0 issues. `npm run specs:check`: exit 0, all 9 specs/changes validated. `npm run openspec -- validate fix-toolchain-inconsistency --strict`: exit 0. `git diff --check`: exit 0. `actionlint`: exit 0. The 11 new untracked files also passed a trailing-whitespace scan.
- Final scope audit: changes are confined to the approved timing refactor/tests, repository toolchain and CI/setup/coverage provenance, current build/rendering/coverage docs, changelog, and this OpenSpec change. `Cargo.toml`, `Cargo.lock`, shaders, medical-domain source/spec contracts, `coverage_summary.py`, `coverage-scope.json`, 45%/80% thresholds, and `implement-test-coverage-plan` artifacts are unchanged. No archives or unrelated files were modified.

## Post-merge verification on `dev`

After merging with the concurrent `prefer-browser-webgpu` change, combined-tree verification also passed: `cargo check --locked --all-targets` (exit 0); `cargo test --locked --lib` (72 passed); `cargo check --locked --lib --target wasm32-unknown-unknown` (exit 0); and `KEPLER_REQUIRE_GPU_TESTS=1 WGPU_BACKEND=metal RUST_LOG=info cargo test --locked -- --nocapture` (348 passed, 0 failed, 23 ignored across 28 summaries; Apple M5 IntegratedGpu/Metal). `npm run docs:check`, Python 3.11 `npm run docs:test` (47 passed), `npm run specs:check` (10/10), `actionlint`, and diff checks also passed. Logs: `/tmp/kepler-toolchain-merged-{native,lib,wasm,full-gpu,docs-check,docs-test,specs-check,actionlint,diff-check}.log`.

## Outstanding external checks

Hosted GitHub Actions have not run. The new strict-Clippy job and existing coverage gate are expected to remain red on the recorded local results (Clippy has baseline errors; valid coverage is 43.11% overall / 64.68% medical against unchanged thresholds). Manual native interaction and browser execution are unperformed. No live acquisition/device commands were sent.
