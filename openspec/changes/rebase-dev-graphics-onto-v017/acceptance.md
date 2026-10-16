# Acceptance and landing — 2026-10-05

The final question identified two unavailable checks: Linux/Wayland GLES and
actual OS hide/suspend/browser visibility transitions. It asked whether to defer
those checks and fast-forward `wgpu-upgrade`. The user replied:

> yes please continue and check the progress with task.md

This approves the named deferments and landing. It does not convert either
unperformed check into a pass. The unchecked D1/D2 entries in [tasks.md](tasks.md)
preserve the remaining platform work; baseline test/quality failures remain visible.

Rechecked destination `a39ca528a55be7f7fe565a7fa7ed20b2251826e4`, a clean tracked
working tree, source dev, and candidate `41dff9f7cb630700ddb5929edcc369aa1cd8ab73`.
Preserved the original untracked proposal at
`/private/tmp/kepler-v017-prelanding-20261005/original-proposal` before installing
its reviewed tracked replacement. Eight unrelated Python files remained in place
and their SHA-256 hashes were checked after landing.

`git merge --ff-only codex/rebase-v017-wgpu` fast-forwarded the main checkout's
`wgpu-upgrade` branch to that candidate. All 13 mapped parent relationships,
including the two-parent merge, are preserved. No force update or push was run.
Dev, release tag and backup refs retain their original commits. A documentation
commit follows to record accepted gates and completed landing tasks; it changes
no production code or validated test expectations.
