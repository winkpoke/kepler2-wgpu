# Build and verification matrix

**Status:** Current executable reference. Run commands from the repository root.
[rust-toolchain.toml](../../rust-toolchain.toml) pins stable Rust 1.98.0 for native
and WASM builds. Use locked Rust dependencies; changes to the lockfile are a
separate dependency decision. The development pin is not an MSRV promise.

## Prerequisites

- Rustup: plain Cargo commands install/use the repository compiler pin, including
  `rustfmt`, Clippy, LLVM tools, and the `wasm32-unknown-unknown` standard library.
- Verify selection with `rustup show active-toolchain` and `rustc -Vv`.
  Explicit `+<toolchain>`, `RUSTUP_TOOLCHAIN`, and rustup directory overrides can
  bypass the file; remove unintended overrides before verification.
- `wasm-pack` for producing the browser package; Python >=3.11 for the CI TOML
  resolver and maintenance-script tests (standard-library `tomllib`). Python 3
  suffices for serving/manual checks.
- Node >=20.19 and npm for project-local OpenSpec maintenance: `npm ci --ignore-scripts`.
  Commit `package-lock.json`; no global OpenSpec upgrade is needed.

## CI compiler selection

The [shared Rust setup](../../.github/actions/setup-rust/action.yml) uses the
[fail-closed TOML resolver](../../scripts/ci_toolchain.py) to read the same
compiler/components/targets as local rustup. Its immutable installer revision
is separate from the compiler release. Every Rust job reports and verifies the
active release after setup, before installing tools or restoring build caches.

[Platform checks](../../.github/workflows/rust-checks.yml) run independent native,
library-test, WASM-library, formatting, and strict-Clippy jobs; a failed style/lint
job does not cancel compiler checks. The dedicated GPU and coverage jobs retain
required adapter execution, and [release builds](../../.github/workflows/release.yml)
use locked native and wasm-pack Cargo arguments. Inspect hosted job logs for the
actual compiler and outcomes: configured workflows are not proof they passed.

## Checks by scope

| Scope | Command | Evidence / limitation |
| --- | --- | --- |
| Native types | `cargo check --locked --all-targets` | Compiles native library, binary, and test targets |
| Native behavior | `cargo test --locked` | Includes active GPU tests; requires an adapter |
| Library behavior | `cargo test --locked --lib` | Includes GPU-backed application unit tests; not CPU-only |
| WASM types | `cargo check --locked --lib --target wasm32-unknown-unknown` | Library only; does not exercise browser behavior |
| Rust formatting | `cargo fmt -- --check` | Read-only; report existing unrelated failures |
| Rust lint | `cargo clippy --locked --all-targets -- -D warnings` | Strict; report baseline warnings/errors, do not silently suppress them |
| Current docs/delta identities | `npm run docs:check` | Local inline links and pending requirement targets |
| Maintenance-script regressions | `npm run docs:test` | Python stdlib tests, no GPU needed |
| Spec syntax | `npm run specs:check` | Baseline specs and active changes, not implementation correctness |

If `python3` is older than 3.11, run maintenance tests directly with a supported
interpreter, e.g. `python3.11 -m unittest discover -s scripts/tests -p 'test_*.py'`.
On systems where Python 3 is named `python` rather than `python3`, use that
supported interpreter directly. PowerShell environment syntax
is `$env:NAME='value'`; POSIX examples below use `NAME=value command`.

## Native run and GPU checks

```bash
cargo run --locked --bin kepler
KEPLER_REQUIRE_GPU_TESTS=1 cargo test --locked \
  --test rendering_correctness_tests --test volume_encoding_tests \
  --test render_resource_lifetime_tests --test redraw_state_tests
```

GPU correctness tests fail if no adapter exists. Software Vulkan (Mesa lavapipe)
is suitable on Linux; the [GPU workflow](../../.github/workflows/gpu-tests.yml)
contains its native package/ICD setup. Headless tests select `WGPU_BACKEND`, while
the application selects `KEPLER_WGPU_BACKEND`; these are different variables.
See [headless evidence and limitations](../testing/headless_gpu.md).

For application diagnosis, use `RUST_LOG=debug`, `KEPLER_WGPU_BACKEND` (e.g. metal
or vulkan), and `KEPLER_WGPU_VALIDATION=true` as applicable. Normal native logging
defaults to INFO, with noisy GPU modules filtered by `src/lib.rs`.

## Browser package and manual runtime verification

```bash
wasm-pack build --target web --out-dir pkg --release -- --locked
python3 -m http.server 8000
```

Serve the **repository root** so `static/` and generated `pkg/` remain reachable.
Open the appropriate page under `http://localhost:8000/static/`. Use synthetic
volumes and record browser/version/backend, rendering, resize, input, idle/redraw,
restoration, and errors. Merely building WASM or serving the page is not a passed
browser test. The repo currently has no npm/Playwright browser-test suite; browser
checks remain manual unless a separate automation change is approved.

`static/remedy.html` can actuate hardware. Do not connect or send live prep/exposure
commands without explicit authorization; acquisition tests normally exercise the
offline protocol. See [repository safety boundaries](../../AGENTS.md).

## Coverage and known baseline limitations

Use [coverage methodology](../testing/coverage.md) for metric/scope/tool evidence.
Do not infer repository coverage from test counts or scoped file measurements.

The [render-resource verification report](../../openspec/changes/optimize-render-resource-lifetimes/verification.md)
records existing global formatting failures and denied Clippy findings in
`image_info.rs` and DICOM validation tests. Check current output before treating
these as unchanged; report scoped success separately from full-gate failure.
Do not reformat unrelated files, disable tests, or mark a blocked check passed.

For every change, report the actual commands, exit status, meaningful test counts,
and missing environments/manual work. Documentation-only changes normally need
the docs/script/spec checks rather than a complete Rust rebuild; Rust contract
corrections additionally warrant the affected domain tests.
