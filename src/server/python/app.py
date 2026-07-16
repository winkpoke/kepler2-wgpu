"""FastAPI service wrapping TotalSegmentator for the kepler2-wgpu stack.

This service is intentionally small and decoupled from the Rust server:

- ``POST /segment``  — submit a task, returns a task_id immediately
- ``GET  /task/{id}`` — poll progress and (eventually) the base64 mask
- ``GET  /result/{id}`` — convenience: returns the mask bytes + metadata
- ``POST /task/{id}/cancel`` — request cancellation of a running task
- ``GET  /health``  — liveness probe

The Rust server polls ``/task/{id}`` once per second from a background
task and forwards ``progress`` / ``complete`` / ``failed`` events to the
browser over WebSocket. The contract here matches ``AiServiceTask`` in
``src/server/ai_model.rs``.

The mask returned in ``mask_base64`` is a contiguous
``width * height * depth`` ``uint8`` buffer where ``1`` means "spine"
and ``0`` means "background". That is the exact format expected by
``AiService::download`` and ``render_content::from_labels_r8`` on the
Rust side.
"""
from __future__ import annotations

import base64
import logging
import os
import threading
import time
import traceback
import uuid
from concurrent.futures import ThreadPoolExecutor
from typing import Optional

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

from inference import (
    DEFAULT_TASK,
    LABEL_TABLE,
    extract_mask_bytes,
    mask_dimensions,
    run_totalsegmentator,
)

# ---------------------------------------------------------------------------
# Configuration (all overridable via env so this can be deployed next to the
# Rust server or in a separate container without code changes).
# ---------------------------------------------------------------------------
LOG_LEVEL = os.environ.get("LOG_LEVEL", "INFO")
SERIES_DIR = os.environ.get("KEPLER_SERIES_DIR", "/tmp/kepler_series")
OUTPUT_ROOT = os.environ.get("KEPLER_OUTPUT_ROOT", "/tmp/kepler_ai_output")
# Whether to run TotalSegmentator in `fast` mode (half resolution, ~2x speedup).
# Safe default for prototyping; flip to "0" if you need full-resolution masks.
TS_FAST = os.environ.get("KEPLER_TS_FAST", "1") not in ("0", "false", "False")

logging.basicConfig(
    level=LOG_LEVEL,
    format="%(asctime)s %(levelname)-7s %(name)s: %(message)s",
)
log = logging.getLogger("kepler_ai")

os.makedirs(SERIES_DIR, exist_ok=True)
os.makedirs(OUTPUT_ROOT, exist_ok=True)

app = FastAPI(title="kepler2-wgpu AI service")

# In-process task store. Mutated only under `_lock`. Lifetime: process
# lifetime. For a production deployment this should be moved to Redis or
# similar; for the kepler2-wgpu dev workflow a dict is enough.
_lock = threading.Lock()
_tasks: dict[str, dict] = {}

# Single-worker executor: TotalSegmentator already saturates the GPU,
# running two at once would just thrash VRAM.
_executor = ThreadPoolExecutor(max_workers=1, thread_name_prefix="kepler-ai")

# Label table exposed over the wire. Keys are strings because Rust's
# `HashMap<String, String>` round-trips JSON cleanly.
LABELS_FOR_API = {str(k): v for k, v in LABEL_TABLE.items()}


# ---------------------------------------------------------------------------
# Request / task model
# ---------------------------------------------------------------------------
class SegmentRequest(BaseModel):
    series_id: str
    model: str = "totalsegmentator"
    # Override the TotalSegmentator task (e.g. "total", "vertebrae_body").
    # When omitted, the service falls back to DEFAULT_TASK.
    task: Optional[str] = None


# ---------------------------------------------------------------------------
# Task store helpers
# ---------------------------------------------------------------------------
def _new_task_record() -> dict:
    return {
        "status": "pending",
        "progress": 0,
        "message": None,
        "volume": None,
        "labels": None,
        "dimensions": None,
        "mask_base64": None,
    }


def _set_task(task_id: str, **kwargs) -> None:
    with _lock:
        t = _tasks.setdefault(task_id, _new_task_record())
        t.update(kwargs)


def _get_task(task_id: str) -> dict:
    with _lock:
        t = _tasks.get(task_id)
        if t is None:
            raise HTTPException(status_code=404, detail=f"Unknown task {task_id}")
        # Shallow copy so the caller cannot mutate the store.
        # Inject task_id into the response: the Rust side's AiServiceTask
        # struct requires it as a non-optional field, even though it's
        # redundant with the URL path. The record itself does NOT store
        # task_id (it's the dict key), so we add it on the way out.
        out = dict(t)
        out["task_id"] = task_id
        return out


# ---------------------------------------------------------------------------
# Background worker
# ---------------------------------------------------------------------------
def _run_task(task_id: str, series_id: str, model: str, task: str) -> None:
    """The function executed on the single-worker executor.

    TotalSegmentator's `python_api` does not expose progress callbacks, so
    we step the progress percentage in coarse chunks. The Rust poller
    forwards whatever we set here straight to the browser, so emitting
    fewer-but-larger steps is fine; the visual smoothing happens in the
    CSS transition on the front-end.
    """
    try:
        series_path = os.path.join(SERIES_DIR, f"{series_id}.mha")
        if not os.path.exists(series_path):
            raise FileNotFoundError(
                f"Series {series_id} not found on disk: {series_path}. "
                f"Did the Rust server save the upload to {SERIES_DIR}?"
            )

        output_dir = os.path.join(OUTPUT_ROOT, task_id)
        os.makedirs(output_dir, exist_ok=True)

        _set_task(task_id, status="running", progress=10, message=None)

        # Inference. This is the slow step (30-90s on a modern GPU for
        # `vertebrae_body` in fast mode; several minutes for `total`).
        run_totalsegmentator(series_path, output_dir, task=task, fast=TS_FAST)

        # Post-process: extract the combined mask and base64-encode it.
        _set_task(task_id, progress=80, message="encoding mask")
        mask_bytes = extract_mask_bytes(output_dir)
        dims = mask_dimensions(output_dir)
        volume = f"seg-{task_id}"

        _set_task(
            task_id,
            status="completed",
            progress=100,
            volume=volume,
            labels=LABELS_FOR_API,
            dimensions=list(dims),
            mask_base64=base64.b64encode(mask_bytes).decode("ascii"),
            message=f"mask {dims[0]}x{dims[1]}x{dims[2]}, {len(mask_bytes)} bytes",
        )
        log.info("Task %s completed: %s", task_id, _tasks[task_id]["message"])
    except Exception as e:
        log.error("Task %s failed: %s\n%s", task_id, e, traceback.format_exc())
        _set_task(task_id, status="failed", message=str(e))


# ---------------------------------------------------------------------------
# HTTP endpoints
# ---------------------------------------------------------------------------
@app.post("/segment")
def submit_segmentation(req: SegmentRequest):
    """Accept a segmentation job and return a task_id immediately."""
    task_id = uuid.uuid4().hex[:12]
    ts_task = req.task or DEFAULT_TASK

    _set_task(task_id, **_new_task_record())
    _executor.submit(_run_task, task_id, req.series_id, req.model, ts_task)
    log.info(
        "Submitted task %s series=%s model=%s ts_task=%s",
        task_id, req.series_id, req.model, ts_task,
    )
    return {"status": "accepted", "task_id": task_id}


@app.get("/task/{task_id}")
def get_task(task_id: str):
    """Poll the current state of a task. Rust polls this once a second."""
    return _get_task(task_id)


@app.get("/result/{task_id}")
def get_result(task_id: str):
    """Return the final mask for a completed task as JSON.

    Mirrors `AiServiceTask` in Rust: when `status == "completed"`, the
    response also contains `mask_base64` so the Rust poller can pull the
    mask via the same endpoint it already polls.
    """
    t = _get_task(task_id)
    if t["status"] != "completed":
        raise HTTPException(
            status_code=409,
            detail=f"Task is {t['status']}, not completed",
        )
    return {
        "task_id": task_id,
        "volume": t["volume"],
        "labels": t["labels"],
        "dimensions": t["dimensions"],
        "mask_base64": t["mask_base64"],
    }


@app.post("/task/{task_id}/cancel")
def cancel_task(task_id: str):
    """Request cancellation. TotalSegmentator cannot be interrupted mid-
    pipeline, so this only flips the task state to `cancelled` — the GPU
    job keeps running to completion in the background. The Rust poller
    will see `status == cancelled` and stop polling."""
    t = _get_task(task_id)
    if t["status"] in ("completed", "failed", "cancelled"):
        return t
    _set_task(task_id, status="cancelled", message="cancelled by client")
    return _get_task(task_id)


@app.get("/health")
def health():
    """Liveness probe + task count. Returns 200 as long as the process is up."""
    return {
        "status": "ok",
        "tasks": len(_tasks),
        "series_dir": SERIES_DIR,
        "output_root": OUTPUT_ROOT,
        "ts_fast": TS_FAST,
        "default_task": DEFAULT_TASK,
    }


@app.get("/cached_mask/{series_id}")
def get_cached_mask(series_id: str):
    """Return the latest cached segmentation mask for ``series_id``.

    Scans OUTPUT_ROOT for any task directory containing ``spine.nii.gz``
    and returns the most recent one as base64-encoded bytes plus
    dimensions, so the browser can re-upload the mask to the GPU
    without re-running TotalSegmentator.

    This is a fast path: it does not invoke the AI model. It only reads
    the on-disk ``spine.nii.gz`` written by the last completed task.
    """
    from pathlib import Path as _P

    if not os.path.isdir(OUTPUT_ROOT):
        return {"status": "missing", "reason": "no output directory"}

    candidates = []
    for task_dir in os.listdir(OUTPUT_ROOT):
        spine = _P(OUTPUT_ROOT) / task_dir / "spine.nii.gz"
        if spine.exists():
            candidates.append(spine)

    if not candidates:
        return {"status": "missing", "reason": "no cached spine.nii.gz"}

    # Most recently written wins.
    spine_path = max(candidates, key=lambda p: p.stat().st_mtime)

    try:
        mask_bytes = extract_mask_bytes(str(spine_path.parent))
        dims = mask_dimensions(str(spine_path.parent))
    except Exception as e:
        return {"status": "error", "reason": str(e)}

    return {
        "status": "ok",
        "task_id": spine_path.parent.name,
        "series_id": series_id,
        "volume": f"seg-{spine_path.parent.name}",
        "labels": LABELS_FOR_API,
        "dimensions": list(dims),
        "mask_base64": base64.b64encode(mask_bytes).decode("ascii"),
    }


if __name__ == "__main__":
    # Allow `python app.py` for quick local runs. The usual entry point
    # is `uvicorn app:app --host 0.0.0.0 --port 8001`.
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=int(os.environ.get("PORT", "8001")))
