"""kepler2-wgpu AI service.

A small FastAPI wrapper around TotalSegmentator. The service is fully
decoupled from the Rust server (see ``ai.rs::AiService``): the Rust side
submits tasks via ``POST /segment`` and polls ``GET /task/{id}`` until
the task is complete, at which point the response contains a base64
encoded ``uint8`` mask ready to be uploaded to the GPU.

The mask produced here is intentionally minimal: only the spine is
kept (all vertebrae collapsed into a single label value ``1``). See
``inference.py`` for the post-processing logic.
"""
