# kepler2-wgpu AI service

A small FastAPI wrapper around [TotalSegmentator](https://github.com/wasserth/TotalSegmentator)
that is consumed by the kepler2-wgpu Rust server (`src/server/ai.rs`).
The service is intentionally minimal: it only segments the **spine**,
so downstream code does not need to know about TotalSegmentator's 117-class taxonomy.

```
kepler2-wgpu
  ├── Electron / Browser  ← user clicks "Segment" button
  │       │
  │       │ WebSocket
  │       ▼
  └── Rust server  (axum)  ← src/server/ai_handler.rs
            │
            │ HTTP (reqwest, KEPLER_AI_URL, default http://localhost:8001)
            ▼
  ┌───────────────────────────────────┐
  │  THIS SERVICE (FastAPI)           │
  │  GET  /api/health                 │
  │  POST /api/volumes/upload         │
  │  POST /api/segment                │
  │  POST /api/segment/cancel         │
  │  GET  /api/segment/progress/:id   │  ← Rust polls this every second
  │  GET  /api/segment/result/:id     │  ← base64 mask, dimensions, labels
  │  GET  /api/segment/result/:id/raw │  ← raw mask, dimensions, labels, raw
  │  GET  /api/cached_mask/:id        │
  │  GET  /ws                         │
  └────────────┬──────────────────────┘
               │
               ▼
        TotalSegmentator (PyTorch, CUDA / MPS / CPU)
               │
               ▼
        NIfTI masks → collapsed to binary `spine.nii.gz`
```

The mask returned in `mask_base64` is a contiguous
`width * height * depth` `uint8` buffer where `1` means "spine" and `0`
means "background". That is the exact format expected by
`AiService::download` and `render_content::from_labels_r8` on the Rust
side.

---

## Install

The service has a heavy dependency stack (PyTorch + TotalSegmentator +
nnU-Net). Use a virtualenv or conda env to keep the system Python clean.

```bash
cd src/server/python
python -m venv .venv
source .venv/bin/activate         # Windows: .venv\Scripts\activate

# CPU-only is fine for prototyping. For GPU inference install the CUDA build
# of torch first (https://pytorch.org/get-started/locally/) and then:
pip install -r requirements.txt
```

> TotalSegmentator downloads the pre-trained nnU-Net weights the first
> time it runs. You can pre-download them with
> `TOTALSEG_HOME_DIR=$PWD/models totalsegmentator -h` or just let the
> first `/segment` request trigger the download. The `models/` directory
> is the right place for them so they are versioned alongside the code.

## Run

```bash
# 1. Where the Rust server saves uploaded MHA files. Both services must agree on this directory.
# export KEPLER_SERIES_DIR= C:/user/kepler_series

# 2. Where to write TotalSegmentator intermediate + final output.
# export KEPLER_OUTPUT_ROOT=C:/user/kepler_ai_output

# 3. (optional) Run TotalSegmentator in fast mode (ONLY FOR CPU MODE).
# export KEPLER_TS_FAST=0

# 4. Start the service. Default port 8001 matches KEPLER_AI_URL in src/server/ai.rs.
# uv run --python "..\totalsegmentator\.venv\Scripts\python.exe" uvicorn app:app --host 0.0.0.0 --port 8001
uvicorn app:app --host 0.0.0.0 --port 8001
```

## File layout

```
src/server/python/
├── __init__.py         # package marker + module docstring
├── app.py              # FastAPI service, task store, HTTP endpoints
├── inference.py        # TotalSegmentator wrapper, spine-only post-processing
├── requirements.txt    # pinned-ish dependencies
├── README.md           # this file
└── models/             # TotalSegmentator pre-trained weights (downloaded on first run)
```