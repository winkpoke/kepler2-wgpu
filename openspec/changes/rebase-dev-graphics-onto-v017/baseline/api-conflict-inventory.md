# Pinned API and Conflict Inventory

Source: `a39ca528a55be7f7fe565a7fa7ed20b2251826e4` (`v0.17.0`). Target: local dev `918e36b6d0d4ea1c25a017cd2ab061f86d6ead47`. Initially inspected 2026-10-04; reference/export review 2026-10-05. This is a migration inventory, not completed implementation or acceptance.

[Machine-generated declarations](release-public-api.json) contain 1,089 declared public items/trait members across 85 of the release's 87 Rust source files. The [export/macro supplement](release-export-surface.json) records 163 source module/export edges, both exported macros, four generated DICOM types with 29 public fields and their constructors/formatters, and 27 generated GLCanvas senders matched against actual browser declarations. The two files without directly captured public declarations are the native binary entry point and documentation-only shader module. These artifacts are inventory evidence, not integrated API conformance.

### Reachability and target review

- Root re-exports expose `GLCanvas`, `RenderApp`, `App`, `Renderable`, `StaticLayout`, `View`, and the mesh module. `CTVolume` is available through public `data`/`ct_volume` paths; the root's plain `use CTVolume` is not a public root alias.
- Private `view`, `view_factory`, `renderable`, and `layout` implementation modules promote their public items through `rendering::view` globs. `rendering::mesh` and root `mesh` are promoted public module paths. Retired mesh-processing modules are not preserved as compatibility paths.
- Private `core::coord::base` promotes `Base` through its glob. Private DICOM implementation modules promote `Patient`, `StudySet`, `ImageSeries`, `CTImage`, `DicomRepo`, and export items through `data::dicom`; `dicom_helper::get_value` remains hidden by its module, despite its declared `pub`.
- Root `server`/`gpu` and GPU implementations/exports are native-guarded. Private `server::ws` promotes only `WsMessage`, not every declared public command/helper found there. Private GPU implementations promote the context/state/volume types and four compute/upload functions.
- `define_dicom_struct!` and `impl_user_event_senders_for_glcanvas!` are exported at the crate root. DICOM macro expansion adds public fields, `new`, `format_tags`, and Debug/Clone/Serialize; commented WASM getters/setters are not exports. Sender expansion adds an index followed by the specified arguments and returns unit; it is not an error-returning facade.
- Evaluated browser exposure comes from the generated declaration artifact below, not nearby source attributes. Source-edge attributes in the supplement remain contextual: they are not a separately compiled proof of every feature/target combination. Candidate native/API/dispatch checks remain tasks 5.1/5.2 and 9.1/9.3.

## Generated browser reference

The tag tracks no `.ts`, `.d.ts`, or `.js` binding files. An unmodified source/manifest/lockfile extraction was built in `/tmp/kepler-v017-release-bindings-src` with:

```sh
RUSTUP_TOOLCHAIN=1.98.0 \
CARGO_TARGET_DIR=/Users/philchen/Projects/2026/kepler2-wgpu/target \
wasm-pack build --target web --dev \
  --out-dir /tmp/kepler-v017-release-bindings -- --locked
```

Initial attempt: exit **1**, `E0554` at the release's `src/lib.rs:1`, `#![feature(duration_millis_float)]`, rejected by the preserved stable compiler; eight warnings. Temporary log: `/tmp/kepler-v017-release-bindings.log`.

The owner then approved removing only that gate in the temporary reference copy. That exact patch was verified against every extracted source/manifest/lockfile: only `src/lib.rs` differs. The same build again exited **1**, now with **15 E0658 errors** on actual unstable duration calls (13 f64 calls in `render_pass.rs`, one f32 call each in `mesh_view.rs` and `performance.rs`), plus eight warnings. Log: `/tmp/kepler-v017-release-bindings-patched.log`. No declarations were generated; the tag, repository compiler, and candidate dependency selection remain unchanged.

The owner then approved all 15 explicit calls to the release's existing stable `DurationExt` helpers in the reference copy. An intermediate AST-only rewrite covered five calls and still failed on the ten calls inside formatting macros; those were then corrected explicitly. The final identical stable/locked command exited **0** and generated the reference package. Log: `/tmp/kepler-v017-release-bindings-stable-timing.log`.

A byte-aware comparison of every extracted source/manifest/lockfile against `MERGE_HEAD` verified exactly four changed files: the gate removal and 15 helper substitutions, with no helper-body, public-signature, dependency, tag, or compiler change. This old-stack reference package remains under `/tmp`; no runtime package is included in the candidate.

[Reference declarations](reference-browser-api.d.ts) preserve the generated signature body, with provenance comments and an explicit ESNext disposable-library reference for its generated `Symbol.dispose` declarations. The unannotated generated file's SHA-256 is `47b8224da71e8d80b47d9726100e908367d84f76d286d36e5afba3e566055b4e`. It exports seven classes: CTVolume, DicomObject, DicomRepo, GLCanvas, Graphics, RemedyWasm, RenderApp. GLCanvas has 52 named declarations, including generated `free`, plus computed `Symbol.dispose`; all 27 source macro senders are present. The five-argument facade, Promise/string quaternion getter, typed-array replies, segmentation/OBJ controls, and entry points are now grounded in actual generated declarations.

Task 1.4 delivers the reference/source/consumer/conflict inventory, not completed migration. Candidate declaration/API assertions remain required by tasks 5.1 and 9.3, and packaging is not browser shader/runtime acceptance.

## Selected public interfaces

| Boundary | Pinned source contract | Migration decision / evidence still required |
| --- | --- | --- |
| Entry points | `get_render_app()`, `drop_render_app`, `RenderApp::new`, async `set_window`/`run`, `get_glcanvas` | Keep release caller shapes and GPU-ready construction; preserve ApplicationHandler/lifecycle internals. |
| Layout facade/events | `set_render_mode(mode, mesh_index, mpr_index, mip_index, orientation_index)`; matching `UserEvent::SetRenderMode` | Adopt five arguments; no superseded dev overload. Orientation selection uses Transverse, Coronal, Sagittal, Oblique. |
| Unified rotation | `set_rotation_angle_degrees(index, roll, yaw, pitch)`, `set_rotation_degrees(index, x, y)`, `set_rotation_quat(index, Vec<f32>)`, `set_oblique_rotation_radians` | Preserve degree/radian distinction, view-index targeting, and quaternion order. |
| Browser rotation getter | `get_rotation(index) -> Result<String, String>` carrying JSON of four quaternion floats | Resolved the sender conflict to `GetRotation`, preserving the release JSON shape rather than the former matrix getter. Application dispatch/reply tests remain pending. |
| Other browser replies | `get_base_screen`, `get_window_level`, `get_pan`, screen/world conversion, oblique normal, click and pixel-value replies | Preserve array lengths/order and result shape; compare actual generated declarations, not guessed ABI types. |
| Segmentation/OBJ | `apply_segmentation`, eight-entry visibility mask, `set_obj_mesh(raw,id,kind)`, async current/split OBJ export | Split export packs parallel name/body arrays. Test independent standalone indices and label identities. |
| Primitive controls | New needle trajectory/position/radius/angle, float needle-enabled parameter, ball-enabled and indexed ball controls, oblique-to-3D sync | Preserve source units/coordinates and capacities: 32 needles, four ball/plane slots, labels 0–7. |
| Factories | Mesh construction now takes `&CTVolume`; content-based mesh construction and shared MPR/MIP constructors remain | Migrate Rust mocks/callers. Preserve release signatures while retaining private fallible preparation/resource ownership. |
| Mesh contexts | `MeshRenderContext`, `MultiMeshContext`, `MeshUpload`, `MeshKind`, volume/basic-lighting uniforms | Add matched production layouts and rebuild format/device-bound resources. The incoming context contains fixed/revision behavior requiring resource/update regressions. |
| Mesh data | Label identity on `Mesh`, `import_obj_bytes`, single/combined/split OBJ conversion, segmentation `spine` extraction | Adopt release data/API; migrate retired mesh-processing callers rather than retaining compatibility modules. |
| View traits/state | `View`, `StatefulView`, `ViewState`, `Orientation`, `ALL_ORIENTATIONS`, `Renderable` | Preserve selected interface and documented modern error/pass-lifetime exceptions. Retained private invalidation/state owners are not caller compatibility shims. |
| Native compute | `GpuContext`, `GpuState`, `GpuVolume`, double and two-stage volume-upload/readback functions | Same dependency, independent headless device owner. Odd-length transfers and bounded readback still need correction/tests. |
| Data/export | Signed `CTVolume` operations and browser MetaImage export; `generate_uid`/DICOM sink interfaces | Release UID generation is `2.25.<uuid>`; old `change_dicom_uid` is removed. Never use display-oriented 0..255 bilinear output as canonical signed CT. |

Full signatures and DTO field declarations are in the JSON snapshot; this table highlights consequential migration seams rather than replacing that snapshot.

### Approved exceptions only

- Retain `PackedI16Rg8`/`R16Float` and exact `u16(i32(voxel)+32768)` storage, not `HuPackedRg8 { offset: 1100 }`/`HuFloat`.
- Retain private validated WindowLevel fields, width `[1,4096]`, level/bias `[-2048,2048]`, finite rejection/clamping, and `clamp((pixel-(level+bias))/width+0.5,0,1)`.
- Use Kepler-owned errors/unit render success rather than removed backend `SurfaceError`, with approved separate borrow/encoder lifetimes.
- The owner additionally approved the content metadata exception below by replying “continue” after its proposal. The approved reference-only timing substitutions preserve the compiler/tooling pin and public signatures; no bootstrap flag, nightly installation, or compiler change was used.

### Approved public metadata decision — 2026-10-05

The next upload migration exposes a conflict not fully specified by the approved API exceptions. Release `RenderContent::from_volume_r8` and `from_labels_r8` both assign `VolumeEncoding::HuFloat` to the public `volume_encoding` field, despite actual R8Unorm/R8Uint storage. The scalar constructor's source comment explicitly calls this a temporary placeholder. Replacing that value with canonical `R16Float` would still mislabel non-CT storage, contrary to design decision 3's distinct content/validation semantics.

The owner approved the proposed narrow additional Rust API exception by replying “continue.” Implemented `ContentEncoding::{Ct(VolumeEncoding), ScalarR8, LabelsR8}` in `src/rendering/view/render_content.rs`, retaining the public field name `volume_encoding` but changing its type. Canonical CT encoding variants/bytes, upload constructor arguments/results, and browser facade signatures stay unchanged. No legacy placeholder/proxy is added. A separate enum avoids mixing one-byte content into the canonical signed-i16 encoder's domain.

The repository field-consumer search finds direct metadata access in RenderContent and its new tests; other upload/decode callers retain constructor/decoder signatures. Complete native/candidate conformance remains pending. The scalar and integer constructors now share checked structural validation but retain distinct format/filtering semantics. `R8Uint` footprint accounting is one byte per voxel.

DR pseudo-colour overlays use the full unsigned-byte domain, unlike AI label tables. An initial constructor-level 0..7 restriction was caught during DR source/changelog review and removed before staging. Regressions now preserve all 256 integer palette values; segmentation-specific label validation remains at the attachment integration boundary (tasks 4.4/7.3), not silently imposed on this generic uploader.

Focused production-source native tests pass on Apple M5/Metal, and a scoped WASM library check passes. Application replacement/state tests and whole production pipeline/browser conformance remain pending, so task 3.3 and dependent rendering migrations stay unchecked. This evidence does not reopen the former reference-declaration blocker.

## Pending attachment-format API decision — 2026-10-05

**Not approved or implemented.** Rendering-context review found another consequential release API conflict, separate from the approved content-kind field change. The pinned `MprRenderContext::new(device, queue)` reads `get_swapchain_format()` with an RGBA default; `MultiMeshContext::new(device, queue)` builds lighting pipelines through the same process-global format. The release `OnceCell` setter deliberately ignores later formats. This cannot supply independent BGRA/RGBA surfaces or format-changing recreation safely. Retained dev factories/resource tests require explicit per-owner formats.

Proposed narrow Rust exception (append the format, retain release names/other arguments):

| Pinned release | Proposed migration |
| --- | --- |
| `MprRenderContext::new(device, queue)` | `new(device, queue, target_format)` |
| `MultiMeshContext::new(device, queue)` | `new(device, queue, target_format)` |
| `create_basic_mesh_pipeline_with_lighting(device, transform_layout, lighting_layout, use_depth, depth_compare)` | Same arguments plus final `target_format`; retain release depth-compare semantics |
| Public `set_swapchain_format` / `get_swapchain_format` | Remove the process-global helpers and migrate consumers; no stale-global or old-signature compatibility wrappers |

`MipRenderContext`, `MeshRenderContext` and the release texture-quad/volume pipeline helpers already receive explicit formats and need no corresponding signature exception. The view-factory contract and browser constructors/control signatures remain unchanged. Queue is needed by MPR's release default uint segmentation upload; reverting to dev's two-argument device/format constructor would lose that dependency and violate release migration direction.

Approval is required before applying this exception. Alternatives retaining process-global formats or guessing RGBA preserve signatures but conflict with resource ownership; they are not implemented workarounds. Proposed verification includes real production MPR/multi-mesh pipelines on BGRA and RGBA configurations, same-device format change and separate-device rebinding, with default labels and CPU-state preservation. No new runtime/pixel evidence was produced by this inspection. Task 4.1 and related acceptance remain open; the trial index still has 31 unresolved paths.

## Actual native router and DTOs

The authority is [routes.rs](../../../src/server/routes.rs), its handlers, [AI DTOs](../../../src/server/ai_model.rs), [server state](../../../src/server/state.rs), and WebSocket serialization, not the incoming unfinished backend proposal.

| Method | Registered path |
| --- | --- |
| GET | `/api/health` |
| POST | `/api/volumes/upload` |
| POST | `/api/upload_needle_params`, `/api/needle_point_offset` |
| GET | `/api/dicom/build/:id` |
| POST | `/api/segment` |
| GET | `/api/segment/progress/:id`, `/api/segment/result/:id`, `/api/segment/result/:id/raw` |
| GET / POST | `/api/nav/config` |
| GET | `/api/nav/health`, `/api/nav/status` |
| POST | `/api/nav/prepared-ct-lookups`, `/api/nav/prepared-cts` |
| POST | `/api/nav/setup-registrations`, `/api/nav/setup-registrations/:id` |
| POST | `/api/nav/raw-acquisitions`, `/api/nav/initial-observations` |
| POST | `/api/nav/setup-registrations-3d` |
| GET | `/api/nav/setup-registrations-3d/:id` |
| GET | `/api/nav/setup-registrations/:id/drr-previews/:frame` |
| GET | `/api/nav/navigation-sessions/:id` |
| POST | `/api/nav/navigation-sessions/:id/completion` |
| POST | `/api/gpu/test`, `/api/gpu/volume/test` |
| GET upgrade | `/ws` |

`/pkg` is a directory service; unmatched paths use the `static` directory service. Startup selects bare `server`; port defaults to 3000 with `KEPLER_SERVER_PORT` parsing. No server was started.

DTO anchors: health `status/loaded_volumes/uptime`; GPU `data`; volume GPU `voxel_count/sample`; segmentation request `series_id/model/backend`, synchronous response `status/task_id`, progress `task_id/value/status/message`, result `task_id/volume/labels/dimensions`. Backend values are `auto`, `onnx`, `python`.

WebSocket `type` values include `parsing_progress`, `export_progress`, `server_status`, `error`, `heartbeat`, `segment_started/progress/complete/failed/cancelled`, and `needle_set`. Raw-processing variants retain their source names (`Slice`, `CircleAccepted`, `Progress`, `CalibrationResult`, `Finished`). Client commands include `segment`, `segment_cancel`, `segment_progress_query`, `LoadSlice`, `ConfirmCircle`, and `RunCalibration`; JSON field/variant declarations remain in the snapshot.

HTTP cancellation DTOs exist, but `/api/segment/cancel` is not registered; cancellation is wired via WebSocket. Neither `/api/dicom/upload`, `/api/volumes/:id/stream`, nor `/api/export/dicom/:id` is registered. `static/index_image.html` also calls `/api/run-recon`, which is not in this router; its frontend disposition remains to be reconciled, not silently implemented as a new endpoint.

## Consumers, tests, and clean-merge hazards

- `static/index.html` and `static/index_image.html` already call the five-argument facade, quaternion getter/setter, and unified rotation. The main page uses segmentation/raw-result/OBJ and needle controls. `static/navcomputer.html` uses the navigation routes above. The reference binding inventory is captured; candidate binding/example smoke checks are still pending.
- The clean merge of `src/core/window_level.rs` made its fields public and widened upper width/level limits to `f32::MAX`. Corrected this **approved retained medical contract** violation and added independently fixed-bound/non-finite/midpoint regressions. Direct caller migration and actual GPU midpoint tests remain pending; do not treat the focused CPU tests as full acceptance.
- Factory and application uploads must keep canonical packing/decode separation. New R8 scalar and categorical label textures need separate validation, not the two-byte CT validator unchanged.
- Incoming export tests assert `1.2.` against the release's `2.25.` generator; their ten-voxel checker helper disagrees with one-voxel assertions. Fix the contradictions independently and retain signed/relationship/geometry checks.
- Existing encoding/rendering/resource/redraw/lifecycle tests remain regression anchors. Update their shader consumers to actual production `mpr.wgsl`, `volume.wgsl`, and common/mesh paths; validating only retired `shader_tex.wgsl` does not cover imported features.
- The imported rendering baseline proposes dual orthogonal MPR, while later tag code implements multi-oblique. Keep the approved delta authoritative and baseline synchronization separately gated.
- Imported coverage/backend task checkmarks and historical archives are source history, not verification of this candidate. Review them without marking earlier obligations complete.

## Conflict and deletion ledger

The trial merge began with **37 content conflicts and three dev-modified/release-deleted conflicts** (40 total). Original paths/index stages and both-parent statistics are recorded in `/tmp/kepler-v017-trial-{conflicts,unmerged,index-paths}.txt` and `/tmp/kepler-v017-trial-vs-{dev,release}.stat`. Statistics include unresolved marker text and are not final integration sizes.

| Conflict family | Resolution authority |
| --- | --- |
| Manifest/lockfile | One modern dependency graph and native guards; resolved/staged. |
| Acquisition module | Retain existing Remedy plus incoming process module; resolved/staged. Offline verification remains required. |
| Application/facade/layout | Release public controls on retained private lifecycle/frame/transactional owners. |
| Data/DICOM | Release APIs plus approved signed/display/geometry contracts; contradictory tests are not medical authority. |
| Pipelines/MPR/MIP/mesh/factories | Release content/feature interfaces on modern descriptors, shared signed decode, and device/format-owned resources. |
| Guidance/architecture/changelog | Retain routed current policy; merge verified feature documentation, not obsolete dual-stack instructions. |
| Rendering baseline | Preserve unsynced identity; reconcile proposed requirements explicitly, not auto-sync unverified behavior. |

Modify/delete paths: `src/rendering/shaders/shader_tex.wgsl`, `src/rendering/view/mesh/camera.rs`, and `src/rendering/view/mesh/mesh_processing.rs`. Their replacement/consumer/test decisions remain open under tasks 4.1/8.3; no blanket deletion or old-interface shim was selected.

Clean deletion review:

- Restore `scripts/coverage.sh`, `scripts/check_coverage_regression.sh`, the patient-position page, the dated factory-extraction page, and all three `refactor-mip-view-glam` artifacts unchanged from dev. These are unrelated tooling/history/open obligations; deletion is not evidence of completion. Stale documentation details still need review in tasks 3.6/8.2.
- Accept removal of opaque image assets, but replace the retained compile-time loader deterministically before acceptance. The tag itself does not expose `from_file_at_compile_time`; task 3.5 explicitly preserves the selected retained loader boundary. Do not restore patient binaries.
- Review cleanly removed obsolete mesh/GLSL shaders and disabled property-test files together with production replacement coverage. This is not yet final deletion acceptance.
- Removed the two incoming tracked Python bytecode files after verifying their working-tree hashes matched the pinned release; no user modification was discarded. Keep Python sources and model-directory placeholder. No model/browser package was added to the candidate.
- Retain dev-only Rust/CI/OpenSpec/npm files and earlier graphics/layout/lifecycle changes. They are absent in the tag but were not release deletions from the common ancestor; do not confuse a direct parent comparison with the actual merge deletion set.

## Still open

Complete migrated consumer/API assertions; remaining source/spec/doc resolutions; native/WASM candidate compilation and generated declaration comparison; production GPU regressions; model/platform/manual/security acceptance. The reference/export/macro inventory completes the inventory outcome only and waives no implementation or acceptance gate. The public R8 metadata decision above is approved and implemented; its application/production-pipeline acceptance remains pending.
