# Agent instructions, documentation, and OpenSpec review

**Reviewed:** 2026-10-01  
**Scope:** root and tool-specific agent instructions, `doc/agents/`, documentation organization, OpenSpec workflow/specs, and current first-party coding-agent guidance. Recommendations only; existing instructions and in-progress code were not modified. This report is the only repository file added by the review.

## Verdict

**Keep the approach; repair its authority model and modernize its delivery.** A concise entry point, task-specific documentation, and versioned requirements remain appropriate for modern frontier coding agents. This repository already has the right basic ingredients. Its main problem is contradictory and stale information, not an excessively large root `AGENTS.md`.

The root is 102 lines / about 506 whitespace-separated words. `doc/agents/` contains 18 Markdown files / 7,288 lines. The two OpenSpec manuals are each 456 lines and are 99.98% text-identical. These are inventory measurements, not the amount every agent automatically loads. Models, instruction discovery, skill loading, and tool enforcement are different layers; changing the model alone does not resolve contradictions.

Current first-party guidance favors concise persistent instructions and genuinely conditional reference/workflow loading. Codex documents a default combined startup-instruction budget of 32 KiB; Anthropic recommends under 200 lines per `CLAUDE.md` and warns that eager imports do not reduce context cost. Neither establishes a universal ideal `AGENTS.md` length or a measured slowdown in this repository. [S1–S3]

## What should stay

- Native/WASM target separation, including native-only Tokio.
- Explicit matrix/uniform layout and medical-data precision invariants.
- Device/attachment-format resource ownership, pipeline recreation rules, and rendering regressions.
- Synthetic fixtures and reproducible tests.
- Review/approval for new capabilities, breaking contracts, material architecture changes, and safety-sensitive behavior.
- OpenSpec's separation of baseline requirements, proposed deltas, implementation tasks, and history.
- Evidence-bearing verification reports. The active `optimize-render-resource-lifetimes/verification.md` is a good pattern: it distinguishes automated checks, existing failures, scoped coverage, and outstanding manual validation.

More capable models justify removing generic tutorials and rigid micromanagement, **not** removing domain constraints, approval boundaries, or verification.

## Concrete findings, in priority order

### 1. Correct domain-critical contradictions before pruning — high priority

**Matrix layout:** `openspec/project.md:11,50` says `cgmath` and instructs agents to transpose Rust row-major matrices before shader upload. `Cargo.toml` uses `glam`, `doc/agents/CONVENTIONS.md:178` describes column-major `Mat4`, and `openspec/specs/rendering/spec.md:79–82` requires column-major upload. The MIP implementation uploads `to_cols_array()` (`src/rendering/view/mip/mod.rs:562`). Following the stale transpose instruction can introduce a real rendering defect.

**Voxel precision:** `doc/agents/RENDERING.md:93–101` describes R16Float as full HU precision and RG8 as lossy 8-bit storage. Current `src/data/volume_encoding.rs:3–10` and `doc/rendering/volume-encoding.md` say the opposite for integer storage: packed RG8 preserves every `i16` value, while half-float can round large integers. Sampling arithmetic still has its documented float-error bounds; storage exactness and sampling precision are separate claims.

**Recommendation:** make the existing volume-encoding document authoritative for this topic; replace duplicated explanations with a pointer. Reconcile matrix instructions with the approved layout contract. When a specification and implementation disagree, report the discrepancy and obtain a decision rather than automatically treating either document or code as infallible.

### 2. OpenSpec is useful, but this installation is legacy — high priority

Local `openspec --version` returns **0.18.0**. As retrieved on 2026-10-01, GitHub's latest non-prerelease release and npm's `latest` channel both identify **1.14.0**, published 2026-09-30. [S4–S5]

The release-pinned migration guide replaces the phase-locked workflow with action-based OPSX, generated skills, and `openspec/config.yaml`. It specifically calls `openspec/AGENTS.md` an obsolete workflow trigger and describes removal of legacy marker blocks and command files. [S6]

**Recommendation:** perform a separate, pinned, reviewed tooling migration; do not merely delete the old file or run an unreviewed global update. Back up custom content first: current release notes explicitly warn that listed cleanup files are deleted entirely. Preserve active changes, archives, and baseline specs; selectively migrate useful `project.md` context. [S4,S6]

Choose only integrations actually used. Command rendering differs by harness: the guide describes `/opsx:*` commands for relevant integrations, while Codex uses generated skills in `.agents/skills/openspec-*`. Do not assume one tool's slash commands or instruction-discovery behavior works everywhere. [S1,S6]

If migration is deferred, pin and label the legacy CLI/workflow explicitly and retain one authoritative legacy guide.

### 3. Baseline specs need both structural and semantic repair — high priority

Executed:

```text
openspec validate --all --strict --no-interactive
7 passed, 1 failed (8 items)
```

`openspec/specs/acquisition/spec.md` lacks `## Purpose` and `## Requirements`. The CLI consequently lists acquisition as **zero requirements**, although the file contains three requirement headings. A targeted validation reproduced the error.

Passing validation is not a correctness guarantee. Examples in the passing testing spec:

- `openspec/specs/testing/spec.md:75` says a series' `SeriesInstanceUID` must match its parent's `StudyInstanceUID`. Current test/model usage distinguishes the series' own UID from its `study_uid` parent reference (`tests/patient_safety_tests.rs:339–383`). This is a contract inconsistency to resolve, not an instruction to make those identities equal.
- `openspec/specs/testing/spec.md:155–158` allows negative window widths and states `transform(level) = window / 2`. `WindowLevel` requires positive widths and its documented normalized display mapping yields 0.5 at the effective level (`src/core/window_level.rs:15–18,80–88,230–242`). Define the intended function, units, bias, and tolerance before correcting the requirement.
- `implement-test-coverage-plan/specs/testing/spec.md` uses `MODIFIED` for `Medical Path Coverage Enforcement`, a requirement name absent from the current baseline. Reconcile the delta target before syncing/archiving; standalone validation passed despite this mismatch.

**Recommendation:** establish a green structural baseline, then review important scenarios against executable domain tests. Link requirements to actual tests/checks, including exceptions and manual checks. Do not equate spec syntax validation or a coverage percentage with clinical safety or regulatory approval.

### 4. Multiple competing sources of truth amplify drift — medium priority

- `openspec/AGENTS.md` and `doc/agents/OPENSPEC.md` differ only in insignificant whitespace and an apostrophe.
- Architecture is repeated in `doc/agents/ARCHITECTURE.md`, `doc/architecture/ARCHITECTURE.md`, and tool-specific instructions.
- `.github/copilot-instructions.md:28` points to nonexistent `rendering/core/state.rs` and describes an older `RenderApp`/`State` ownership model.
- `doc/agents/CONVENTIONS.md:193–202` and `doc/architecture/ARCHITECTURE.md:115` describe a `mesh` feature flag absent from `Cargo.toml`; `src/lib.rs:29–30` explicitly says mesh is always available.
- `.trae/rules/project_rules.md:5` disallows Node-based WASM testing, while `doc/agents/wasm_testing_strategy.md:71,120–123` prescribes npm/Playwright commands. `package.json` defines neither `test` nor `test:integration`. The legacy rule also hardcodes Windows/Beijing operational assumptions.

**Recommendation:** maintain one canonical repository policy and use small tool adapters only for genuine discovery differences. Decide whether the old browser-testing restriction still applies; mark proposed automation as proposed rather than pretending it is executable today. Do not assume every harness loads `.trae` rules.

### 5. Separate current reference, backlog, and historical evidence — medium priority

`doc/agents/test_implementation_plan.md` is 1,463 lines, labels itself an active plan, and repeats work tracked by `openspec/changes/implement-test-coverage-plan/tasks.md`. Its phase-three target is 55%; the active delta specifies 65%. `test_strategy_comprehensive.md` repeatedly calls a historical 24% measurement current, without a fresh measurement in this review.

Some referenced paths are explicitly planned `(NEW)` files, not broken links. The problem is their placement alongside current guidance without a reliable status/authority distinction.

The split between `doc/redering/` and `doc/rendering/` also makes navigation harder. Avoid a broad rename until backlinks have been inventoried.

**Recommendation:** current documentation describes how the system works now; OpenSpec change tasks own active work; dated research/archives own past measurements and decisions. Add `Status`, `Verified against` (commit or release), and `Supersedes` only where useful. Remove onboarding pointers to obsolete plans, preserving their historical value.

### 6. Replace generic or misleading instructions with concrete completion criteria — medium priority

`doc/agents/PITFALLS.md:25–29` puts TypeScript's `as any` in a Rust example. Its async-test example (`:224–233`) claims a normal synchronous test containing `.await` will hang, although that example does not compile. Generic OpenSpec complexity thresholds such as `>1000 users` / `>100MB` (`openspec/AGENTS.md:387`) are not meaningful architectural gates for this volume renderer.

The legacy guide's `Before Any Task` checklist is too broad for documentation-only requests or localized fixes. Its instruction to mark every task checked after all work is done is also weaker than updating each task when its own evidence is complete; manual tasks must remain open.

**Recommendation:** remove generic syntax lessons and invented examples. Keep concise, repository-specific failure modes with source/test references. Specify outcomes, scope, verification, and stop conditions, allowing the agent to choose routine implementation steps.

### 7. Make verification commands and coverage claims executable — medium priority

`src/lib.rs:1` requires nightly. `BUILD.md` mentions this in the newer GPU section, but the root and most quick commands omit it. The intended WASM target is the library; the active verification report documents failure when the native binary is included.

`coverage_methodology.md` declares branch coverage and a medical-path subset, while `.github/workflows/coverage-gate.yml:49–77` obtains the same undifferentiated text output for medical and overall thresholds. This is static evidence that documentation and implementation need reconciliation, not a claim that CI was run here.

**Recommendation:** one authoritative verification matrix in `doc/agents/BUILD.md`, backed by scripts/CI where practical:

- Native: locked nightly checks and tests, with GPU requirements explicit.
- WASM compile: `cargo +nightly check --locked --lib --target wasm32-unknown-unknown`.
- Browser/native interaction: separate manual/runtime checks; a compile check is not a browser test.
- Formatting/lint: documented commands and current known blockers, without silently weakening checks.
- Coverage: declared metric, file scope, toolchain/tool version, denominator, and machine-readable evidence.

## Suggested target organization

Keep `doc/`; renaming it to `docs/` has no intrinsic model benefit. Consolidate incrementally rather than moving everything at once.

```text
AGENTS.md                         # concise task router + hard repository constraints
<tool-specific adapters>          # only when the selected harness needs them

doc/agents/README.md               # compact reading map, not another policy copy
doc/agents/BUILD.md                # authoritative build/test matrix
doc/agents/CONVENTIONS.md          # non-obvious Rust/platform conventions
doc/architecture/                 # current architecture and durable decisions
doc/rendering/                    # current GPU/encoding contracts
doc/testing/                      # current fixtures, checks, coverage methodology
doc/research/                     # dated investigations, not standing instructions
doc/archive/                      # superseded plans/reviews, with redirects as needed

openspec/config.yaml              # after deliberate OPSX migration
openspec/specs/                    # approved baseline behavioral contracts
openspec/changes/                  # active proposals/deltas/tasks/evidence + archive
```

This is a suggested authority map, not a demand for a new file per topic. Existing useful documents can stay in place and be referenced canonically. Scoped `AGENTS.md` files are optional: add them only when constraints really differ and the chosen harness's loading behavior has been verified. Explicit root routing remains useful for portability. [S1–S3]

## Suggested root AGENTS.md content

Aim for roughly the current scale, with fewer repeated links and more valuable invariants. A 50–100-line entry point is a working target, not an enforced limit.

1. **Scope and authority:** small requested changes; preserve unrelated work; pause for a product/safety decision when approved contracts conflict.
2. **Hard constraints:** native/WASM separation; explicit uniform layout; medical-data precision; synthetic fixtures and no identifiable patient data/secrets in logs or artifacts; no live exposure/device actuation without explicit authorization.
3. **Task routing:** code → conventions/build; GPU → rendering/encoding; domain change → affected specs; new capabilities/breaking changes → selected OpenSpec workflow; documentation-only → no unnecessary proposal scaffolding.
4. **Verification:** short fast commands or a single authoritative build-guide pointer; state what was actually run and distinguish unavailable/manual checks.
5. **Done:** requested behavior implemented, relevant regression checks run, docs/specs updated where behavior changed, residual limitations reported.

Remove repeated onboarding sections, boilerplate language tutorials, copied dependency versions, and universal read-everything instructions. Add safety-specific rules missing from the current root instead of pruning indiscriminately.

## Right-size OpenSpec rather than abandoning it

| Work | Suggested process |
| --- | --- |
| Typo, docs correction, localized bug restoring accepted behavior | Direct edit and focused verification; no feature proposal |
| Small internal refactor with unchanged contracts | Lightweight task plan; use a formal change only for material risk/cross-cutting impact |
| New capability, API/format change, architectural/resource-lifecycle change | Approved OpenSpec change, focused scenarios, relevant design and verification |
| Acquisition/exposure control, medical identity/orientation/precision policy | Explicit owner decision, safety-relevant scenarios and evidence; do not infer permission from a general coding request |

Approve outcomes and consequential design choices, not every routine tool call. Allow findings to revise plans within approved scope; scope or safety-contract changes require renewed approval. Incrementally check completed tasks and keep blocked/manual tasks visible. Retain validation but supplement it with semantic review and tests.

After OPSX migration, keep `context:` small because it is injected into all artifact instructions; put proposal/spec/design/task-specific constraints under matching `rules:`. Do not paste the entire documentation tree into configuration. [S6]

## Recommended sequence

1. **Correctness first:** repair matrix/precision guidance, acquisition spec structure, misleading test contracts, and absent-feature instructions.
2. **Consolidate without moving code:** choose canonical docs, turn the duplicate OpenSpec manual into a pointer, reconcile tool-specific policy, label old plans/measurements.
3. **Upgrade separately:** back up customized legacy files, pin OpenSpec 1.14.0 (or the version reviewed at migration time), migrate on an isolated branch, review generated skills/config and validate existing changes. Do not disrupt the uncommitted rendering work.
4. **Make drift detectable:** link/path checks that distinguish proposed paths, strict OpenSpec validation, requirement-to-test references, and reliable verification/coverage scripts. Resolve baseline failures rather than introducing a permanently ignored gate.
5. **Measure the result:** compare current versus revised guidance on a fixed commit and the same model/harness/settings using representative bug, GPU, WASM, docs-only, spec-planning, and safety-sensitive tasks. Track correct outcomes, unsafe/unapproved actions, tests run, unrelated edits, tokens, and elapsed time. There is no repository-specific evidence yet that shortening alone improves performance.

## Review validation and limits

- Read root/OpenSpec/core agent guides, all baseline specs, selected testing guides/plans, tool adapters, relevant manifests/CI, and source contracts needed to check decisive contradictions.
- Inventoried 72 Markdown files under `doc/`; not every historical design note was semantically audited.
- Verified local CLI version, active changes/spec listing, strict all-item validation, targeted acquisition validation, manual similarity, manifest/source claims, and clean whitespace via `git diff --check`.
- Did **not** run Rust builds, tests, coverage, browser checks, a model evaluation, or any migration. Existing verification reports are prior evidence, not tests performed by this review.
- Working tree already contained uncommitted rendering changes and updated rendering/pitfall guides. They were reviewed as the current state and left untouched.
- Upstream docs/releases were retrieved by the research lane; parent independently rechecked release metadata and the full pinned migration guide. Direct `fetch_content` encountered local fake-IP/SSRF protection; read-only HTTPS retrieval succeeded for these sources. No security configuration was changed.
- No claim is made about a universally best/latest model, universal instruction-loading behavior, clinical readiness, or measured speedup. Recommendations are engineering judgments informed by the evidence below.

## First-party sources

All retrieved **2026-10-01**. Live pages may subsequently change; the OpenSpec migration reference is release-pinned.

- **S1 — OpenAI, Custom instructions with AGENTS.md:** https://developers.openai.com/codex/guides/agents-md/  
  Relevant passages: “How Codex discovers guidance” and “Customize fallback filenames.” Startup walks root to working directory, reads at most one discovered file per directory, and limits combined instructions by `project_doc_max_bytes` (“32 KiB by default”). Nested reference files are not automatically loaded merely because they exist.
- **S2 — Anthropic, How Claude remembers your project:** https://code.claude.com/docs/en/memory.md  
  Relevant passages: “Write effective instructions,” “When to add to CLAUDE.md,” and “Path-specific rules.” Exact guidance: “target under 200 lines per CLAUDE.md file”; “Imports help you organize a long file but don't reduce its context cost.” Current direct AGENTS.md support is conditional and version-dependent; check the runner before adding a duplicate adapter.
- **S3 — Anthropic, Extend Claude Code:** https://code.claude.com/docs/en/features-overview.md  
  Relevant passages: “Understand context costs” and “CLAUDE.md vs Skill.” Persistent instructions and on-demand skills have different loading costs. These are official recommendations, not a controlled benchmark of this repository.
- **S4 — Fission AI, OpenSpec v1.14.0 release:** https://github.com/Fission-AI/OpenSpec/releases/tag/v1.14.0  
  Endpoint provenance: https://api.github.com/repos/Fission-AI/OpenSpec/releases/latest returned `tag_name=v1.14.0`, `draft=false`, `prerelease=false`, `published_at=2026-09-30T22:52:17Z`. Release notes warn: “Cleanup now says the listed files are deleted entirely and asks you to back up custom content first.”
- **S5 — OpenSpec npm metadata:** https://registry.npmjs.org/@fission-ai%2Fopenspec  
  Research retrieval: `dist-tags.latest=1.14.0`, publication `2026-09-30T22:54:22.058Z`, `engines.node=>=20.19.0`.
- **S6 — OpenSpec, Migrating to OPSX, pinned v1.14.0:** https://raw.githubusercontent.com/Fission-AI/OpenSpec/v1.14.0/docs/migration-guide.md  
  Relevant passages: “What Gets Removed,” “Migrating project.md to config.yaml,” “The New Commands,” and “The New Config System.” Describes skills, profile-dependent actions, preservation/migration boundaries, concise injected context, and per-artifact rules. Upstream reliability rationale is maintainer-reported, not measured here.
