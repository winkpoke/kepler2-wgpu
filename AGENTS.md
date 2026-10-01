# Kepler2-WGPU agent instructions

Rust medical imaging application: native desktop and WASM, DICOM/MHA/MHD data,
MPR, MIP/MinIP/AvgIP, mesh rendering, and Remedy acquisition protocol.

## Scope and authority

- Make the requested change; preserve unrelated edits and public compatibility.
- Read the affected implementation and tests before editing. Read only the guides
  relevant to the task using the routing table below.
- Approved specs define intended behavior; source/tests show implemented behavior.
  If they conflict, report the discrepancy. Do not silently change medical,
  safety, or public API contracts to make a test pass.
- Use synthetic data. Keep patient-identifying data and secrets out of fixtures,
  logs, screenshots, and research artifacts.
- Live device connections, preparation, and exposure commands require explicit
  authorization. Default acquisition verification is offline protocol testing.

## Repository invariants

- Keep native-only dependencies, including Tokio, behind native target guards.
  The WASM build target is the library, not the native `kepler` binary.
- Use existing `KeplerResult`/domain error types and crate-level `log` macros.
- Use `glam` for geometry. Matrix upload is column-major; preserve Rust/WGSL
  uniform layout and alignment. A layout change requires matching shader tests.
- Preserve signed voxel data independently of display window/level. See the
  encoding contract below before changing texture formats or decode parameters.
- GPU resources belong to their device/attachment-format configuration. Rebuild
  format/device-dependent resources when that configuration changes; ordinary
  same-format resize is not a reason to rebuild pipelines.

## Read by task

| Task | Authoritative reference |
| --- | --- |
| Rust/platform changes | [Conventions](doc/agents/CONVENTIONS.md) |
| Build, tests, or verification | [Build matrix](doc/agents/BUILD.md) |
| Ownership or module boundaries | [Architecture](doc/architecture/ARCHITECTURE.md) |
| GPU resources, shaders, or redraw | [Rendering](doc/agents/RENDERING.md) |
| Voxel precision or texture encoding | [Volume encoding](doc/rendering/volume-encoding.md) |
| Fixtures, coverage, or regression tests | [Testing](doc/testing/README.md) |
| New capability, breaking contract, or material architecture/safety change | [OpenSpec workflow](doc/agents/OPENSPEC.md) and affected `openspec/specs/` |
| PR preparation | [PR checklist](doc/agents/PR_GUIDELINES.md) |

Docs corrections, localized fixes restoring accepted behavior, and small internal
refactors do not need proposal scaffolding. For significant changes, obtain
approval of scope and consequential design before implementation. A request to
explore or draft a proposal is not approval to implement it.

## Verification and completion

Use [BUILD.md](doc/agents/BUILD.md) for nightly/locked native, WASM-library, GPU,
and documentation checks. Add a regression for a bug fix where feasible.
Report commands actually run, outcomes, and unperformed/manual checks separately.
Keep failing tests and quality gates visible; do not mark deferred tasks complete.

OpenSpec is pinned locally: use `npm run openspec -- <arguments>`, including when
an upstream skill example says plain `openspec`. Install with `npm ci` if needed;
the global CLI may be an incompatible legacy version.

Update current docs/specs when their contracts change, and
[the changelog](doc/CHANGELOG.md) for user-visible changes. Dated research and
archives are evidence, not standing instructions. See [the documentation map](doc/README.md).
