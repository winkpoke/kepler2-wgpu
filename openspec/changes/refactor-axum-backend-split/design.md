# Design: Axum Backend Split

## Context

Kepler2-WGPU currently bundles DICOM parsing, CT volume generation, and DICOM export into a single WASM module. WASM has ~2GB memory limits, making large CT series export (>2GB) impossible. The WASM binary is also bloated with parsing code that is only useful during data loading, not during rendering.

The user wants to split the architecture so that:
- **Axum backend** handles: DICOM parsing, MHA/MHD reading, CT volume generation, DICOM export
- **WASM module** handles: GPU rendering only (MPR, MIP, 3D mesh views)

## Goals

- Enable DICOM export of arbitrarily large CT series (no 2GB limit)
- Reduce WASM binary size by removing parsing logic
- Maintain backward compatibility for native (desktop) usage
- Keep both native and WASM targets buildable
- Provide streaming download for large exports

## Non-Goals

- Database integration (volumes stored in memory/temp files only)
- Authentication or multi-user support (single-user local deployment)
- Cloud storage or S3 integration
- Real-time DICOM streaming from PACS

## Decisions

### Decision 1: Single Binary with Dual Mode

The native binary (`kepler`) will run both the Axum server and optionally the native window. The WASM build remains a separate target.

**Rationale**: Simpler deployment, shared code between server and native window, no need for separate processes.

**Alternatives considered**:
- Separate binary for server: adds deployment complexity, duplicates dependencies
- Microservices architecture: overkill for single-user medical imaging tool

### Decision 2: REST API for Data Operations + WebSocket for Progress

Data operations (upload, query, export) use REST. Progress updates and server events use WebSocket.

**Rationale**: REST is standard, well-tested, and handles streaming downloads natively. WebSocket provides low-latency push for progress tracking during long operations (e.g., DICOM parsing of 1000+ files).

### Decision 3: Shared Crate Structure

The codebase remains a single Cargo workspace with conditional compilation:
- `src/server/` - Axum routes and handlers (cfg(not(target_arch = "wasm32"))))
- `src/data/` - shared data models and parsers (available to both server and WASM)
- `src/rendering/` - unchanged GPU rendering (WASM only)

**Rationale**: Maintains single codebase, shared types between server and WASM, simpler build process.

### Decision 4: Streaming Export with Temporary Files

DICOM export uses FsSink to write individual slices to a temporary directory, then streams them as a ZIP file over HTTP chunked transfer encoding.

**Rationale**: Avoids loading all DICOM files into memory simultaneously. The temporary directory is cleaned up after the download completes or on timeout.

### Decision 5: Volume Data Transfer Format

CT volume data is transferred from server to WASM as binary chunks via HTTP POST response. The WASM module receives raw voxel data, creates a GPU texture, and renders.

**Rationale**: Direct binary transfer avoids JSON serialization overhead. Chunked transfer allows progressive loading for large volumes.

### Decision 6: Server Port Configuration

Default server port is configurable via environment variable `KEPLER_SERVER_PORT` (default: 3000).

## Data Flow

```
Frontend (Browser)                  Backend (Axum Native)
=================                   =====================

POST /api/dicom/upload
  multipart/form-data              → Parse DICOM files
  (DICOM directory)                  Build DicomRepo
                                     Store in memory (Arc<DicomRepo>)
  ← JSON { patients, studies,       }
       series, volumes }

GET /api/volumes/{id}/metadata
  → Returns: dimensions, spacing,
             orientation, etc.      → Build metadata from CTVolume
  ← JSON metadata                   }

POST /api/volumes/{id}/stream
  → Binary chunk (voxel data)      → Stream voxel_data in chunks
                                     (e.g., 1MB chunks)
  ← application/octet-stream        }
  (WASM creates GPU texture)

POST /api/export/dicom/{volume_id}
  → Progress updates               → Generate DICOM slices (FsSink)
  (via WebSocket)                    to temp directory
  ← application/zip                 → Stream ZIP to client
  (streaming download)              → Cleanup temp dir

GET /api/volumes
  → List all loaded volumes
  ← JSON array
```

## API Endpoints

| Method | Path | Description | Request | Response |
|--------|------|-------------|---------|----------|
| POST | /api/dicom/upload | Upload DICOM files | multipart/form-data | JSON: volume list |
| GET | /api/volumes | List loaded volumes | - | JSON array |
| GET | /api/volumes/{id} | Get volume metadata | - | JSON metadata |
| POST | /api/volumes/{id}/stream | Stream voxel data | - | binary stream |
| POST | /api/export/dicom/{id} | Export DICOM series | JSON: export options | ZIP stream |
| POST | /api/export/png/{id} | Export slice as PNG | JSON: slice params | PNG binary |
| GET | /api/health | Server health check | - | JSON status |
| WS | /ws | WebSocket for events | - | JSON messages |

## Risks / Trade-offs

| Risk | Impact | Mitigation |
|------|--------|------------|
| Large volume transfer time | User waits for texture upload | Chunked transfer with progress bar |
| Temp disk space for export | Disk full during export | Pre-check available space, cleanup on timeout |
| WASM memory still limited | Very large volumes crash WASM | Server-side downsampling, multi-resolution |
| Cross-origin issues in dev | Browser blocks requests | CORS middleware pre-configured |

## Migration Plan

### Phase 1: Backend API Development
- Implement Axum server with all endpoints
- Add streaming DICOM export
- Test with existing DICOM datasets

### Phase 2: WASM Simplification
- Remove WASM-specific parsing code branches
- Update WASM bindings to only expose rendering functions
- Update frontend to use backend API

### Phase 3: Integration
- Update `main.rs` to start server
- Test full workflow: upload → parse → render → export
- Performance validation

### Rollback
- Keep existing WASM parsing code in git history
- Server mode is opt-in via CLI flag (`--server`)
- Original desktop mode remains functional

## Open Questions

1. Should the native window mode be kept alongside server mode, or replaced entirely?
2. What is the maximum concurrent user count expected? (affects memory management)
3. Should export progress use Server-Sent Events instead of WebSocket for simplicity?