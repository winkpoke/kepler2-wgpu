# Rust and platform conventions

**Status:** Current reference. Dependency versions/features come from `Cargo.toml`
and `Cargo.lock`, not documentation copies.

## Platform and async boundaries

Use target guards for native filesystem/runtime/window behavior versus browser APIs.
Tokio and Rayon dependencies are native-only; browser async work uses
`wasm-bindgen-futures` and existing `async_lock` synchronization where appropriate.
Use the synchronization primitive of the affected owner; do not introduce a runtime
or block the browser/render loop merely to match an example.

For a synchronous native test awaiting GPU setup, `pollster::block_on` is available.
A plain `#[test] fn` containing `.await` is a compile error, not a hanging test.
See [the build matrix](BUILD.md) for native and WASM-library verification.

## Errors, logging, and public APIs

- Use the existing `KeplerResult<T>` or subsystem-specific result/error type.
  Propagate actionable context without leaking patient identifiers or secrets.
- Use crate-level `log` macros. Native logging honors `RUST_LOG` (INFO by default)
  and filters noisy GPU modules. WASM uses the custom `web_sys::console` logger
  in `src/lib.rs`; do not add a competing logger.
- Gate expensive high-frequency diagnostics with `trace-logging` and sample them
  where appropriate. Leave them off in normal release builds.
- Preserve external API compatibility and review `src/lib.rs` re-exports when
  changing public interfaces. Re-export only intended public API, not every helper.
- Mesh support is always compiled. The optional feature currently declared for
  diagnostics is `trace-logging`; there is no `mesh` feature flag.

## Geometry and data

`Base` owns a `glam::Mat4`; world/screen/voxel labels describe spaces, not separate
Rust coordinate wrapper types. Record units and the source/destination spaces of
transforms. World geometry uses physical spacing, not an assumed one-to-one voxel
mapping. Upload `glam` matrices in column-major order without an extra transpose
unless converting explicitly row-major input at the input boundary.

Storage decode parameters and window/level are separate contracts. Read
[volume encoding](../rendering/volume-encoding.md) before touching signed voxels,
texture formats, or filtering. Read [rendering](RENDERING.md) for uniform layout
and device/format-dependent resource ownership.

## Tests and changes

- Use synthetic fixtures from the [testing reference](../testing/README.md).
  Prefer existing shared helpers to duplicate per-file fixtures.
- Add a minimal regression for the behavior being fixed. Keep checks deterministic;
  ordinary correctness tests should not enforce arbitrary wall-clock thresholds.
- Benchmark performance separately with recorded hardware/backend, data, build
  settings, and baseline. Counts or coverage percentages do not prove correctness.
- Document public API purpose, units, valid ranges, and meaningful error cases.
  Let `rustfmt` handle layout rather than duplicating language style tutorials.
- Follow [architecture](../architecture/ARCHITECTURE.md) ownership boundaries and
  [the PR checklist](PR_GUIDELINES.md); avoid unrelated cleanup.
