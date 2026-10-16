## MODIFIED Requirements
### Requirement: Error Recovery in Application Layer
Application-level errors SHALL be handled gracefully without crashing and SHALL preserve a valid application state. Server connectivity errors SHALL be distinguished from data processing errors.

#### Scenario: Invalid file load
- **WHEN** a file load fails due to invalid input or corruption
- **THEN** the application SHALL show a clear, actionable error
- **AND** the application SHALL remain usable without restart

#### Scenario: State corruption detected
- **WHEN** application state validation detects invalid invariants
- **THEN** the application SHALL prevent unsafe operations
- **AND** the error SHALL be logged with enough detail to diagnose the issue

#### Scenario: Server connection failure
- **WHEN** the WASM client cannot connect to the backend server
- **THEN** the application SHALL display a connection error message with retry option
- **AND** the rendering canvas SHALL remain functional with any previously loaded data
- **AND** the application SHALL auto-retry connection every 5 seconds up to 3 attempts

## ADDED Requirements
### Requirement: Backend Data Loading
The WASM application SHALL load CT volume data from the backend server via HTTP API instead of parsing DICOM files locally.

#### Scenario: Load Volume from Backend
- **WHEN** the user selects a volume from the server
- **THEN** the application SHALL fetch volume metadata from GET /api/volumes/{id}
- **AND** SHALL fetch voxel data from POST /api/volumes/{id}/stream
- **AND** SHALL create a GPU texture from the received voxel data
- **AND** SHALL update the rendering pipeline with the new volume

#### Scenario: Handle Large Volume Loading
- **WHEN** the volume data exceeds WASM memory limits
- **THEN** the application SHALL request downsampled data via the downsample query parameter
- **AND** SHALL inform the user that the volume was loaded at reduced resolution

### Requirement: Server-Aware Application State
The WASM application SHALL maintain awareness of the backend server connection state and adapt its data loading and export behavior accordingly.

#### Scenario: Server State Management
- **WHEN** the application starts
- **THEN** it SHALL check server connectivity via GET /api/health
- **AND** SHALL display a server status indicator in the UI
- **AND** SHALL disable data operations when the server is unavailable

#### Scenario: Reconnect After Server Restart
- **WHEN** the server becomes available after being offline
- **THEN** the application SHALL automatically reconnect
- **AND** SHALL re-fetch the volume list
- **AND** SHALL notify the user that the connection was restored
