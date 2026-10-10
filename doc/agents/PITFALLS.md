# Repository-specific pitfalls

Read only the cases relevant to the affected subsystem. Detailed contracts live
in [rendering](RENDERING.md), [encoding](../rendering/volume-encoding.md), and
[build/test procedures](BUILD.md).

| Failure mode | Correct boundary / verification |
| --- | --- |
| Native runtime imported into browser code | Keep Tokio behind native target guards; check the WASM library |
| Extra transpose during matrix upload | `glam::Mat4` upload is column-major; transpose only explicitly row-major input at its boundary |
| Packed RG8 treated as 8-bit medical precision | It stores exact offset-binary `i16`; R16Float can round large values |
| Storage bias applied to window/level | Keep storage decoding separate from medical display parameters |
| GPU formats stored globally | Use each graphics configuration's authoritative format; test independent factories and rebinding |
| Depth pool constructed per frame | Keep the depth-only texture/view cache in `GraphicsContext`, keyed by size/samples/depth format |
| Cache reused across devices | Recreate device-owned resources; release unused depth targets when mesh views disappear |
| Clean views skipped on a fresh surface frame | Draw every visible view after one clear; surface images are not retained content |
| Upload mistaken for successful presentation | Mark presentation complete only after the complete frame succeeds |
| No-op controls trigger continuous redraw | Compare CPU state; coalesce scheduling and use bounded animation/retry deadlines |
| Zero-sized/minimized surface acquired | Wait without acquisition and retain pending work for restoration |
| Browser compile check called a runtime test | Record actual manual browser/native checks separately |
| Passing OpenSpec validation called proof of behavior | Inspect scenario meaning and corresponding tests; delta targets must resolve |
| Every task checked despite manual work remaining | Update each task only on its own completion evidence |

The resource/redraw cases have regressions in `tests/render_resource_lifetime_tests.rs`,
`tests/redraw_state_tests.rs`, and `tests/rendering_correctness_tests.rs`.
Precision is checked in `tests/volume_encoding_tests.rs`. Keep those expectations
when changing implementations rather than weakening tolerances to conceal regressions.
