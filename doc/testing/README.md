# Current testing reference

**Status:** Current procedures. [BUILD.md](../agents/BUILD.md) owns execution commands;
[baseline specs](../../openspec/specs/) define intended behavior, not achieved safety
or coverage. Remaining work belongs to [active changes](../../openspec/changes/).

## Fixtures and regressions

Use [tests/common/mod.rs](../../tests/common/mod.rs) and the actual shared modules:
[DICOM](../../tests/common/fixtures/dicom.rs), [volumes](../../tests/common/fixtures/ct_volume.rs),
[patients](../../tests/common/fixtures/patient.rs), and [MHA/MHD](../../tests/common/fixtures/format.rs).
Read their function signatures rather than copying historical mock structs.
Generate synthetic input and minimize dimensions except when scale itself is tested.
For parser rejection tests, corrupt a valid serialized fixture at the relevant boundary;
an assertion about a hand-built object's fields is not proof of parser rejection.

For a bug, write a reproducer that fails for the original behavior, fix the behavior,
and retain the active regression. Use the most relevant existing test module rather
than forcing every regression into one file. Record the issue/contract and expected
behavior; a missing issue number is not a reason to omit a regression. Disabled,
ignored, and empty tests are not execution evidence.

## Behavioral verification map

| Contract | Actual verification sources | Limits |
| --- | --- | --- |
| Patient/study/series relationships | [patient tests](../../tests/patient_safety_tests.rs), [metadata tests](../../tests/dicom_metadata_validation_tests.rs) | Some cases assert model state rather than enforcing validation; inspect assertions |
| Signed pixel data and rescaling | [DICOM pixel tests](../../tests/dicom_pixel_data_validation_tests.rs), [encoding tests](../../tests/volume_encoding_tests.rs) | Storage precision does not prove every parser error path |
| Coordinates and geometry | [precision](../../tests/coordinate_precision_tests.rs), [safety](../../tests/coordinate_safety_tests.rs), [core tests](../../src/core/coord/mod.rs) | Units, ranges, tolerances, and handedness must be explicit |
| Window/level | [WindowLevel unit tests](../../src/core/window_level.rs), [GPU pixels](../../tests/rendering_correctness_tests.rs) | Normalized display midpoint is 0.5 at effective level; clamped display is not invertible |
| MHA/MHD and volumes | [format tests](../../tests/medical_imaging_tests.rs), [volume tests](../../tests/volume_integrity_tests.rs) | Audit implemented rejection paths, not merely fixture invariants |
| Rendering / resource ownership | [headless tests](headless_gpu.md) | Native pixels/state; not browser, clinical, wall-clock, or driver-memory certification |
| Remedy protocol | [offline protocol/unit tests](../../src/acquisition/remedy.rs), [protocol reference](../acquisition/remedy-serial-web.md) | Live hardware, serial transport, and exposure checks require explicit authorization |

Use seeded property-based inputs with realistic valid domains and explicit edge
cases. Derive expected values independently from the implementation where possible.
A passing syntax validator, test count, or coverage percentage is not a clinical
safety guarantee or regulatory assessment.

## Coverage, performance, memory, and browsers

- [Coverage methodology](coverage.md) separates metrics, scope, targets, and measurements.
- [Headless GPU checks](headless_gpu.md) exercise real production pipelines and pixels.
- Performance benchmarks need a specified build, hardware/backend, data, baseline,
  and tolerance. Correctness CI should not assert arbitrary elapsed-time thresholds.
- Resource-allocation/cache tests check bounded ownership/reuse. They do not measure
  driver memory leaks or process RSS; Valgrind/browser memory automation is not
  implemented merely because an archived plan describes it.
- WASM library compilation and manual browser checks are separate. Follow the
  [browser GPU smoke procedure](browser_gpu.md); the repository does not currently
  provide npm/Playwright browser-test scripts.

The [archived testing guidance](../archive/agent-guidance-2026-10-01/) preserves
old proposals and measurements, including unimplemented API examples. Do not
reintroduce those examples as executable requirements or call old percentages current.
