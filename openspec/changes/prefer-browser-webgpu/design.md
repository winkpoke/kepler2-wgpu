# Design

## Context

See [proposal.md](proposal.md) for motivation and approved backend-only scope. The lockfile resolves wgpu 23.0.1 and winit 0.29.15. wgpu's locally installed `src/util/init.rs` already contains `new_instance_with_webgpu_detection`: it probes a browser adapter rather than only checking `navigator.gpu`. Its instance chooses either browser WebGPU or the core/GL backend; it does not retry failed application device requests.

`Graphics::initialize` currently binds the real window surface before requesting an adapter/device. `App::device_supports_r16float` checks adapter filtering and sampling but not upload usage or the features actually enabled on the device. Browser device requests fix the 3D limit at 1024; `RenderContent::from_bytes` does not check dimensions before GPU allocation. Signed RG8 storage is already repaired and has exhaustive tests. Existing active resource-lifetime work explicitly excludes dependency upgrades, and the MIP glam change is unrelated.

## Goals / Non-Goals

**Goals:** select a usable browser backend before binding the application's canvas, snapshot capabilities of the selected device, preserve native selection and public APIs, and provide reproducible WebGL diagnostics.

**Non-Goals:** runtime device-loss recovery; backend switching after binding the real canvas; replacement of the Winit window/canvas and its event listeners; migration of dependencies; new storage-texture/compute capabilities not used by current rendering; changes to clinical precision or image algorithms.

## Decisions

### 1. Browser negotiation happens before real-canvas binding

Keep the native surface-first path unchanged. For WASM, construct the detection instance with `BROWSER_WEBGPU | GL`; request the candidate adapter without a real compatible surface and request its device using the supported browser limits. WebGPU does not need a presentation surface for adapter selection. If detection chooses GL, select GL directly. If the detected WebGPU adapter request returns none or its device negotiation fails, discard that candidate and make one explicit GL attempt with a new GL-only instance. No loops or native fallback behavior are introduced.

Use a disposable detached canvas for browser candidate surface compatibility checks where a compatible surface is needed by the GL path; this surface must belong to the candidate instance and be dropped after probing. Before committing WebGPU, query the candidate's surface format/present-mode/alpha-mode capabilities with that disposable canvas. Unsupported required volume capabilities are also a pre-commit failure. Bind the real Winit canvas only after selection succeeds. Never create a GL surface on a canvas previously used for WebGPU.

After binding the real canvas, surface creation/configuration or pipeline errors are actionable errors, not reasons to retry another context type on the same canvas. This is deliberately not a general 'any WebGPU failure falls back' promise. Forced WebGL is available on a fresh page load as recovery. Replacing the real canvas after binding would require coordinated Winit/event-loop ownership changes and is excluded.

Alternative: merely replace `Instance::new` with the detection helper. Rejected as insufficient for adapter/device negotiation failures. Alternative: retry on the same real canvas. Rejected because browser canvas context types are sticky. Alternative: rebuild the Winit window on failure. Excluded to preserve lifecycle/control compatibility.

### 2. A device-owned capability snapshot is the single decision source

Add a small rendering-owned capability type attached to `Graphics` with backend, usable filtered R16Float/RG8 flags, and effective `device.limits().max_texture_dimension_3d`. Keep fields read-only through a Rust accessor; do not expose wgpu-specific structs through wasm-bindgen. Preserve existing constructors and re-exports.

A usable volume format requires both `TEXTURE_BINDING | COPY_DST` and `FILTERABLE`. Derive this from capabilities legal under the enabled device feature set: guaranteed format features for enabled features, with adapter-specific extras only if the corresponding enabling feature was actually requested and enabled. Do not request native-only optional format features on browsers merely to prefer half-float. Prefer R16Float if usable; otherwise require usable RG8. Reject initialization if neither volume path is usable. Do not equate WebGPU with R16Float, GL with RG8, or FLOAT32_FILTERABLE with R16Float support.

Use this snapshot in `App::initialize`, graphics replacement, and format toggles. Preserve a user's RG8 preference during graphics replacement, retaining existing CPU view state and device/attachment ownership. Keep byte encoding in `src/data/volume_encoding.rs`; graphics only chooses representations.

Alternative: leave adapter checks scattered in App. Rejected because device features and limits must match the resource owner, especially after replacement.

### 3. Negotiate browser limits without shrinking data

Use downlevel WebGL2 defaults for the existing rendering requirements on both browser backends, promoting the 3D limit to the candidate adapter's supported value instead of a hard-coded 1024. Verify requested limits against the adapter before requesting a device and report incompatible limits. Native requests remain unchanged. This does not require WebGPU-specific compute or storage features.

Before each volume texture allocation, check nonzero dimensions and each axis against the effective device 3D limit. Check the expected two-byte-per-voxel length with checked arithmetic and reject malformed data before upload. Return the existing subsystem result/error type with numeric dimensions/limits and encoding, not study names. Never clamp, resample, truncate, or change signed voxels to fit a device.

### 4. Diagnostics and rollback use existing mechanisms

Make WASM feature declarations explicitly include `webgpu` and `webgl`; retain both versions and the lockfile. Browser selection accepts `?kepler_wgpu_backend=gl` before startup; absent or `auto` means detection/preference, and unknown values warn and use auto. No `std::env` browser configuration and no new UI/control API are needed. Read only this key; do not log the full URL.

Log actual backend, adapter, device-effective maximum 3D dimension, usable R16Float flag, and default encoding. Fallback logs identify the failed backend and negotiation stage. Use the existing web/native logger, avoiding voxel metadata and identifiers. Use forced GL to diagnose/roll back preference without changing storage code; a full rollback can restore WASM GL selection while retaining capability validation.

### 5. Verification separates policy from browser execution

Write deterministic regressions first for format usage/filter combinations, device-enabled feature handling, limit negotiation, zero/oversized dimensions, malformed uploads, selection overrides, retry eligibility, and one-shot fallback exhaustion. Exercise the real orchestration via an injectable internal attempt seam rather than tests that only reproduce expected policy. Retain existing native pixel/encoding/resource tests. Probe changed Rust paths with LSP and run the build matrix; global baseline failures remain visible.

Manual browser checks use a generated synthetic signed volume, covering negative values and a low-byte carry, plus MPR, oblique MPR, MIP/MinIP/AvgIP and mesh. Record browser/version, selected backend, encoding, limits, window/level, resize, input, redraw/idle, and restoration. Test WebGPU enabled, disabled/unavailable, forced GL, and a controllably failed pre-canvas WebGPU attempt. No claims of speed, browser parity, or coverage are made before measurement.

## Risks / Trade-offs

- Probe adapter availability can change before the real request → handle the subsequent failed request explicitly and retry GL before canvas binding.
- Disposable canvas checks are not a proof that the real canvas will succeed → report commit-stage errors and document fresh-page forced GL; do not promise post-binding fallback.
- Browser capability tables and native adapter-specific features differ → use enabled-device features, conservative format legality, and independent backend runtime checks.
- Smaller devices cannot render a large volume → reject uploads clearly; preserve data and do not conceal this as successful rendering.
- WebGPU shader/presentation behavior can differ from GL → keep shader/layout/linear-format policy unchanged and leave manual image checks open until executed.
- Lockfile may not contain dependencies needed by explicit features → first check locked native/WASM builds; stop for a separate dependency decision rather than silently updating versions/lockfile.

## Migration Plan

1. Add failing focused regressions, then implement capability/validation changes without altering signed storage or native selection.
2. Implement browser-only pre-canvas negotiation, bounded GL fallback, explicit features and diagnostic override.
3. Run native/WASM locked compilation and existing GPU correctness checks; update current guidance/changelog with actual behavior.
4. Execute and record independent browser backend checks. Keep this change open if a required manual environment is unavailable; do not archive or sync intended browser behavior as verified completion.
