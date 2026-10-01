# Proposal

## Why

Kepler currently requires nightly solely through the `duration_millis_float` feature gate, while local builds and Rust CI workflows use an unpinned nightly compiler. Removing that unnecessary dependency and establishing a shared stable compiler baseline isolates toolchain changes from the later winit/wgpu migration and makes failures and coverage measurements easier to reproduce.

## What Changes

- Use explicit `core::timing::DurationExt` calls for all fourteen floating-point millisecond measurements (eleven f64 render-pass calls and three f32 application/mesh calls), remove the nightly feature gate, and add deterministic f64/f32 conversion regressions. This additional source scope was approved after stable compilation exposed the f32 callers.
- Add `rust-toolchain.toml` with an exact verified stable release, minimal profile, rustfmt, Clippy, LLVM tools, and `wasm32-unknown-unknown`. Installed stable Rust 1.98.0 is the first candidate; compatibility has not yet been tested.
- Make GPU, coverage, and release workflows derive their compiler from that file, and add native/WASM compile, unit-test, formatting, and lint verification using the same pin.
- Pin the verified compatible `cargo-llvm-cov` 0.6.23 CLI release in `scripts/ci-tool-versions.toml` and share its version between both coverage jobs and documented local installation. Install the CLI with `--locked`; the existing Cargo dev-dependency is not a CLI installation or version pin.
- Use locked project builds in the affected workflows, update current build/timing/coverage instructions, and retain exact measurement provenance in both coverage artifact paths.

### Non-goals

No wgpu/winit or locked application-dependency upgrades; no public timing API removal, clock-source changes, or WASM timer-resolution changes; no shader, voxel, device-ownership, or medical-contract changes. Do not alter coverage scope, arithmetic, thresholds (45% overall / 80% medical), implement comparable-baseline trend enforcement, add GPU/browser test coverage, or repair unrelated formatting/lint/test failures. Do not edit the existing coverage-plan change or reinterpret its unchecked tasks as complete.

## Capabilities

### New Capabilities

None. This is a behavior-preserving timing refactor and build/CI tooling correction.

### Modified Capabilities

None. Existing `core` and `testing` requirements remain unchanged. This change declares `skip_specs: true`; no delta specification is created.

## Impact

- Source/tests: `src/lib.rs`, `src/core/timing.rs`, `src/rendering/core/render_pass.rs`, `src/application/app.rs`, `src/rendering/view/mesh/mesh_view.rs`, and `src/rendering/view/mesh/performance.rs`. There is one observed feature gate and fourteen millisecond calls; existing public helpers remain available.
- Build configuration: new repository toolchain pin and shared CI toolchain setup; native Tokio guards and the WASM library-only target remain intact.
- CI: `.github/workflows/gpu-tests.yml`, `coverage.yml`, `coverage-gate.yml`, and `release.yml`, plus a native/WASM verification workflow. These Rust workflows already use nightly; the issue is not a current stable-CI mismatch.
- Coverage tooling: `scripts/ci_tool_versions.toml`, fail-closed resolver, and resolver/workflow regressions. Both coverage workflows install the shared CLI pin and retain compiler/tool/configuration/adapter/scope/test-outcome artifacts. The existing JSON summarizer and medical scope are preserved. Trend enforcement is already explicitly pending; the legacy scalar regression script is not a reliable baseline and remains outside this change.
- Current documentation: `doc/agents/BUILD.md`, `doc/agents/RENDERING.md`, `doc/testing/coverage.md`, `doc/testing/headless_gpu.md`, a focused build-reference link in `README.md`, and `doc/CHANGELOG.md`. Leave dated research/archives untouched.
- Overlap: `implement-test-coverage-plan` owns coverage expansion and trend enforcement. Existing rendering-correctness tests are active, including pixel assertions; do not resurrect historical ignored-placeholder claims.

### Acceptance and compatibility

- Deterministic tests explicitly exercise the f64 and f32 extension methods for zero, fractional-millisecond, and multi-second durations, retaining millisecond units and floating-point values without integer truncation. All fourteen callers explicitly dispatch through the trait without new method-name collision warnings.
- The selected stable compiler passes locked native all-target and WASM-library compilation and the timing/unit tests; native full-suite and required headless GPU results are recorded separately.
- Rust CI resolves the same exact compiler as a clean local checkout, independent of the user's default toolchain. Coverage jobs install the same exact compatible CLI and retain revision/toolchain/tool/configuration provenance.
- Existing coverage scopes and thresholds are unchanged. A valid measurement below threshold remains a failed gate; it is not repaired by changing the denominator, ignoring tests, or lowering targets.
- Formatting, strict Clippy, documentation/script checks, and strict OpenSpec validation are run. Existing unrelated failures remain visible with scoped results; a toolchain fix is not a claim that all CI gates are green.

No runtime/public API break is intended. The compiler pin is a development baseline, not a newly declared minimum-supported-Rust-version promise. If the first stable candidate is incompatible, record the blocker and select another exact stable version without changing application dependencies; material source/API fixes require renewed scope approval. Roll back the source, compiler/setup, CLI-version, and documentation changes together to avoid restoring a mixed-toolchain configuration.
