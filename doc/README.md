# Documentation map

## Current reference

- [Agent entry point](../AGENTS.md): repository policy and conditional routing.
- [Architecture](architecture/ARCHITECTURE.md): current ownership and subsystem map.
- [Conventions](agents/CONVENTIONS.md): non-obvious Rust/platform conventions.
- [Build and verification](agents/BUILD.md): authoritative executable command matrix.
- [Rendering](agents/RENDERING.md) and [voxel encoding](rendering/volume-encoding.md): GPU contracts.
- [Testing](testing/README.md): fixtures, regressions, coverage, and evidence limits.
- [OpenSpec workflow](agents/OPENSPEC.md): approval, local tooling, and spec maintenance.
- [Remedy protocol](acquisition/remedy-serial-web.md): acquisition reference; offline tests by default.

## Authority and history

- [Baseline specs](../openspec/specs/) define approved intended behavior.
- [Active changes](../openspec/changes/) own proposals, deltas, tasks, and verification.
  They are not baseline behavior until reviewed and synced.
- [Research](research/) contains dated investigations. Its measurements and dependency
  recommendations apply to the recorded revision/environment, not automatically to HEAD.
- [Archived guidance](archive/agent-guidance-2026-10-01/) preserves superseded testing
  plans and examples. It is not executable instruction or evidence of current coverage.
- `redering/`, dated view notes, and older architecture analyses are historical material.
  Existing paths are retained for backlinks; do not treat old plans as current ownership.
- [Changelog](CHANGELOG.md) records user-visible changes.

Maintain each policy in one current reference. Old agent-guide paths may remain as
short redirects. New reference pages should be linked from this map or a task guide;
add genuinely current guide paths to `scripts/check_docs.py` when extending its scope.

Run `npm run docs:check` after changing current links or pending spec deltas. The
checker validates local inline link targets and delta identities, not remote URLs,
anchors, clinical correctness, or every historical document.
