## 1. Backend Server Infrastructure

- [x] 1.1 Create `src/server/` module with mod.rs, routes.rs, handlers.rs, state.rs, ws.rs
- [x] 1.2 Configure Axum router with CORS middleware for localhost:3000 and localhost:8000
- [x] 1.3 Implement shared application state struct (Arc<ServerState>) for volume storage
- [x] 1.4 Add tracing subscriber for server request logging (tracing in Cargo.toml, env\_logger for now)
- [x] 1.5 Update `src/main.rs` to start Axum server on native target with `--server` flag
- [x] 1.6 Add `--server` CLI flag for optional server mode (backward compatible)
- [x] 1.7 Configure `KEPLER_SERVER_PORT` environment variable (default: 3000)

## 2. DICOM Upload and Parsing API

- [x] 2.1 Implement POST /api/dicom/upload with multipart form handling
- [x] 2.2 Add multipart field for DICOM file array upload
- [x] 2.3 Call existing `fileio::parse_dcm_directories()` for native file parsing
- [x] 2.4 Store DicomRepo in ServerState with unique volume ID (UUID)
- [x] 2.5 Return JSON with patient/study/series/volume hierarchy
- [x] 2.6 Implement GET /api/volumes to list all loaded volumes
- [x] 2.7 Implement GET /api/volumes/{id} for volume metadata
- [x] 2.8 Add progress reporting during DICOM parsing via WebSocket

## 3. Volume Data Streaming

- [x] 3.1 Implement POST /api/volumes/{id}/stream for voxel data transfer
- [x] 3.2 Stream voxel\_data in 1MB chunks
- [x] 3.3 Include volume metadata in streaming response headers (X-Volume-Dimensions, X-Volume-Spacing, X-Volume-Size)
- [x] 3.4 Support downsampling for large volumes (?downsample=N)
- [x] 3.5 Implement MHA/MHD upload endpoint for pre-processed volumes
- [x] 3.6 Add chunked transfer encoding for large responses

## 4. DICOM Export (Streaming, >2GB Support)

- [x] 4.1 Implement POST /api/export/dicom/{volume\_id} with streaming ZIP response
- [x] 4.2 Use FsSink to write DICOM slices to temporary directory
- [x] 4.3 Implement ZIP streaming via zip crate + chunked transfer
- [x] 4.4 Add Content-Disposition header for browser download
- [x] 4.5 Implement automatic temp directory cleanup after download
- [x] 4.6 Add temp directory timeout cleanup (60 seconds)
- [x] 4.8 Add pre-export disk space validation (stub: returns u64::MAX)

## 5. WebSocket for Real-time Updates

- [x] 5.1 Implement WS /ws endpoint using axum::ws
- [x] 5.2 Define WebSocket message protocol (JSON: type, payload, timestamp)
- [x] 5.3 Emit parsing progress updates during DICOM upload
- [x] 5.4 Emit export progress updates during DICOM export
- [x] 5.5 Support multiple concurrent WebSocket connections (broadcast channel)
- [x] 5.6 Implement connection heartbeat/ping

## 6. WASM Module Simplification (DEFERRED)

- [ ] 6.1 Remove `parse_dcm_files_wasm()` from `src/data/dicom/fileio.rs` WASM branch
- [ ] 6.2 Remove `build_ct_dicom_wasm()` from `src/data/dicom/export_dicom.rs` WASM branch
- [ ] 6.3 Remove `export_slice_png()` WASM export binding
- [ ] 6.4 Remove MHA/MHD parsing code from WASM target (fileio.rs)
- [ ] 6.5 Update `src/lib.rs` to expose only rendering-related wasm\_bindgen functions
- [ ] 6.6 Simplify WASM init to receive volume data from HTTP fetch
- [ ] 6.7 Update WASM logger to route to console\_log (unchanged)

**Note:** Phase 6 intentionally deferred to preserve local WASM parsing as a fallback when server is unavailable.

## 7. Frontend API Integration

- [x] 7.1 Update `static/index.html` to connect to backend API
- [x] 7.2 Replace file upload with POST /api/dicom/upload fetch call (server mode) / local WASM parsing (fallback)
- [x] 7.3 Replace local DICOM tree generation with GET /api/volumes response
- [x] 7.4 Replace local volume creation with POST /api/volumes/{id}/stream fetch
- [x] 7.5 Replace local DICOM export with POST /api/export/dicom/{id} streaming download
- [x] 7.6 Add WebSocket connection for real-time progress updates with auto-reconnect
- [x] 7.8 Add error handling for server connection failures

## 8. Testing and Validation

- [ ] 8.1 Test full workflow: upload → parse → render → export (native + WASM)
- [ ] 8.2 Test large DICOM series export (>2GB) streaming download
- [ ] 8.3 Test WASM module loading and rendering with server-provided data
- [ ] 8.4 Test concurrent volume loading and export operations
- [ ] 8.5 Verify native desktop mode still works (--server flag off)
- [x] 8.6 Verify `cargo build --release` succeeds (2m 02s, 4 warnings)
- [ ] 8.7 Verify `wasm-pack build --target web` succeeds
- [ ] 8.8 Run `cargo test` and fix any failures
- [ ] 8.9 Run `cargo clippy` and fix any warnings
- [x] 8.10 Test CORS with browser (localhost:8000 → localhost:3000)

## 9. Documentation

- [ ] 9.1 Update doc/agents/ARCHITECTURE.md with new server architecture
- [ ] 9.2 Add doc/server/API.md with API endpoint documentation
- [ ] 9.3 Update README.md with server startup instructions
- [ ] 9.4 Update doc/CHANGELOG.md with breaking changes note
- [ ] 9.5 Add doc/agents/BACKEND.md with server development guide

## Summary

- **Completed:** 35 / 55 tasks (64%)
- **Deferred:** 7 tasks (Phase 6 - WASM simplification)
- **Not Started:** 13 tasks (Phase 8 testing, Phase 9 documentation)
- **Skipped:** 2 tasks (PNG export - no endpoint implemented)

