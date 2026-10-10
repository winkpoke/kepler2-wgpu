# Candidate evidence

This directory records the release-adapted candidate, not the source branch's
historical verification. See [execution](../execution.md) for outcomes and limits.
`manifest.json` hashes the artifacts and changed production/verification sources.
The manifest excludes itself; evidence commit identity is supplied by Git.

Baseline native runtime uses only the documented ORT platform-normalization patch.
Candidate full tests report 443 passes and the same 18 baseline failure names.
Quality failures remain visible. Browser results are a curated record of observed
CUA controls and console output, with the exact synthetic harness retained.

The rustdoc audit counts public re-export aliases separately and does not compare
trait implementations. Native/WASM consumer fixtures cover approved P2/P3 and
upstream graphics changes while keeping viewer/browser calls. To reproduce a
consumer, copy the manifest/lock into a temporary crate, replace `__KEPLER_PATH__`
with the candidate checkout and place `public-api-consumer.rs` at `src/main.rs`.
Use `native-smoke-consumer.rs` instead for the native `--smoke` event-loop run.

The browser harness imports a neighboring generated `kepler_wgpu.js`; serve it
with the corresponding optimized WASM package over loopback. Use fresh pages for
normal WebGPU, `?kepler_wgpu_backend=gl`, `?no_webgpu=1`, and `?fail_adapter=1`.
No generated binaries or patient data are committed here.
