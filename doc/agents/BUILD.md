# Build and verification matrix

**Status:** Current executable reference. Run commands from the repository root.
The crate requires nightly (`src/lib.rs` uses `duration_millis_float`). Use locked
Rust dependencies; changes to the lockfile are a separate dependency decision.

## Prerequisites

- Nightly Rust, including `rustfmt`/Clippy when using those checks.
- WASM standard library: `rustup target add --toolchain nightly wasm32-unknown-unknown`.
- `wasm-pack` for producing the browser package; Python 3 for serving/manual checks.
- Node >=20.19 and npm for project-local OpenSpec maintenance: `npm ci --ignore-scripts`.
  Commit `package-lock.json`; no global OpenSpec upgrade is needed.

## Checks by scope

| Scope | Command | Evidence / limitation |
| --- | --- | --- |
| Native types | `cargo +nightly check --locked --all-targets` | Compiles native library, binary, and test targets |
| Native behavior | `cargo +nightly test --locked` | Includes active GPU tests; requires an adapter |
| WASM types | `cargo +nightly check --locked --lib --target wasm32-unknown-unknown` | Library only; does not exercise browser behavior |
| Rust formatting | `cargo +nightly fmt -- --check` | Read-only; report existing unrelated failures |
| Rust lint | `cargo +nightly clippy --locked --all-targets -- -D warnings` | Strict; report baseline warnings/errors, do not silently suppress them |
| Current docs/delta identities | `npm run docs:check` | Local inline links and pending requirement targets |
| Maintenance-script regressions | `npm run docs:test` | Python stdlib tests, no GPU needed |
| Spec syntax | `npm run specs:check` | Baseline specs and active changes, not implementation correctness |

On systems where Python 3 is named `python` rather than `python3`, run the Python
script/test command with that interpreter directly. PowerShell environment syntax
is `$env:NAME='value'`; POSIX examples below use `NAME=value command`.

## Native run and GPU checks

```bash
cargo +nightly run --locked --bin kepler
KEPLER_REQUIRE_GPU_TESTS=1 cargo +nightly test --locked \
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
RUSTUP_TOOLCHAIN=nightly wasm-pack build --target web --out-dir pkg --release -- --locked
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
