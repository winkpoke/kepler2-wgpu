# Headless rendering verification

**Status:** Current test entry points. Original baseline measurements are preserved
in [the dated baseline](../archive/agent-guidance-2026-10-01/headless_gpu_baseline.md);
they are not current test counts. Rust graphics dependencies have not been upgraded
by the agent-guidance migration.

## What the native tests check

| Source | Behavioral evidence |
| --- | --- |
| [rendering_correctness_tests.rs](../../tests/rendering_correctness_tests.rs) | Real MPR/MIP/mesh pipelines, synthetic volume pixels, projection modes, depth, attachment formats, and existing-view rebinding |
| [volume_encoding_tests.rs](../../tests/volume_encoding_tests.rs) | Exhaustive signed-i16 CPU encoding, production decoder, GPU upload/readback and affine filtering |
| [render_resource_lifetime_tests.rs](../../tests/render_resource_lifetime_tests.rs) | Depth cache reuse/invalidation, shared immutable MPR contexts, uniform/presentation invalidation, and complete/empty frames |
| [redraw_state_tests.rs](../../tests/redraw_state_tests.rs) | Deterministic scheduling and pending work across idle/retry/visibility states |
| [shared GPU fixture](../../tests/common/headless_gpu.rs) | Device setup, validation scopes, offscreen targets, row-aligned copies, readback, and pixel expectations |

Factory-format and device-rebinding checks supplement the original explicit-format
baseline. The old process-wide surface-format limitation was removed by later
commits; it is not the current architecture. See [rendering](../agents/RENDERING.md).

## Running

Commands and prerequisites are canonical in [BUILD.md](../agents/BUILD.md).
For the focused native suites:

```bash
KEPLER_REQUIRE_GPU_TESTS=1 cargo test --locked \
  --test rendering_correctness_tests --test volume_encoding_tests \
  --test render_resource_lifetime_tests --test redraw_state_tests
```

A missing adapter is a failure for active rendering/resource tests. The environment
flag also prevents GPU decoder coverage from silently returning without execution.
Native headless tests use `WGPU_BACKEND`, not application `KEPLER_WGPU_BACKEND`.
The [GPU workflow](../../.github/workflows/gpu-tests.yml) configures software Vulkan
on Linux; a display server is not required. Report actual adapter/backend with results.

Use analytically known synthetic values and numeric tolerances rather than brittle
cross-driver screenshots. Preserve the existing expectations when migrating GPU APIs;
do not widen tolerances solely to hide a regression. Refer to
[the encoding contract](../rendering/volume-encoding.md) for storage exactness,
half-float rounding, and GPU sampling error distinctions.

## Limits

Offscreen native tests do not prove browser/WebGL behavior, every arbitrary camera
orientation, live surface negotiation, clinical suitability, FPS/power gains, or
driver-memory leak freedom. Resource counters are allocation evidence, not memory
profiling. Manual native/browser interaction remains a separate completion criterion.

The [resource-lifetime verification record](../../openspec/changes/optimize-render-resource-lifetimes/verification.md)
records prior automated results and outstanding manual checks. Do not reinterpret
prior scoped coverage as whole-repository or branch coverage.
