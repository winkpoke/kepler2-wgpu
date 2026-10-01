# OpenSpec workflow

**Status:** Current repository policy for OpenSpec **1.14.0**, pinned in
`package.json` and `package-lock.json`. Baseline specs and active/history directories
remain in `openspec/`. Project context/rules are in
[config.yaml](../../openspec/config.yaml); the old trigger manual is retired.

## Tooling and task selection

```bash
npm ci --ignore-scripts
npm run openspec -- list
npm run openspec -- list --specs
npm run openspec -- show <item>
npm run openspec -- status --change <change-id>
npm run openspec -- instructions <artifact> --change <change-id>
npm run specs:check
npm run docs:check
```

Use the local npm command even when generated skill examples say plain `openspec`.
The global CLI may still be legacy. For generated-command maintenance, use the
local CLI's `init`/`update` help, inspect the resulting diff, and preserve custom
content before allowing cleanup. Do not edit generated skills to copy policy.

The core profile provides propose, explore, apply, update, sync, and archive.
For Pi/OpenCode/Cline this checkout generates `/opsx-propose`, `/opsx-explore`,
`/opsx-apply`, `/opsx-update`, `/opsx-sync`, and `/opsx-archive`. Codex CLI/IDE
uses `$openspec-propose` and the corresponding skills; Codex desktop selects
skills through its UI. These are agent commands, not shell CLI commands.
Restart/reload the relevant agent to discover newly generated skills/prompts.

## Right-size the process

| Work | Process |
| --- | --- |
| Docs correction or localized fix restoring accepted behavior | Direct edit and focused verification |
| Small behavior-preserving internal refactor | Lightweight task plan; formal change only for material risk |
| New capability, breaking API/format, material architecture/performance/security change | Approved OpenSpec change with relevant scenarios/design/checks |
| Medical identity/orientation/precision or live acquisition safety policy | Explicit owner decision and safety-relevant verification |

Inspect affected baseline specs and overlapping active changes before drafting a
significant change. A request to explore/propose is not permission to implement.
Obtain approval of scope and consequential design; routine tool steps need not be
micromanaged. New scope or contract/safety changes require renewed approval.

## Artifacts and deltas

- `proposal.md`: why, scope/non-goals, impact, and acceptance.
- `design.md`: consequential ownership, compatibility, alternatives, and rollback.
- `tasks.md`: checkable implementation/verification outcomes. Update each task when
  its own evidence is complete; manual checks stay separate and open until performed.
- `changes/<id>/specs/<capability>/spec.md`: deltas against the baseline requirement names.
- `specs/<capability>/spec.md`: approved intended behavior, not measured completion.
- `changes/archive/`: completed change history; do not reinterpret it as current policy.

Use `## ADDED|MODIFIED|REMOVED|RENAMED Requirements`, `### Requirement: Name`,
and at least one `#### Scenario: Name` per requirement. Use normative SHALL/MUST
for observable behavior. MODIFIED replaces the entire requirement: include all
unchanged scenarios as well as edits. A missing target is not a valid MODIFIED;
use ADDED for a genuinely new requirement. Renames use FROM/TO and a behavior
change under the new name. Keep units, input ranges, tolerances, and real test
references explicit. Test counts alone are not acceptance evidence.

Validate syntax **and** pending identities with the two check commands above.
They cannot prove medical semantics, complete test coverage, or clinical readiness.
Read implementation/tests and report contradictions before changing contracts.

## Sync, archive, and maintenance

Sync only reviewed, implemented behavior into baseline specs. Review the spec diff
and rerun validation after sync. Archive when the agreed completion/deployment
criteria are met; leave blocked browser/device/deployment work visible. Never
blanket-check tasks to make archive succeed. Tooling-only changes may use the
CLI's explicit `--skip-specs` archive option; this does not excuse unperformed checks.

Custom policy belongs in this guide, [AGENTS.md](../../AGENTS.md), and compact
config context/artifact rules/operation guidance. Generated integrations are
mechanical outputs of the pinned tool. Update them deliberately, not via a global
upgrade. The migration and backup details are recorded in
[the implementation record](../research/2026-10-01-agent-guidance-implementation.md).
