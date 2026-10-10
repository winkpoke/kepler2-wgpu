# Headless rendering baseline (wgpu 23)

**Status: Historical evidence — statements/counts apply to the original baseline.**
Later commits removed global surface-format state and added resource/redraw tests.
Use [current headless checks](../../testing/headless_gpu.md). Archived on 2026-10-01.

## Review and scope

Pipeline construction and pixel readback are complementary: shader parsing or a
compute-only decoder test does not prove that the application's render pipeline,
bind groups, vertex layouts, uniforms and attachment formats work together.
Selected pixels with numeric tolerances are preferable to cross-GPU screenshot
goldens for this baseline.

`tests/rendering_correctness_tests.rs` replaces the 14 ignored, empty placeholders
with **10 active native GPU tests**. `tests/common/headless_gpu.rs` supplies device
creation, validation scopes, offscreen rendering, row-aligned texture copies,
mapped readback, BGRA-to-RGBA normalization and pixel assertions. Each test has its
own device, so parallel tests cannot interfere through device error scopes.

No dependency upgrade is included. The production shaders and volume encoding
are unchanged. Existing constructors retain their global-format behavior; new
`MprRenderContext::new_with_format` and `BasicMeshContext::new_with_format`
constructors provide explicit attachment formats without mutating global state.

## Coverage

| Test area | Behavioral assertion |
| --- | --- |
| Pipeline creation (3 tests) | Production MPR, MIP and basic lighting-enabled mesh pipelines validate; mesh is built with and without depth |
| HU rendering (2 tests) | AppModel encoding → RenderContent upload → real MPR **and** MIP shader → pixels, for RG8 and R16Float independently |
| MPR slices | Axis-distinct 4³ volume; axial/coronal/sagittal slices, two voxel-center slices, halfway interpolation and out-of-bounds black |
| MIP maximum | A single bright voxel wins on its ray, neighboring rays stay dark, and entirely negative rays retain negative maxima |
| Projection modes | MIP, MinIP and AvgIP match nine analytically known samples; upper-threshold filtering is checked for all three modes |
| Mesh depth | Near red triangle occludes later far blue triangle; without depth blue wins; untouched background remains unchanged |
| Attachment formats | MPR, MIP and mesh build **and render** to RGBA/BGRA unorm and sRGB; sRGB transfer and colored BGRA channel order are checked |

HU cases include `-32768`, `-1200`, `-1100`, `-1024`, `-1`, `0`, `1000`,
`2048`, `2049`, `3000` and `32767`. R16Float's intentional half-float rounding
(`2049 → 2048`, `32767 → 32768`) has explicit expectations. A narrow 65-HU
window makes a byte of output correspond to approximately 0.25 HU. HU roundtrips
allow ±1 output byte, which rejects two-HU errors even after display quantization;
interpolation, projection and format checks allow ±2. Solid mesh colors and
out-of-bounds black use exact bytes.

The existing `tests/volume_encoding_tests.rs` also checks CPU encoding over the
entire i16 domain and GPU decoding using the shared WGSL function. Those tests
remain complementary to, rather than substitutes for, actual render output.

## Running

Use nightly Rust: the crate currently requires `#![feature(duration_millis_float)]`.
The GPU and coverage CI workflows use nightly for the same reason.

```bash
KEPLER_REQUIRE_GPU_TESTS=1 cargo +nightly test --locked \
  --test rendering_correctness_tests --test volume_encoding_tests
```

The new rendering tests always fail when no adapter is available. The environment
flag makes the older compute-decoder test fail instead of skipping as well.
There is no ignored-test opt-in and no successful no-adapter return in the new
suite. Normal `cargo test` runs these rendering tests.

Native backend selection uses wgpu's `WGPU_BACKEND`, **not** the application's
`KEPLER_WGPU_BACKEND`. Adapter identity is logged when the test log is enabled:

```bash
RUST_LOG=warn,rendering_correctness_tests=info \
  cargo +nightly test --locked --test rendering_correctness_tests -- --nocapture
```

On Linux, a software Vulkan adapter is sufficient; no display server is needed:

```bash
sudo apt-get install libvulkan1 mesa-vulkan-drivers
WGPU_BACKEND=vulkan KEPLER_REQUIRE_GPU_TESTS=1 cargo +nightly test --locked \
  --test rendering_correctness_tests --test volume_encoding_tests
```

`.github/workflows/gpu-tests.yml` installs the native build dependencies and Mesa
lavapipe, selects its Vulkan ICD explicitly, and runs both suites on pull requests
and pushes to main/develop. Coverage workflows also install software Vulkan now
that ordinary test execution requires an adapter. Workflow actions are pinned to
verified upstream commit revisions.

## Baseline evidence

Local execution on **Apple M5 / Metal**, wgpu **23.0.1**:

- Rendering correctness: **10 passed, 0 ignored**.
- Existing volume encoding suite: **9 passed, 0 ignored**.
- Full `cargo test --locked`: **301 passed, 0 failed, 23 ignored**. The remaining
  ignored tests elsewhere in the repository are not GPU-baseline coverage.
- `cargo check --locked --all-targets` passed.
- Changed Rust files pass formatting checks and active LSP diagnostics.

Temporary mutation probes were run and then fully reverted:

| Deliberate regression | Observed failure |
| --- | --- |
| Remove packed storage bias in WGSL | HU roundtrip output 255 instead of 128 |
| Add 2 HU in the MPR shader only | Both RG8 and R16Float roundtrips fail: output 135 instead of 128 |
| Force MPR depth to zero | Slice pixel 0 instead of 42 |
| Initialize MIP maximum to zero | Negative-ray pixel 128 instead of 13 |
| Reverse mesh clip-space depth | Blue occludes red at the tested center pixel |
| Select unavailable DX12 backend on macOS | Adapter setup fails explicitly, rather than passing without execution |

Full Clippy remains blocked by pre-existing denied lints in
`src/data/medical_imaging/metadata/image_info.rs` (inherent `to_string` shadows
Display) and `tests/dicom_pixel_data_validation_tests.rs` (always-true i16 bound
comparisons). Repository-wide `cargo fmt` also encounters existing trailing
whitespace in unrelated files; formatting verification is scoped to changed Rust
files. No global coverage percentage is claimed.

## Limits and migration use

This is a small behavioral baseline, not complete renderer validation. It does
not test surface negotiation, interactive view/camera state, arbitrary rotated
rays, the separate advanced mesh shader, browser/WebGL behavior, performance or
GPU memory leak detection. The onscreen `SWAPCHAIN_FORMAT` OnceCell limitation is
**not fixed** here; the format matrix specifically proves explicit-format
pipeline construction and rendering without using that global.

Software-Vulkan CI is configured but was not executed locally; DX12, Vulkan and
browser parity still need execution on their respective environments. Tolerances
must not be widened merely to hide a migration regression.

Before a wgpu upgrade, run and preserve this baseline. After API changes, run the
same tests with unchanged synthetic scenes and expectations. A failure then
identifies the affected shader/pipeline behavior rather than just an API compile
error.
