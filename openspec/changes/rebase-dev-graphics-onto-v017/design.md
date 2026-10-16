# Rebase design and execution plan

Status: implementation authorized, 2026-10-05. The following design records the
original planning observations. Current execution evidence is in
[execution.md](execution.md); individual graphics decisions are in
[public-exceptions.md](public-exceptions.md). Backup refs and an isolated candidate
have been created, and baseline builds/checks have run. Replay and destination
update have not begun.

## 1. Exact inputs and observed state

| Role | Ref | Pinned commit |
| --- | --- | --- |
| Source base, excluded from replay | `603bb96` / local `main` | `603bb96bb774f8a61652e9d14985a2b5046b3e75` |
| Source tip | `dev` | `8e308cb3c2e047f530ddbfde690fa45d8e30d1fc` |
| Destination and API baseline | `wgpu-upgrade` / `v0.17.0` | `a39ca528a55be7f7fe565a7fa7ed20b2251826e4` |

The merge base is `603bb96`. The source has 13 unique commits: 12 ordinary
commits and merge `37a027e`. The release has 56 unique commits. The requested
source diff changes 237 files, with 30,078 insertions and 10,280 deletions;
much of that is documentation/tooling. The release changes 175 files from the
same base. Both sides modify 54 paths, including 32 source and three test paths.
Overlap counts forecast review scope, not a measured rebase conflict count.

The main checkout is now on `wgpu-upgrade`; existing untracked Python bytecode
must be left alone. `diamond-inlay` contains an unfinished opposite-direction
merge at `918e36b`, and `moonstone-culet` contains `7adfaa9`. Neither worktree is
an implementation candidate for this request. In particular, do not bring in
the latter's fixed-LOD shader approach or reset/abort the former's work.

All refs and counts above were inspected locally. No fetch or remote validation
was performed; execution must recheck the exact local refs before starting.

## 2. Public compatibility is the governing rule

The user clarified that public APIs must be preserved, while internal APIs may
change. The earlier clarification permits explicit review of low-level graphics
exceptions; it does not authorize breaking an exported API without that review.
Reachability, not the module's name or apparent implementation role, determines
whether an item is public.

Freeze the v0.17.0 interface before replay:

- Rust public modules/re-exports, traits and required methods, enum variants and
  payloads, public struct fields and constructibility, exported macros, functions,
  async/result shapes, serialization, and native/WASM target guards.
- Browser entry points and the seven reference classes, including the five-argument
  `set_render_mode(mode, mesh_index, mpr_index, mip_index, orientation_index)`,
  unified quaternion/rotation controls, and typed-array/Promise results.
- `get_rotation(index)` returning a Promise of a JSON string of four quaternion
  components; split OBJ export returning parallel name/body arrays; segmentation,
  DR overlays, needle/ball controls, and their index/unit/capacity semantics.
- Actual HTTP methods/paths, status/DTO shapes, WebSocket variant names, and the
  existing native CLI. Source uses bare `server`; do not substitute `--server`.
- Local WASM parsing/export behavior. The pending backend-split proposal's planned
  removal of bindings is not the shipped release contract.

Private lifecycle, frame, storage, invalidation, and layout helpers can adapt.
Do not replace the release facade or dispatch enum with dev's older controls.
Do not remove or privatize a reachable item merely because no in-repo caller uses it.

### Existing inventory to reuse and verify

The independent inventory in `diamond-inlay` is pinned to the same release SHA:

- [Declared Rust items](baseline/release-public-api.json):
  1,089 declarations/trait members across 85 of 87 source files.
- [Exports and macro supplement](baseline/release-export-surface.json):
  163 module/export edges, two exported macros, and 27 GLCanvas macro senders.
- [Generated browser reference](baseline/reference-browser-api.d.ts)
  and its [provenance](baseline/api-conflict-inventory.md).

These are read-only references, not candidate conformance. The declaration JSON
includes items hidden behind private modules and needs a reachability audit.
The browser reference was generated previously from a temporary release copy
with the feature gate removed and 15 duration calls switched to existing stable
helpers in four files. Preserve that provenance and regenerate independently
before final acceptance; do not describe it as a fresh unmodified-tag build.
Copy/hash the useful reference evidence into a self-contained candidate so it
does not depend on the lifetime of another unfinished worktree.

### Public graphics exception review — planning inventory

Current implementation decisions and the viewer/browser compatibility boundary
are recorded in [public-exceptions.md](public-exceptions.md). P2/P3 are approved;
P4 preserves the release interfaces. The table below records the original review
questions, rather than a new approval request.

| ID | Public seam at risk | Preferred resolution and review requirement |
| --- | --- | --- |
| P1 | Public arguments/fields containing wgpu 23 or Winit 0.29 types, including Graphics, view traits, factories, and event-loop constructors | Upgrading to wgpu 30/Winit 0.30 changes upstream type identity even when the spelling stays the same. Inventory downstream uses and describe dependency alignment explicitly. Cross-version GPU objects cannot be cast/shared. Approve this boundary before the upgrade checkpoint. |
| P2 | `Renderable::render`, `App::render`, related view/pass results, `KeplerError::Surface`, and `From<wgpu::SurfaceError>` | Dev changes these to Kepler-owned errors because its new backend path no longer supplies the old contract. Determine whether a faithful boundary adapter is possible. Otherwise obtain approval of the exact new signatures/enum payload/conversion behavior; do not call this internal. |
| P3 | Public `PassContext<'a>` | Dev introduces two borrow/encoder lifetimes. Try retaining the published wrapper over a private context; if that is not sound or practical, explicitly review the second lifetime and downstream annotation changes. No unsafe lifetime conversion. |
| P4 | `MprRenderContext::new(device, queue)`, `MultiMeshContext::new(device, queue)`, basic-lighting pipeline helpers, and public swapchain-format helpers | Prefer additive explicit-format constructors/helpers for production paths while retaining the published signatures and legacy helper behavior. Internal viewer resources use owner-specific formats. Any removal or signature change is a public exception, not an automatic cleanup. |

The approved plan must name which, if any, of P1–P4 are allowed to change and
their consumer migration. Until then, no implementation can claim full public
source compatibility. Additive default trait methods are preferable to new
required methods that break downstream `View` implementations.

### Public data interfaces also need adapters

Preserve `VolumeEncoding::HuPackedRg8 { offset: f32 }`, `HuFloat`, the public
decode/content fields, and caller-supplied offset interpretation. Dev's renamed
variants and metadata are implementation evidence, not permission to replace
the release enum. Internal canonical uploads can use exact i32-intermediate
packing with offset 32768 while exposing the existing variant carrying that
offset. Legacy bytes uploaded with offset 1100 must still decode as requested.
Do not silently change the public default constant or add required struct fields
that break caller struct literals/exhaustive matches.

Keep CT storage, R8 continuous DR/scalar data, and R8Uint labels distinct inside
the implementation. Derive/track the distinction privately without changing
the published metadata type unless a separate public exception is approved.
The uint uploader preserves its full byte domain; segmentation label limits are
applied at the segmentation attachment boundary.

The release's WindowLevel fields are public and its upper width/level bounds are
`f32::MAX`. The prior integration's field-privacy/range exceptions are not
inherited here. Keep the release public shape; propose any validation/range
change separately and do not alter medical behavior to satisfy old dev tests.

## 3. Replay map

Use a merge-preserving rebase on a disposable candidate copied from the source
tip. This leaves `dev` unchanged and makes the candidate descend from the release.
Do not rebase the original `dev` branch or merge the release back into it.

| Source commit | Changes to carry | Adaptation/checkpoint |
| --- | --- | --- |
| `685e419` | Exact full-domain signed packing, decode/upload validation | Preserve release enum/facade; port to CT, MPR, MIP, and DVR paths. Distinguish R8 content. |
| `57d4cee` | Lint cleanup, timing/data fixes, assessment docs | Inspect each patch; retain release-only data/export APIs, UID behavior, and new fields. Avoid unrelated reformatting. |
| `d6c044a` | Headless production GPU correctness baseline | Exercise release production shader families; replace old shader consumers without dropping coverage. |
| `1edaf80` | Attachment-format ownership | Adapt MPR/multi-mesh constructors under P4 and test BGRA/RGBA owners. |
| `947b4fc` | Redraw scheduling, shared resources, depth caching | Include release primitive/overlay/segmentation mutations and animation in invalidation; checkpoint resource/redraw behavior. |
| `6136c46` | Agent/docs consolidation, OpenSpec/npm maintenance | Carry the tooling with release backend/features/specs intact. Preserve package identity/version. Most file churn is here. |
| `b254673` | Browser WebGPU preference, pre-canvas GL fallback | Preserve release controls and loading. Extend capabilities to all actual content formats. |
| `34bc2aa` | Stable timing helpers, Rust 1.98.0 pin and CI | Carry on its recreated sibling branch. Replace all release unstable timing calls, not only the old source call sites. |
| `37a027e` | Merge the browser/toolchain branches | Recreate topology and review merge resolution. Its four post-merge verification lines remain source history, not candidate passes. |
| `6ff5a0e` | wgpu 30.0.1/Winit 0.30.13, frame/recovery/handler migration | Main checkpoint: P1–P3, release event dispatch, shaders, descriptors, native compute, native/WASM types, generated bindings. |
| `f42fab2` | Transactional layout construction and failure preservation | Implement against release layout/mesh/oblique semantics and five-argument facade. Preserve release APIs with private preparation/commit helpers. |
| `918e36b` | Lifecycle dispatch and complete GPU rebinding | Extend rebinding to every release resource: labels, overlays, DVR, multi-mesh, primitive buffers, camera and CPU state. |
| `8e308cb` | Additional source verification evidence | Carry as historical documentation; record candidate verification separately. |

Do not silently omit any commit as “only docs” or “already applied.” Record empty,
split, adapted, or superseded patches individually. If default rebase handling
drops a patch, stop and account for it rather than continue unattended.

## 4. Expected difficult resolutions

### Removed/replaced renderer paths

Three source-modified paths were actually deleted on the release history:
`shader_tex.wgsl`, `mesh/camera.rs`, and `mesh/mesh_processing.rs`. Transfer the
needed corrections to `mpr.wgsl`, `volume.wgsl`, the shared camera, and release
mesh/segmentation code. Do not resurrect retired APIs/assets by taking a whole
dev file. The release production pipeline, rather than an orphaned old shader,
must be the regression target.

MPR currently returns on varying bounds before implicit texture sampling. Port
the uniform-derivative correction while preserving aliasing, labels, primitive
overlays, and multi-oblique behavior. MIP/DVR ray loops need production browser
validation, too. Fixed LOD zero is not an equivalent minification correction;
the source footprint approach also avoids its recorded Metal 3D-gradient issue.
Review each actual sampler: release R16Float and packed paths do not have identical
minification filters. A change to filter policy or window/level math must be
identified as a behavioral decision, not hidden in a shader conflict resolution.

Rust/WGSL structs and bindings must be ported together, with size/offset/alignment
and actual pipeline tests. Preserve signed values, decode/display separation,
coordinate conventions, opacity/threshold units, and result capacities.

### Dependency and platform boundary

Reconcile Cargo.toml first; preserve release server/ORT/Modbus dependencies behind
native guards. Use source versions 30.0.1 and 0.30.13 rather than an unreviewed
“latest” upgrade. Keep the native `wgpu30` source alias where required by public
compute paths, resolving it to the same wgpu package version as the viewer.
Compute and viewer devices remain independent owners even when types unify.
Inspect exact locked resolution and native/WASM feature trees; no wholesale lockfile
replacement or blanket dependency update.

Keep the native display-handle fix, browser/headless display-free initialization,
Queue presentation/error-scope ownership, bounded recovery, and fresh-page GL
diagnosis. Preserve GPU-ready asynchronous construction, pre-run queued controls,
the proxy handle, and browser `spawn_app` returning to JavaScript.

### Application and server survival

Release-only UserEvent variants and replies must survive the new handler/lifecycle
owner. Audit dispatch exhaustively, including OBJ exports, segmentation, DR,
quaternion getters, and primitive updates. Every visual mutation invalidates the
correct owner; getters/no-ops remain idle. Layout failure must preserve all affected
release state rather than clear views before construction succeeds.

The dev source has no release server module, so copying its lib.rs/main.rs wholesale
would lose native exports/startup. Preserve the actual router/DTOs and pages;
compare against executable routes, not unfinished backend proposal checkmarks.
Use mocked/offline handler tests. Do not send navigation, preparation, exposure,
or acquisition commands or launch a device-connected server.

### Medical/test and documentation discrepancies

Record known source-level release contradictions before treating them as rebase
regressions: `generate_uid()` uses `2.25.<uuid>` while export tests assert `1.2.`,
and the checker fixture uses ten-voxel cells while assertions expect one-voxel
parity. Reproduce under the baseline and correct tests only against an independently
agreed contract. Do not change UID generation, signed voxels, geometry, display
ranges, or tolerances just to pass inherited expectations.

The release's rendering spec still describes dual orthogonal MPR while later
implementation uses multi-oblique planes. Keep this discrepancy visible; do not
silently rewrite the accepted baseline during the docs replay.

Preserve Cargo package version 0.17.0 and existing npm version 0.16.0 during
replay; a separate version-policy decision can reconcile the existing mismatch.
Do not replace release manifests with dev's older package metadata. Do not restore
deleted opaque image assets/compile-time loaders. Use deterministic synthetic
fixtures if a transferred test/loader needs data.

## 5. Execution phases and stop gates

### Phase A — freeze the baseline and approve public exceptions

1. Recheck refs/worktree state and hash source/API evidence. Preserve all existing
   worktrees and untracked files. Create backup refs for the source/destination.
2. Capture reachable Rust API and actual generated browser declarations. Build
   compile fixtures for public domain/facade users, exported macros, and custom views.
3. Capture actual HTTP/WebSocket/CLI contracts and baseline native/WASM/GPU outcomes.
   The tag has an unstable duration feature gate and no toolchain pin; report that
   baseline honestly. Any timing-only reference patch needs recorded provenance.
4. Review P1–P4 individually. Unapproved public breaks stop the relevant phase.

### Phase B — isolated merge-preserving replay

Example commands, not executed; the new names must first be checked for collisions:

```sh
rtk git branch codex/backup-dev-graphics-20261005 8e308cb3c2e047f530ddbfde690fa45d8e30d1fc
rtk git branch codex/backup-v017-wgpu-20261005 a39ca528a55be7f7fe565a7fa7ed20b2251826e4
rtk git worktree add -b codex/rebase-v017-wgpu ../kepler2-wgpu-v017-rebase 8e308cb3c2e047f530ddbfde690fa45d8e30d1fc
```

From the isolated checkout:

```sh
rtk git rebase -i --rebase-merges --onto a39ca528a55be7f7fe565a7fa7ed20b2251826e4 603bb96bb774f8a61652e9d14985a2b5046b3e75
```

Edit the generated todo without altering its recreated branch labels/resets.
Insert deliberate stops after the resource foundation, recreated browser/toolchain
merge, graphics upgrade, layout change, and lifecycle change. Resolve conflicts
semantically and stage only reviewed paths; never use blanket ours/theirs.
Document old/new commit mapping and any necessary per-patch adaptations.
Stop on unapproved public changes, ambiguous data semantics, unexplained dropped
features/tests, missing native dependencies, or new compile/GPU/API failures.

### Phase C — checkpoint verification

At the resource foundation, test encoding, format ownership, production pipelines,
invalidation, and depth bounds. Before the toolchain change, use the recorded
baseline compiler situation rather than assume stable can build every intermediate
commit. After the recreated toolchain/browser merge, native and WASM library
compilation must pass with the selected stable pin before continuing to wgpu 30.

At the graphics upgrade, require native/WASM types and browser declaration/public
compile checks before proceeding. Then test every production rendering family
and lifecycle dispatch. Finally test layout failure preservation and rebinding
with all release feature state, including separate devices and format changes.
A Rust/WASM compile alone is insufficient for any runtime checkpoint.

### Phase D — final acceptance and branch update

Compare the candidate against both source and destination, using range-diff for
ordinary commits and a separate review of the recreated merge resolutions:

```sh
rtk git range-diff 603bb96..8e308cb a39ca52..codex/rebase-v017-wgpu
rtk git diff --stat a39ca52..codex/rebase-v017-wgpu
```

Require an empty unapproved public API diff and tests of caller behavior. Review
release-only feature survival and every source patch; do not use conflict-marker
absence as correctness evidence. Keep baseline failures and unperformed manual
checks visible. An outstanding required check remains outstanding unless its
specific acceptance/deferment is explicitly agreed.

Only after implementation authorization and accepted gates, update `wgpu-upgrade`
with `git merge --ff-only codex/rebase-v017-wgpu`. Recheck that the destination has
not moved and preserve unrelated worktree content. If it moved, rebuild/reverify
the candidate against the new destination; never force it over another change.
Leave `dev`, tags, and other worktrees unchanged. No push/force-push is part of this
plan. During replay, rollback is `git rebase --abort` in the candidate only;
before landing the original branches already remain the safe rollback points.

## 6. Candidate verification matrix

After the source toolchain/docs maintenance has been adapted, run in the candidate:

```sh
rtk rustup show active-toolchain
rtk rustc -Vv
rtk cargo check --locked --all-targets
rtk cargo check --locked --lib --target wasm32-unknown-unknown
KEPLER_REQUIRE_GPU_TESTS=1 rtk cargo test --locked
KEPLER_REQUIRE_GPU_TESTS=1 rtk cargo test --locked --test rendering_correctness_tests --test volume_encoding_tests --test render_resource_lifetime_tests --test redraw_state_tests
rtk cargo fmt -- --check
rtk cargo clippy --locked --all-targets -- -D warnings
rtk wasm-pack build --target web --out-dir pkg --release -- --locked
rtk npm run docs:check
rtk python3.11 -m unittest discover -s scripts/tests -p 'test_*.py'
rtk npm run specs:check
rtk git diff --check
```

Also run the new public API/compile, dispatch/reply, HTTP/DTO serialization,
uniform-layout, DVR/multi-mesh/overlay, and compute-owner tests. Inspect native
and WASM dependency trees for unexpected old graphics versions and native leakage.
Never run every ignored test indiscriminately: the large DICOM export writes to
a hard-coded path and has substantial memory cost.

Manual acceptance uses synthetic volumes in native and actual browser pages:
MPR/MIP/MinIP/AvgIP/DVR, oblique controls, labels, OBJ import/export, DR/primitive
overlays, minification/magnification, resize, idle, hide/restore, canvas replacement,
pre-run events, getter replies, WebGPU and fresh-page WebGL2. Check Linux/Wayland
GLES separately; macOS offscreen results do not establish it. Record unavailable
ORT/model/platform environments rather than claim they passed.

Source records include baseline formatting/Clippy failures and unfinished browser
acceptance. Rerun and compare on the exact release/candidate; do not transplant
source failure counts or old pass counts into this acceptance report.

## 7. Planning validation

Planning uses source/history/API inspection, not a candidate rehearsal. No exact
rebase conflict count, native/WASM build result, GPU result, or API conformance
pass is claimed. This proposal must pass the pinned OpenSpec syntax validator
before it is presented as ready for review. The release lacks an npm `openspec`
script; validate through a temporary npm runner using the already installed
project-local CLI 1.14.0 without altering the release package.json.

Planning checks completed on 2026-10-05:

- `npm --prefix /private/tmp/kepler-v017-rebase-plan-cli-01a109f5 run openspec -- validate rebase-dev-graphics-onto-v017 --strict --no-interactive`: exit 0, valid.
- The same pinned runner listed current changes and baseline specs successfully.
- Local link/trailing-whitespace/task inspection: six Markdown files, three delta
  specs, six requirements, 13 scenarios, six valid local links, zero whitespace
  issues, and 36 open/zero completed execution tasks.
- `git diff --check`: exit 0 for tracked changes; untracked proposal whitespace
  was checked separately by the inspection above.
- Final ref check: dev remains `8e308cb`; destination/tag remain `a39ca52`.
  Native/WASM builds, GPU tests, runtime checks, and a rebase rehearsal were not run.
