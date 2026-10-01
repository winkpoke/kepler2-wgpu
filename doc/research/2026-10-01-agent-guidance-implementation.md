# Agent guidance and OpenSpec implementation

**Date:** 2026-10-01  
**Status:** Implemented documentation/tooling changes; limitations below remain open.  
**Approved input:** [the agent-instructions review](2026-10-01-agent-instructions-review.md)  
**Starting baseline:** `947b4fc` (`perf(rendering): optimize redraw scheduling and resource lifetimes`), clean worktree.

## Changes and authority

- [AGENTS.md](../../AGENTS.md) now routes by task and defines approval, synthetic-data,
  live-device, compatibility, precision, and verification boundaries. Cline,
  Copilot, and Trae files are thin adapters, not competing policy manuals.
- [The documentation map](../README.md) separates current reference, active OpenSpec
  work, and dated history. [Architecture](../architecture/ARCHITECTURE.md),
  [conventions](../agents/CONVENTIONS.md), [build procedures](../agents/BUILD.md),
  and [testing](../testing/README.md) own their respective topics.
- Nine duplicate testing plans/guides were moved to
  [the dated archive](../archive/agent-guidance-2026-10-01/). Historical banners make
  proposals and old coverage estimates non-executable. Old `doc/agents/` paths
  remain redirects. The prior headless report and legacy project context are also
  preserved there. Original relative examples may assume their former location;
  use the current reference map rather than treating those examples as live APIs.
- Matrix guidance now uses glam column-major upload without an extra transpose.
  Rendering guidance defers to [the committed encoding contract](../rendering/volume-encoding.md):
  packed RG8 preserves every signed i16; R16Float can round. Removed absent-feature,
  nonexistent-module, invented test-script, and arbitrary GPU-drop advice.
- Acquisition now has Purpose/Requirements sections and three discoverable
  requirements. Testing fixes distinguish StudyInstanceUID from SeriesInstanceUID,
  validate positive window width/effective level/normalized midpoint, allow either
  coordinate handedness, and reference actual fixture/test locations.
- Core/application/rendering/acquisition/testing specs now identify verification
  sources and evidence limits. The pending coverage delta modifies existing
  `Test Coverage Targets`, not nonexistent `Medical Path Coverage Enforcement`;
  complete replacement scenarios retain baseline obligations. Pending milestones
  were not silently synced into baseline specs.

## OpenSpec migration and recovery

Installed exact dev dependency **`@fission-ai/openspec@1.14.0`** with a new
npm lockfile. `npm ci --ignore-scripts --no-audit --no-fund` was rerun successfully;
local CLI reports 1.14.0. No global CLI upgrade was performed.

The migration was rehearsed on a copy before the repository invocation:

```bash
npm run openspec -- init --tools pi,codex,opencode,cline \
  --profile core --force --no-animation
```

Generated modern skills/commands under `.pi/`, `.agents/`, `.opencode/`, `.cline/`,
and `.clinerules/`. Retired legacy trigger manuals/commands. Useful context is
in [config.yaml](../../openspec/config.yaml), with policy in
[the OpenSpec guide](../agents/OPENSPEC.md), rather than a duplicate 456-line manual.
Config validation and generated proposal/apply/archive instructions confirmed
artifact rules, context, and operation guidance are actually consumed.

Before cleanup, a local backup (`legacy-guidance.tar.gz`), rehearsal checkout, and
`immutable-hashes.json` were created. `/tmp/kepler-agent-migration-path.txt` records
the ephemeral backup directory; do not assume temp files are durable recovery.
Git baseline `947b4fc` retains removed legacy tracked files; archived documentation
preserves the superseded testing/project context. Restore individual files with
reviewed diffs rather than resetting a worktree containing subsequent changes.
To reverse generated integration changes, inspect this migration's diff and
explicitly remove only its new generated paths; do not run indiscriminate cleanup.

SHA-256 verification found **zero changes to 173 protected files**: Rust source/tests,
Cargo manifests/lock, the active render-resource change, and existing OpenSpec
archives. No changes were synced or archived and no outstanding tasks were checked:

| Active change | Modern CLI status after migration |
| --- | --- |
| `optimize-render-resource-lifetimes` | 22/23; manual work remains open |
| `implement-test-coverage-plan` | 0/295 |
| `refactor-mip-view-glam` | 0/6 |

The modern CLI counts nested checklist items differently from the legacy CLI;
that task-count increase is not evidence of lost or completed work.

## Executable checks and coverage handling

- [check_docs.py](../../scripts/check_docs.py): current inline Markdown local links,
  repository-boundary checks, and pending ADDED/MODIFIED/REMOVED/RENAMED identities.
  Historical documents, remote URLs, anchors, and fenced examples are intentionally
  outside this check; OpenSpec still owns syntax validation.
- [coverage_summary.py](../../scripts/coverage_summary.py): LLVM JSON native source-line
  counts, weighted denominators, explicit [medical file scope](../testing/coverage-scope.json),
  and fail-closed missing/invalid data. Integration-test/dependency counts cannot
  inflate production-source coverage. Existing thresholds stay 45% overall/80% medical.
- [Documentation CI](../../.github/workflows/docs.yml) installs the locked local tool
  and runs script regressions, link/delta checks, and strict spec validation.
- [Coverage CI](../../.github/workflows/coverage-gate.yml) exports JSON/LCOV from one
  instrumented run, records tools/config/revision, retains artifacts even on gate
  failure, and explicitly labels missing comparable trend evidence. It no longer
  scrapes the same human summary as both medical and overall coverage or compares
  a provenance-free scalar with `HEAD~1`.

See [coverage methodology](../testing/coverage.md) for native/inline-test/branch
limitations. Branch gating and a trustworthy comparable trend baseline remain
coverage-plan work; this migration does not claim they are implemented.

## Verification performed

| Check | Outcome |
| --- | --- |
| `npm ci --ignore-scripts --no-audit --no-fund` | Passed; local OpenSpec 1.14.0 |
| `npm run docs:test` | 29 maintenance-script regression/CLI tests passed |
| stdlib `trace` over the maintenance suite | Source-line coverage: link/delta checker 85%, coverage summarizer 97%; not Rust/branch coverage |
| `npm run docs:check` | Current links and pending identities pass |
| `npm run specs:check` | **8 passed, 0 failed**, replacing the previous 7/1 result |
| `cargo +nightly check --locked --all-targets` | Passed, with existing warnings |
| `cargo +nightly check --locked --lib --target wasm32-unknown-unknown` | Passed; not browser execution |
| `cargo +nightly test --locked --lib core::window_level` | 8 passed; other library tests filtered |
| `cargo +nightly test --locked --test patient_safety_tests --test coordinate_precision_tests` | 31 passed, 4 existing ignored; model assertions are not proof of parser rejection |
| `KEPLER_REQUIRE_GPU_TESTS=1 cargo +nightly test --locked --test rendering_correctness_tests --test volume_encoding_tests --test render_resource_lifetime_tests --test redraw_state_tests` | 32 passed; native headless checks, not browser/hardware/performance validation |
| Existing LLVM JSON through the new coverage CLI | Correctly accepted/exported; default gates correctly failed: 2412/9360 overall and 449/1208 medical lines. This is a prior artifact, **not a fresh coverage run** |
| `npm audit --json` | 0 reported vulnerabilities at execution time; not a comprehensive security audit |
| Changed Python/YAML/current-guide LSP diagnostics | No error/warning findings after fixes |
| `git diff --check` and protected-file hashes | Passed; runtime/source/archives remain unchanged |

The headless run proved an adapter was available; captured passing-test output did
not identify its model/backend. Do not infer a measured adapter identity from the
host platform. Linux software-Vulkan CI execution remains separate from this local run.

### Existing quality-gate failures preserved

- `cargo +nightly fmt -- --check`: **exit 1**, existing formatting differences in
  `coordinate_safety_tests.rs`, `patient_safety_tests.rs`, and `volume_integrity_tests.rs`.
- `cargo +nightly clippy --locked --all-targets -- -D warnings`: **exit 101**, existing
  library/library-test lint errors (36/53 reported), including coordinate/style,
  acquisition dead code, argument-count/API patterns, and `image_info.rs` inherent
  methods. The strict lint command stops before every later target can be checked.

No source edits, warning suppression, tolerance widening, or unrelated formatting
were used to make these gates appear green. This is not an all-green PR readiness claim.

## Not performed / follow-up

- No full native test/coverage campaign, fresh Rust coverage measurement, branch
  instrumentation, comparable trend comparison, browser runtime check, live serial
  device/exposure, performance benchmark, or driver-memory profiling.
- GitHub workflows were configured and locally syntax-checked, not run on GitHub.
- No controlled agent/model evaluation. The changes remove specific contradictions
  and introduce checkable contracts; no measured frontier-model productivity gain
  or universal instruction-length claim is made.
- Keep manual rendering work open. Address existing formatting/lint and deeper
  medical rejection-path/coverage gaps as separately scoped changes.
