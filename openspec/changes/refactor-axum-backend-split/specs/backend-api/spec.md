## ADDED Requirements
### Requirement: Backend Server
The system SHALL run an Axum-based HTTP server on native targets to serve data processing operations, enabling WASM clients to offload DICOM parsing and export to the backend.

#### Scenario: Server Startup on Native Target
- **WHEN** the native binary is executed
- **THEN** an Axum HTTP server SHALL start on the configured port (default 3000)
- **AND** the server SHALL be accessible via http://localhost:{port}
- **AND** the server SHALL run concurrently with the native window if --server flag is provided

#### Scenario: Server Port Configuration
- **WHEN** KEPLER_SERVER_PORT environment variable is set
- **THEN** the server SHALL bind to the specified port
- **AND** if the port is unavailable, the server SHALL exit with a clear error message

### Requirement: DICOM Upload API
The system SHALL provide a REST API endpoint for uploading DICOM files or directories, parsing them into a DicomRepo, and returning the patient/study/series tree structure.

#### Scenario: Upload DICOM Directory
- **WHEN** a client sends POST /api/dicom/upload with multipart DICOM files
- **THEN** the server SHALL parse the files using fileio::parse_dcm_directories()
- **AND** SHALL store the DicomRepo in ServerState with a unique UUID
- **AND** SHALL return JSON with patient/study/series/volume hierarchy
- **AND** SHALL respond within reasonable time for typical CT series (512x512, 500 slices)

#### Scenario: Upload MHA/MHD Volume
- **WHEN** a client sends POST /api/volumes/upload with MHA/MHD files
- **THEN** the server SHALL parse the volume using existing MhdParser/MhaParser
- **AND** SHALL store the CTVolume in ServerState
- **AND** SHALL return the volume ID and metadata

#### Scenario: Invalid DICOM Upload
- **WHEN** uploaded files are not valid DICOM
- **THEN** the server SHALL return HTTP 400 with error details
- **AND** SHALL not store any partial data in ServerState

### Requirement: Volume Data Streaming API
The system SHALL provide a streaming endpoint to transfer voxel data from server to WASM client in chunks, enabling efficient loading of large CT volumes without memory constraints.

#### Scenario: Stream Volume Voxel Data
- **WHEN** a client requests POST /api/volumes/{id}/stream
- **THEN** the server SHALL stream the voxel_data in configurable chunks (default 1MB)
- **AND** SHALL include volume metadata (dimensions, spacing, orientation) in response headers
- **AND** SHALL use chunked transfer encoding

#### Scenario: Stream with Downsampling
- **WHEN** a client requests POST /api/volumes/{id}/stream?downsample=2
- **THEN** the server SHALL reduce volume resolution by the specified factor
- **AND** SHALL return downsampled voxel data with updated metadata

### Requirement: DICOM Export API (Streaming, >2GB)
The system SHALL provide a streaming export endpoint that generates DICOM series from a loaded CT volume and streams them as a ZIP file, supporting exports exceeding 2GB by using disk-based temporary storage and HTTP streaming.

#### Scenario: Export Large DICOM Series (>2GB)
- **WHEN** a client requests POST /api/export/dicom/{volume_id}
- **THEN** the server SHALL use FsSink to write DICOM slices to a temporary directory
- **AND** SHALL stream the resulting ZIP file via HTTP chunked transfer
- **AND** SHALL set Content-Disposition: attachment; filename="dicoms.zip"
- **AND** SHALL clean up the temporary directory after download completes

#### Scenario: Export with Progress Reporting
- **WHEN** a DICOM export is in progress
- **THEN** the server SHALL send progress updates via WebSocket (type: "export_progress")
- **AND** progress SHALL include current slice count and total slice count

#### Scenario: Export Pre-validation
- **WHEN** a client requests DICOM export
- **THEN** the server SHALL check available disk space in temp directory
- **AND** SHALL reject the request with HTTP 507 if insufficient space
- **AND** SHALL estimate required space based on volume dimensions

#### Scenario: Temporary Directory Cleanup
- **WHEN** a download completes or client disconnects
- **THEN** the server SHALL delete the temporary directory
- **AND** SHALL enforce a 30-minute timeout for orphaned temp directories

### Requirement: WebSocket Event Channel
The system SHALL provide a WebSocket endpoint for real-time server events, including parsing progress, export progress, and error notifications.

#### Scenario: Connect to WebSocket
- **WHEN** a client opens a WebSocket connection to /ws
- **THEN** the server SHALL accept the connection
- **AND** SHALL respond to ping messages with pong
- **AND** SHALL send server status updates as JSON messages

#### Scenario: Receive Parsing Progress
- **WHEN** DICOM files are being parsed
- **THEN** the server SHALL send WebSocket messages with type "parsing_progress"
- **AND** message SHALL include { current_file, total_files, volume_id }

### Requirement: CORS Support
The system SHALL configure Cross-Origin Resource Sharing to allow browser-based WASM clients to communicate with the backend server during development and production.

#### Scenario: Browser CORS Preflight
- **WHEN** a browser sends an OPTIONS preflight request from localhost:8000
- **THEN** the server SHALL respond with appropriate CORS headers
- **AND** SHALL allow methods: GET, POST, PUT, DELETE
- **AND** SHALL allow headers: Content-Type, Authorization

### Requirement: Health Check
The system SHALL provide a health check endpoint for monitoring server status and readiness.

#### Scenario: Server Health Check
- **WHEN** a client sends GET /api/health
- **THEN** the server SHALL return HTTP 200 with JSON status
- **AND** response SHALL include { status: "ok", loaded_volumes: N, uptime: "..." }
