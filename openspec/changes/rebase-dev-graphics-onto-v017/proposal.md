# Rebase dev graphics changes onto v0.17.0

Status: implementation authorized by the user's follow-up request on 2026-10-05.
Baseline preparation is complete; the individual public graphics decisions remain
pending. See [execution.md](execution.md) and [public-exceptions.md](public-exceptions.md).

## Why

`wgpu-upgrade` currently points to the v0.17.0 release. The graphics fixes,
wgpu/Winit upgrade, and lifecycle improvements on `dev` must be replayed onto
that release without replacing its public interfaces or dropping its features.
The histories overlap in application, rendering, data, dependencies, and docs,
so an unattended rebase is insufficient.

## What Changes

- Replay `603bb96..8e308cb` on an isolated candidate based on
  `a39ca52`, preserving the source merge topology and recording every resolution.
- Use v0.17.0 as the public API authority: exported Rust paths, fields, traits,
  macros, browser bindings, HTTP routes, WebSocket messages, and caller behavior.
- Port the source graphics corrections into the release's actual MPR, DVR,
  multi-mesh, segmentation, primitive, and overlay implementations.
- Carry toolchain, CI, documentation, tests, scheduling, transactional layout,
  and GPU rebinding changes with the relevant release adaptations.
- Prefer additive/private adapters. Identify unavoidable public graphics changes
  individually; none is approved by this proposal or by the request to plan.
- Validate the candidate independently. Source-branch verification remains
  historical evidence and does not establish candidate acceptance.

## Impact

- Affected specs: `application`, `rendering`, `testing`.
- Affected implementation: application/lifecycle, production renderers/shaders,
  data upload, dependency manifests, native compute integration, tests, and docs.
- Protected interfaces: all externally reachable v0.17.0 APIs; internal modules
  may change when they do not alter those interfaces.
- Protected work: `dev`, the v0.17.0 tag, other worktrees, and unrelated files.

See [design.md](design.md) for the commit map, public exception review, phases,
verification gates, and landing procedure, and [tasks.md](tasks.md) for execution.

## Approval boundary

The follow-up request authorizes applying this design. Its requirement to review
public graphics exceptions individually still applies: P1, P2, and P3 have been
presented for decision, and P4 will use additive interfaces. Earlier exception
decisions recorded in the opposite-direction integration are not carried into
this implementation automatically.
