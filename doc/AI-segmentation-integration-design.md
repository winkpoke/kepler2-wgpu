# AI Segmentation Integration Design
## TotalSegmentator Integration for dirac2 + kepler2-wgpu

**Project:** dirac2 + kepler2-wgpu
**Author:**  admin
**Date:**    2026-09-17
**Version:** 2.0

> ⚠️ **状态核对 (2026-09-23)**
> - 文中出现的 `PythonRuntime`、`scripts/deploy_python_runtime.ps1` 属于**已废弃的嵌入式
>   PyO3 路径**（`python_runtime.rs` / `.pyembed/`），**勿据此实现**。现行为：AI 推理以
>   **ONNX 为主**（`ort` + `directml`），Python 仅作**可选 HTTP fallback**
>   （`KEPLER_AI_URL`，默认 `http://localhost:8001`），走 `src/server/python/`。
> - `NnUNetBackend` / `RawMask` 等为**设计层概念名**，代码中没有同名类型。
>   实际分割入口见 `src/server/segment_engine.rs`、`ai_handler.rs`、`ai_task.rs`。
> - 数值链路的硬约束（重排 / LPS↔RAS / 重采样 / logits 投影）见
>   `doc/agents/PITFALLS.md` 与 `src/server/resample.rs`、`orientation.rs`。

---

# 1. Overview

This document describes the design for integrating AI-based anatomical
segmentation into the existing medical imaging platform.

The current system consists of:

- Electron desktop application (`dirac2`)
- Rust backend server (`axum`)
- WebSocket communication layer
- Rust/WASM rendering engine (`kepler2-wgpu`)
- WebGPU-based MPR, Volume Rendering, and Mesh Rendering pipelines

The goal is to integrate the **TotalSegmentator** model to provide:

- Automatic multi-organ segmentation
- Segmentation overlay in MPR views
- Segmentation overlay in Volume Rendering
- Surface mesh generation from segmentation masks
- Anatomical measurements and future AI-assisted workflows

---

# 2. Existing Architecture

```text
Electron / Browser UI
          │
          │ WebSocket
          │
      Rust Axum Server
          │
          │
      kepler2-wgpu
          │
          │
      WebGPU Renderer
          │
          ├── MPR
          ├── Volume Rendering
          └── Mesh Rendering
```

# 3. Design Goals

## Functional Goals

1. Automatic segmentation of CT datasets.
2. Real-time segmentation progress reporting.
3. Segmentation visualization in all viewers.
4. Segmentation mesh generation.
5. Future support for additional AI models.

## Non-Functional Goals

1. No modifications to existing coordinate systems.
2. No AI inference inside WASM.
3. No AI inference inside WebGPU shaders.
4. Minimal changes to existing rendering pipelines.
5. Extensible architecture for future AI services.
6. `kepler.exe` is the single user entry point — no separate Python server.

---

# 4. Proposed Architecture (v2 — embedded, no external service)

```text
Electron / Browser
          │
          │ WebSocket
          │
      Rust Axum Server (kepler.exe)
          │
          │  POST /api/segment   (single endpoint)
          │
          ▼
   ModelManager  ── has_model("{m}.onnx")?
          │
          ├── Auto ──┐
          │          │
          ▼          ▼
   OnnxBackend   NnUNetBackend (embedded Python)
          │               │
          │               ▼
          │        PythonRuntime (PyO3, in-process)
          │               │
          │               ▼
          │        inference.py → TotalSegmentator (nnU-Net)
          │               │
          └──────► SegmentResult (same shape for both)
                      │
                kepler2-wgpu
                  ├── MPR
                  ├── Volume Rendering
                  ├── Mesh Rendering
                  └── Segmentation Overlay
```

Key principles (v2):

- **No external Python HTTP server.** The Python interpreter lives *inside*
  `kepler.exe` via PyO3. There is no `reqwest` → FastAPI hop, no
  `localhost:<port>`, and `ai.rs` (the old HTTP client) has been deleted.
- **Two interchangeable backends behind one trait.** Both
  `OnnxBackend` and `NnUNetBackend` implement `SegmentationBackend` and
  return the identical `SegmentResult` shape, so the Axum layer and the
  frontend never know which one ran.
- **`NnUNetBackend` is the accuracy reference.** It delegates the *entire*
  nnU-Net pipeline (resampling, canonical transpose, foreground crop,
  normalization, sliding window, Gaussian weighting, TTA, fold ensemble,
  inverse resampling, postprocessing) to the validated `inference.py`
  wrapper around TotalSegmentator. Rust never re-implements any of it.
- **`OnnxBackend` is the fast approximate path.** It reuses the existing
  `OnnxSegmentator` / `ModelManager` (with DirectML GPU EP). It is only
  used when a local `models/{model}.onnx` exists.

---

# 5. Why Python Runs Embedded (PyO3), Not as an External Service

TotalSegmentator is based on:

- Python
- PyTorch
- nnU-Net
- CUDA acceleration

Running the model inside WASM is not feasible because:

- WASM does not support CUDA execution.
- PyTorch inference inside WASM is impractical.
- Model size is very large.
- GPU memory requirements are significant.

However, deploying it as a **separate HTTP service** (the v1 design) was
rejected because it:

- forces a `Rust → HTTP → Python Server` round trip (extra latency, extra
  failure mode, extra process to manage);
- requires the user to start a second process;
- duplicates the API contract between Rust and Python.

Instead, v2 embeds the interpreter **in-process** with PyO3:

- `PythonRuntime` owns a single interpreter inside `kepler.exe`.
- It is **not** initialized at startup — `new()` only stores paths, so
  `kepler.exe` boots fine without a deployed Python runtime.
- `available()` is a cheap on-disk probe for `runtime/python/inference.py`.
- `segment()` lazily initializes the interpreter on the first call and then
  reuses it, so the model / CUDA context load exactly **once** for the whole
  process lifetime.
- Calls are wrapped in `tokio::task::spawn_blocking` so the GIL / PyTorch
  work never blocks the Axum async runtime.

---

# 6. Server Module Design

## Directory Structure

```text
server/
├── routes.rs           // 注册 HTTP / WS API（/api/segment 不变）
├── handlers.rs         // REST / WS 入口；构造 SegmentRequest（含 backend 字段）
├── ws.rs               // 收 websocket 消息、推送进度
├── state.rs            // 保存 python_runtime、TaskManager、onnx_models
├── ai_task.rs          // task 管理、进度查询
├── ai_model.rs         // 请求/响应结构体 + BackendPreference
├── ai_handler.rs       // 开始分割、取消、查询结果；按 backend 派发
├── backend.rs          // SegmentationBackend trait + OnnxBackend + NnUNetBackend
├── python_runtime.rs   // 内嵌 Python 运行时（PyO3）
├── segment_engine.rs   // OnnxSegmentator + ModelManager（ONNX 引擎，原样复用）
├── resample.rs         // ONNX 路径的体积三线性重采样 / mask 最近邻逆重采样
└── python/
    ├── inference.py    // 普通 Python 函数：run_totalsegmentator / extract_mask_bytes / mask_dimensions
    ├── __init__.py
    └── requirements.txt  // torch / totalsegmentator / nibabel / SimpleITK ...
```

> Deleted in v2: `ai.rs` (Rust→HTTP client), `python/app.py` (FastAPI
> server), `python/worker.py` (subprocess worker). The FastAPI/HTTP path is
> no longer used.

---

# 7. Module Responsibilities

## routes.rs / handlers.rs / ws.rs

Unchanged from v1. `handlers.rs` constructs `SegmentRequest` with
`backend: Default::default()` (i.e. `Auto`) for WebSocket-initiated
requests; external `/api/segment` callers may pin a backend.

## state.rs

Holds:

```text
ServerState
 ├── python_runtime : Arc<PythonRuntime>   // 内嵌 Python 运行时（启动不加载）
 ├── onnx_models     : Arc<Mutex<Option<Arc<ModelManager>>>>  // ONNX 注册表
 ├── tasks           : TaskManager
 └── ... (rendering / volume state 不变)
```

`python_runtime` is created in `ServerState::new()` but does **not** start
the interpreter (matches the "startup does not load Python" rule).

## ai_model.rs

- `SegmentRequest { series_id, model, backend: BackendPreference }`
- `BackendPreference` enum: `Auto` (default) | `Onnx` | `Python`
  (serializes as `"auto"` / `"onnx"` / `"python"`).

## backend.rs — `SegmentationBackend` trait

```rust
#[async_trait]
pub trait SegmentationBackend {
    fn name(&self) -> &'static str;            // "onnx" / "python"
    async fn segment(
        &self,
        req: &ApiSegmentRequest,
        volume: Arc<Vec<f32>>,                // ONNX 用；Python 忽略
        shape: (usize, usize, usize),
    ) -> Result<EngineResult>;
}
```

- `OnnxBackend { mgr: Arc<ModelManager> }` — wraps existing ONNX engine,
  session cache reused, `spawn_blocking` for sync ORT.
- `NnUNetBackend { runtime: Arc<PythonRuntime> }` — embedded Python
  nnU-Net reference; checks `available()`, runs `segment()` inside
  `spawn_blocking`, maps `RawMask` → `EngineResult`.

## python_runtime.rs — `PythonRuntime`

- `new(series_dir, output_root)` — stores paths only, no interpreter init.
- `available() -> bool` — cheap probe for `runtime/python/inference.py`.
- `segment(model, series_id) -> Result<RawMask>` — PyO3 call into
  `inference.py`; lazily initializes the interpreter (model loaded once).

## inference.py

Plain Python module (no server). Exposes:

- `run_totalsegmentator(input_path, output_dir, model, fast)` — wraps
  TotalSegmentator, merges L1..S1 + sacrum into a single `spine.nii.gz`.
- `resolve_task(model) -> str` — maps a kepler model name to a real
  TotalSegmentator task name.
- `extract_mask_bytes(output_dir) -> bytes` — returns the (Z,Y,X) `uint8`
  mask buffer.
- `mask_dimensions(output_dir) -> (z, y, x)`.

### Why the model→task mapping exists

`TotalSegmentator.python_api.totalsegmentator()` dispatches on a very long
`if task == "...": task_id = ...` chain **with no else branch**. Passing a
name it does not recognise (e.g. the kepler model identifier
`"totalsegmentator"`, which is *not* a TS task) falls off the end of the chain
with `task_id` never bound, raising:

```text
UnboundLocalError: cannot access local variable 'task_id' where it is not associated with a value
```

So Rust passes the **model name**, and `inference.py` translates it via
`_MODEL_TO_TASK` (unknown names degrade to `"total"` with a warning logged)
before calling TS. Never forward a kepler model name to TS as `task`.

`fast=False` is used so TS selects the 1.5 mm model
(`task_id = [291, 292, 293, 294, 295]`, a 5-fold ensemble), matching the ONNX
path's `[1.5, 1.5, 1.5]` resampling so the two backends stay comparable.

---

# 8. WebSocket Message Design

(Unchanged from v1.)

## Start Segmentation

Client → Server

```json
{
  "type": "segment",
  "model": "totalsegmentator",
  "series": "CT001"
}
```

## Segmentation Accepted / Progress / Completed / Failed

(Identical message shapes to v1: `segment_started`, `segment_progress`,
`segment_complete`, `segment_failed`, `segment_cancel`.)

---

# 9. Embedded Python Runtime (replaces the v1 FastAPI service)

v1 recommended:

```text
FastAPI + TotalSegmentator + PyTorch + CUDA
POST /segment
GET  /task/{id}
```

v2 replaces this with an **in-process** runtime:

```text
kepler.exe
   │
   │  PyO3 (no socket, no HTTP)
   ▼
inference.py
   │
   ▼
TotalSegmentator (nnU-Net reference)
   │
   ▼
RawMask bytes → EngineResult
```

Deployment layout (Python runtime is an external *resource*, not bundled
into the EXE — so `kepler.exe` stays small and the runtime can be updated
independently):

```text
Kepler/
├── kepler.exe
├── models/
│   └── totalsegmentator.onnx   (optional; enables the ONNX path)
└── runtime/
    └── python/
        ├── python.exe / Lib / DLLs / ...
        ├── inference.py
        └── (totalsegmentator + torch + nibabel + SimpleITK installed)
```

`PythonRuntime` resolves `runtime/python/inference.py` relative to
`kepler.exe` — it never hard-codes `C:\Python...` or a dev venv.

### 9.1 Load-time Python DLL (STATUS_DLL_NOT_FOUND fix)

pyo3 **links `python3xx.dll` at load time** — the import table of
`kepler.exe` references it, so the process dies with
`STATUS_DLL_NOT_FOUND` (0xc0000135) before `main()` runs unless the DLL is
resolvable in the standard search order (exe dir → System32 → PATH). The
lazy `available()` probe cannot help here; the loader needs the files first.

Deployment therefore also stages the **python.org embeddable CPython**
(freely redistributable, PSF license) *next to the exe*:

```text
Kepler/
├── kepler.exe
├── python.exe               ← interpreter host (to bootstrap/install deps)
├── python311.dll            ← load-time linked interpreter (embeddable pkg)
├── python3.dll              ← stable-ABI forwarder
├── python311.zip            ← PURE-PYTHON stdlib (no C extensions inside!)
├── python311._pth           ← sys.path config (import site ENABLED)
├── *.pyd                    ← C-extension modules (_socket, _ssl, _ctypes…)
├── vcruntime140.dll / libcrypto-3.dll / libssl-3.dll / sqlite3.dll / ...
├── runtime/python/          ← inference.py (+ optional site-packages/)
└── models/ ...
```

> **Where the ML stack does NOT live.** It must not be installed into
> `<target>/debug/Lib/site-packages`. Cargo garbage-collects unrecognised
> top-level directories in the profile dir, so that path is wiped on rebuild
> (observed: `Lib/` and `Scripts/` disappearing after a plain `cargo build`,
> which surfaced as `ModuleNotFoundError: No module named 'SimpleITK'`).
> `PythonRuntime` therefore resolves the stack from a durable location —
> see [Resolving the ML stack](#resolving-the-ml-stack).

Tooling (already implemented):

- `scripts/deploy_python_runtime.ps1` — downloads the embeddable package
  once into `<repo>/.pyembed` (MD5-pinned), extracts it, and stages the
  files into `target\debug` (or `-TargetDir target\release`). It copies:
  the loader DLLs, `python.exe`, ALL `*.pyd` C-extension modules, the
  `_pth` (with `import site` already enabled), **and** the user's
  `src/server/python/{inference.py,__init__.py,requirements.txt,models}`
  into `runtime/python/`.
  The version MUST match the minor version pyo3 linked against
  (`python311` → 3.11.x, default 3.11.9).
- `build.rs` — repeats the DLL + `*.pyd` + python-source copy step on **every**
  build, using idempotent `stage_if_needed` (files with a matching size are
  skipped, so the steady state copies nothing). It deliberately declares **no**
  `cargo:rerun-if-changed` inputs: with no declared inputs cargo re-runs the
  script on every build, which is what lets it repair out-of-band deletions.
  It only warns when a load-time critical file is genuinely absent next to the
  exe, or when `.pyembed/` is missing (so other machines can still compile).

### Resolving the ML stack

`PythonRuntime::resolve_site_packages` probes, in priority order, and injects
every hit into `sys.path` before importing `inference`:

1. `KEPLER_PYTHON_SITE_PACKAGES` — `;`-separated env override for custom
   deployments.
2. `<exe_dir>/runtime/python/site-packages` — the packaged layout, where the
   stack travels next to the exe in shipped builds.
3. `<exe_dir>/../../src/server/totalsegmentator/.venv/Lib/site-packages` —
   the development layout (`target/<profile>` → repo root → committed venv).

The resolved paths are logged at startup as
`[AI/Python] site-packages: ...`, so a mis-provisioned environment is obvious
in the log rather than as a mid-inference `ImportError`.

### Provisioning the ML stack (one-time, dev)

The embeddable interpreter ships **no packages**. Provision the stack into the
dedicated venv that `PythonRuntime` looks for in development builds:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/install_python_ml_deps.ps1
# recreate from scratch:
powershell -ExecutionPolicy Bypass -File scripts/install_python_ml_deps.ps1 -Force
```

The script creates `src/server/totalsegmentator/.venv` (via `uv`, falling back
to the embedded interpreter's `venv`), installs `TotalSegmentator` with the
CUDA wheel index, then re-asserts `torch`/`torchvision` from that index and
verifies `torch.cuda.is_available()`.

Notes / gotchas:
- **CUDA torch gets clobbered by TotalSegmentator.** Its dependency graph can
  pull a **PyPI CPU** `torch`, silently disabling CUDA. The script re-asserts
  the pair from the CUDA index afterwards; do NOT rely on a bare
  `pip install TotalSegmentator` for a GPU build.
- The PyTorch PyPI wheel on Windows is **CPU-only**; the CUDA build only comes
  from the pytorch index (RTX 50-series needs cu124+; `cu128` if `cu124` fails
  to load).
- The venv's CPython minor version must match the embedded interpreter's ABI
  (`python311` → 3.11.x). `uv venv --python 3.11` guarantees this.
- `get-pip.py` must be bootstrapped **after** the `*.pyd` files are present,
  or it dies with `No module named '_socket'` (the `.pyd` C extensions are
  NOT inside `python311.zip`).
- `import site` is required in `python311._pth` so `Lib/site-packages` is on
  the path — both for `pip` and for the embedded interpreter at runtime.
- `requirements.txt` deliberately omits `torch`/`nnunetv2` (TotalSegmentator
  pulls them) and contains **no** `fastapi`/`uvicorn`/`pydantic` — the HTTP
  server was deleted, so those are dead.
- TotalSegmentator downloads its model weights (~hundreds of MB) on first
  inference, not at install time.
- The venv is durable across `cargo clean` (it lives under `src/`), so the
  stack does not need reinstalling after a clean. Only the staged runtime
  next to the exe is reprovisioned — `build.rs` does that automatically.


---

# 10. Backend Selection / Comparison Switch

`POST /api/segment` accepts an optional `backend` field. This is what lets
you **compare ONNX vs Python head-to-head on the same CT volume**:

| `backend` | Behaviour |
|---|---|
| `"auto"` (default) | ONNX if `models/{model}.onnx` exists, else embedded Python; else clear error |
| `"onnx"` | Force ONNX. Errors if no local `.onnx`. |
| `"python"` | Force embedded Python nnU-Net reference. Errors if runtime not deployed. |

Dispatch logic in `ai_handler::run_segmentation`:

```text
pref        = req.backend            // Auto | Onnx | Python
has_onnx    = onnx_models.has_model(model)
py_available= python_runtime.available()

use_onnx    = (pref==Onnx  && has_onnx) || (pref==Auto && has_onnx)
use_python  = (pref==Python && py_available) || (pref==Auto && !has_onnx && py_available)

if use_onnx   { OnnxBackend::new(mgr).segment(...) }
elif use_python { NnUNetBackend::new(rt).segment(...) }
else { fail_task(clear error: which backend was requested + why unavailable) }
```

Example calls:

```bash
# Force ONNX (even if Python runtime is also deployed)
curl -X POST localhost:3000/api/segment \
  -H 'content-type: application/json' \
  -d '{"series_id":"S1","model":"totalsegmentator","backend":"onnx"}'

# Force the nnU-Net reference (Python)
curl -X POST localhost:3000/api/segment \
  -H 'content-type: application/json' \
  -d '{"series_id":"S1","model":"totalsegmentator","backend":"python"}'

# Default (Auto) — unchanged behaviour for the existing WASM frontend
curl -X POST localhost:3000/api/segment \
  -H 'content-type: application/json' \
  -d '{"series_id":"S1","model":"totalsegmentator"}'
```

The route and the frontend are unaffected: they still only ever see
`/api/segment` and `SegmentResult`.

---

# 11. Segmentation Data Flow

Current pipeline:

```text
DICOM
   │
CTVolume
   │
Density Texture
   │
Volume Rendering
```

Extended pipeline:

```text
DICOM
   │
CTVolume
   │
   ├── Density Texture
   │
   └── Segmentation Texture
```

Both textures share the same coordinate system.

---

# 12. Segmentation Output Format

TotalSegmentator produces:

```text
spine.nii.gz
```

`inference.py::extract_mask_bytes` converts this into a raw `uint8` buffer
(`width*height*depth`, (Z,Y,X) row-major) which `PythonRuntime` returns as
`RawMask.mask` and `NnUNetBackend` forwards as `EngineResult.mask_data`.

Label table (spinal subset):

```json
{
  "1": "L1", "2": "L2", "3": "L3",
  "4": "L4", "5": "L5", "6": "S1",
  "7": "sacrum"
}
```

---

# 13. Segmentation Volume Structure

```text
SegmentationVolume
├── dimensions
├── labels
└── label_table
```

Example:

```text
512 x 512 x 300
uint8 labels
```

Label values:

```text
0 = background
1..7 = L1..S1, sacrum
```

---

# 14. GPU Texture Design

Recommended texture format:

```text
TextureFormat::R8Uint
```

Advantages:

- Compact memory footprint
- Fast upload
- Simple shader sampling
- Direct label lookup

Pipeline:

```text
SegmentationVolume
        │
Texture3D<R8Uint>
        │
Shader Sampling
```

---

# 15. MPR Integration

Segmentation textures must use exactly the same coordinate system as CTVolume.

Coordinate information:

```text
ImagePositionPatient
ImageOrientationPatient
Voxel Spacing
Base Matrix
```

Both textures share:

```text
CT Texture
Segmentation Texture
```

Benefits:

- Axial synchronization
- Coronal synchronization
- Sagittal synchronization
- Oblique synchronization
- No additional transforms required

---

# 16. Volume Rendering Integration

Current pipeline:

```text
Ray Marching
Window/Level
Transfer Function
```

Extended pipeline:

```text
Ray Marching
      │
Sample Density
      │
Sample Segmentation Label
      │
Apply Overlay Color
      │
Composite Result
```

Example:

```text
Label = 1
Color = Green
Opacity = 0.3
```

---

# 17. Mesh Integration

Pipeline:

```text
Segmentation Mask
        │
Marching Cubes
        │
Vertex Buffer
        │
Index Buffer
        │
Mesh Rendering
```

Benefits:

- Surface visualization
- Organ isolation
- Measurement tools
- Surgical planning

---

# 18. Task Lifecycle

```text
Client
   │
segment
   │
Rust Server
   │
Create Task (ai_task::TaskManager)
   │
run_segmentation
   │
   ├─ use_onnx?    → OnnxBackend → ORT (DirectML)
   │
   └─ use_python?  → NnUNetBackend → PythonRuntime (PyO3) → TotalSegmentator
   │
Progress Updates (WebSocket)
   │
Completed → SegmentResult (mask + labels + dimensions)
   │
Create Texture
   │
Render Overlay
```

---

# 19. Final Architecture

```text
                 dirac2
                    │
               WebSocket
                    │
                 Axum (kepler.exe)
                    │
        ┌───────────┴────────────┐
        │                        │
   kepler2-wgpu            PythonRuntime (PyO3, in-process)
        │                        │
        │                  inference.py
        │                        │
        │                  TotalSegmentator (nnU-Net reference)
        │                        │
        ├── MPR                  │
        ├── Volume Rendering     │
        ├── Mesh Rendering       │
        ├── Segmentation Overlay │  ── SegmentResult (same shape from both)
        └── Measurements         │
```

The integration embeds the Python inference runtime directly inside
`kepler.exe` and exposes it as a `SegmentationBackend` peer to the ONNX
path, while preserving the existing rendering architecture and coordinate
systems.
