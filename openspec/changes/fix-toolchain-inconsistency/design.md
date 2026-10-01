# Design

## Context

See [proposal.md](proposal.md) for motivation and scope. Read-only discovery found:

- The pre-change `src/lib.rs` enables `duration_millis_float`; no other Rust feature gates were found. `render_pass.rs` has eleven `.as_millis_f64()` calls. Stable compilation additionally exposed three `.as_millis_f32()` calls in `application/app.rs`, `rendering/view/mesh/mesh_view.rs`, and `rendering/view/mesh/performance.rs`; their inclusion was explicitly approved.
- `DurationExt` already converts seconds to floating-point milliseconds using stable APIs. Its public trait also exposes other conversions; removing or renaming those methods is unnecessary.
- The cross-platform clock uses native `std::time::Instant` and browser `performance.now()`. WASM elapsed durations currently truncate to whole milliseconds; that behavior is separate from duration conversion and remains unchanged.
- All four Rust workflows select floating nightly. There is no repository toolchain pin or dedicated native/WASM check-and-lint workflow. Release builds omit locked project dependency flags.
- The coverage gate already aggregates explicit source-line counts, checks an explicit medical scope, preserves 45%/80% thresholds, and declares trend detection pending. Both coverage workflows install a floating CLI. The historical verification report identifies a mapping issue with cargo-llvm-cov 0.6.23 on nightly; it does not prove any stable pairing works.
- Installed stable reports Rust 1.98.0; installed nightly reports 1.100.0-nightly. Neither has been verified against this proposed change. Current documentation prescribes nightly and records unrelated global formatting and denied Clippy failures.

This crosses source, platform verification, CI, and documentation boundaries, so a design artifact is warranted. No baseline specification changes are needed (`skip_specs: true`).

## Goals / Non-Goals

**Goals:** One explicit compiler selection for default local commands and Rust CI; stable duration conversion without a public API break; compatible, versioned coverage tooling; auditable verification results.

**Non-Goals:** See the proposal. In particular, reproducibility here means compiler/project lockfile/coverage CLI selection, not a fully hermetic operating-system, driver, GitHub runner, or browser environment. It does not establish an MSRV or change GPU correctness criteria.

## Decisions

### 1. Reuse DurationExt without changing clock ownership

Use fully qualified trait dispatch at all fourteen call sites: `DurationExt::as_millis_f64(&duration)` or `DurationExt::as_millis_f32(&duration)`, importing the trait alongside the existing clock where appropriate. Method-call syntax produced `unstable_name_collisions` warnings on stable because the standard library has similarly named unstable methods; explicit dispatch preserves the existing helper and avoids that ambiguity. Remove the library feature gate and verify every caller on the selected stable compiler. Keep the trait, re-exports, and existing native/WASM guards intact; do not introduce another duration helper or runtime.

Tests in `src/core/timing.rs` explicitly invoke both extension methods. Use predetermined expectations: zero -> 0.0 ms, 250 microseconds -> 0.25 ms, 1500 milliseconds -> 1500.0 ms, and a multi-second duration with a fractional component. Use exact equality for exactly representable examples and explicit absolute tolerances in milliseconds for non-exact examples (1e-9 for the f64 fixture; 0.002 for the f32 fixture, accommodating f32 rounding while rejecting millisecond truncation). No wall-clock performance threshold is added.

Alternatives: Calling `as_secs_f64() * 1000.0` at every site duplicates the established abstraction. Renaming/removing trait methods risks public compatibility. Repairing browser fractional timing changes behavior and is deferred.

### 2. The repository toolchain file is the compiler source of truth

Use an exact stable release in `rust-toolchain.toml`, `profile = "minimal"`, components `rustfmt`, `clippy`, and `llvm-tools-preview`, and target `wasm32-unknown-unknown`. Start verification with installed stable 1.98.0. Commit that version only after locked native all-target and WASM-library compilation and timing regressions succeed. If it fails, identify whether the blocker is toolchain compatibility or an unrelated pre-existing defect before selecting another exact stable candidate.

Local commands use plain `cargo`/`rustc`; explicit `+nightly`, a rustup directory override, or `RUSTUP_TOOLCHAIN` can bypass the pin, so instructions must identify those overrides and verification must record `rustup show active-toolchain` and `rustc -Vv`. Do not set `Cargo.toml`'s `rust-version` merely to duplicate the development pin: an MSRV claim needs different evidence.

Alternatives: Floating stable still changes the compiler over time; dated nightly remains an unnecessary nightly dependency. Duplicating the release in every workflow creates multiple update points.

### 3. Share CI setup and add an explicit platform verification gate

Introduce a repository-local composite setup action under `.github/actions/setup-rust/`. After checkout it reads the toolchain TOML with Python's standard-library TOML parser on the Ubuntu runners, validates that the channel is an exact stable release, and supplies channel/components/targets to an immutable revision of the Rust setup action. Missing or malformed configuration must fail rather than fall back to stable/nightly. Export the resolved compiler for provenance, and keep the installer revision distinct from the compiler pin.

All four Rust workflows use this setup. Replace the release job's old toolchain action rather than leaving an override that defeats the repository pin. Install/cache coverage tools only after the compiler selection, and include compiler/tool versions in any new cache key.

Add a pull-request/main/develop verification workflow with native locked all-target checking, locked library tests, WASM locked library checking, formatting, and strict all-target Clippy. Use the native package/GPU prerequisites needed by the actual tests; do not label library tests CPU-only if they initialize GPUs. Separate jobs or independent check steps should keep formatting/lint findings from hiding compiler results. Retain the dedicated headless pixel tests and adapter-required behavior rather than inferring rendering correctness from line coverage.

Use `--locked` for release native builds and pass locked Cargo arguments through wasm-pack. No workflow compiles the native `kepler` binary for the WASM target. Preserve native Tokio guards, shader layouts, tolerances, and GPU driver setup.

Alternatives: Repeated YAML compiler literals drift. Relying on an installer's implicit default-file behavior obscures version resolution; a small shared resolver makes it auditable and testable. A new dependency management system is unnecessary.

### 4. Pin measurement tooling, not coverage results

Store the verified exact cargo-llvm-cov CLI release `0.6.23` in `scripts/ci-tool-versions.toml`; this version passed the focused stable timing-test mapping smoke. The Python >=3.11 `scripts/ci_toolchain.py` resolver validates both the toolchain and coverage release declarations, fails closed, and exposes the version to the shared CI setup. Unit tests cover valid, missing, malformed, and floating inputs plus every workflow's use of the shared pin. Document local installation from the same declaration. Install with `cargo install cargo-llvm-cov --version <declared-version> --locked` under the selected compiler. Leave the existing dev-dependency and application lockfile unchanged; explain that it does not install the CLI.

Choose the CLI by a successful smoke measurement on the selected stable compiler, not by assuming the historical version is compatible. Validate that executed unit code appears in the LLVM report before trusting an aggregate. In each coverage artifact retain revision, full compiler and CLI versions, feature/target configuration, backend/adapter evidence, the exact coverage scope, test command/pass-fail/ignored results, and the declared CLI version. Preserve the test log and outcome even on failure; upload artifacts with an always condition. Export JSON/LCOV/HTML as appropriate from one instrumented run per job; derive secondary report formats from the saved coverage data, do not rerun tests to obtain another format, and do not mix reports from different compiler versions or rerun just to satisfy thresholds.

Keep `coverage_summary.py`, medical scope, and 45%/80% policy unchanged. Use its JSON summary as the newly measured compiler-specific reference, not a new trend implementation or evidence that thresholds passed. Keep trend detection pending until a comparable prior artifact exists. Do not invoke the legacy grep/scalar regression script as acceptance evidence.

Alternatives: A floating CLI undermines compiler pinning. Lowering thresholds, excluding failing paths, or using the scalar legacy report hides measurement failures rather than repairing reproducibility.

## Risks / Trade-offs

- Stable incompatibility elsewhere -> Run locked native/WASM checks before declaring the pin validated; stop for material source/dependency changes outside the timing refactor.
- Method resolution or tiny floating-point rounding differences -> Explicit trait regression calls, fractional-duration fixtures, documented tolerance, and no timer-resolution or display changes.
- Existing formatting/lint/tests/coverage failures -> Record full command, exit status, and failure identity alongside scoped success. Keep failing checks blocking; do not add suppressions or continue-on-error to imply success. Recheck historical findings instead of assuming they persist.
- Coverage mapping anomalies -> Inspect unit-code mappings and retain raw reports and toolchain evidence; invalid measurement is not zero coverage or a passed gate.
- Pin does not freeze runner/driver/wasm-pack -> Record environment versions. Broader dependency/tool hermeticity is outside scope; do not promise identical cross-platform percentages.
- Missing GPU/manual browser evidence -> Retain software Vulkan CI and actual local adapter results; browser execution is a separate, unperformed check unless explicitly carried out with synthetic data.

## Migration Plan

1. Capture current nightly compiler/build and formatting/lint outcomes; keep application dependencies unchanged.
2. Add deterministic trait regressions, wire the existing trait into rendering, remove the feature gate, and verify the initial stable candidate against native/WASM targets.
3. Add the compiler and CLI declarations, shared setup, CI platform gates, and locked release commands. Verify resolution is independent of a developer's default nightly.
4. Run the existing full suite and headless suites, then one native coverage measurement with the selected tool pair. Preserve threshold failures and trend limitations.
5. Update only current guidance and changelog, run documentation/script/spec validation, and attach an implementation verification record. Planning status is not build evidence.

Roll back timing wiring, feature gate, toolchain/setup declarations, affected workflows, and current documentation together if compiler compatibility cannot be established. Preserve rendering/medical tests and existing coverage policy throughout.

## Open Questions

- Which exact cargo-llvm-cov release produces valid unit mappings with the selected stable compiler? Resolve through the focused smoke measurement before recording the CLI pin as verified.
- Does Rust 1.98.0 pass both locked platform builds after the timing-only refactor? It is the first candidate, not a claimed tested release; the stable-selection approach and acceptance checks do not depend on that particular version.
