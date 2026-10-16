# Change: Refactor Axum Backend Split

## Why

The current WASM build embeds DICOM parsing, MHA/MHD reading, CT volume generation, and DICOM export logic, causing WASM binary bloat and hitting the ~2GB memory ceiling for large CT exports. By offloading all data processing to an Axum backend and keeping WASM focused solely on GPU rendering, we eliminate the memory bottleneck, enable streaming export of arbitrarily large DICOM series, and produce a smaller, faster-loading WASM module.

## What Changes

- **ADD** Axum-based backend server with REST API and WebSocket support for data operations
- **ADD** HTTP endpoints for DICOM upload, CT volume retrieval, metadata queries, and streaming DICOM export (supports files >2GB)
- **ADD** WebSocket channel for progress tracking and real-time status updates
- **MODIFY** WASM module to remove DICOM parsing, MHA/MHD reading, and export logic; WASM becomes a pure rendering client
- **MODIFY** data flow: frontend fetches processed CTVolume from backend instead of parsing locally
- **MODIFY** native entry point to start Axum server alongside optional native window
- **ADD** streaming DICOM export via HTTP chunked transfer using FsSink (bypasses WASM memory limits)
- **MODIFY** existing `DicomSink` trait usage to support both backend (FsSink) and legacy WASM (MemSink) paths

## Impact

- Affected specs:
  - `application` - WASM app simplifies to rendering-only; data loading via HTTP
  - `rendering` - unchanged GPU code; receives volume data from HTTP instead of local parsing
  - `backend-api` (NEW) - REST API, WebSocket, file upload, DICOM export
  - `data-processing` (NEW) - DICOM parsing, CT volume generation, export pipeline

- Affected code:
  - `src/data/dicom/` - WASM branches removed; extracted to backend crate
  - `src/data/medical_imaging/` - WASM branches removed; extracted to backend crate
  - `src/data/export_dicom.rs` - backend streaming export handler
  - `src/lib.rs` - WASM module simplified; wasm_bindgen exports reduced
  - `src/main.rs` - native entry point starts Axum server
  - `src/application/app.rs` - simplified data loading path
  - `static/index_image.html` - frontend connects to backend API
  - New: `src/server/` - Axum routes, handlers, middleware

- **BREAKING**: WASM module no longer exposes `parse_dcm_files_wasm()`, `build_ct_dicom_wasm()`, or `export_slice_png()` WASM bindings. These operations move to backend HTTP API.
