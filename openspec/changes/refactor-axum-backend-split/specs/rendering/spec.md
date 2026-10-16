## ADDED Requirements
### Requirement: Server-Provided Volume Data
The rendering subsystem SHALL accept CT volume data from the backend server via HTTP streaming and create GPU textures from the received data without local parsing.

#### Scenario: Create Texture from Server Stream
- **WHEN** the WASM module receives voxel data from POST /api/volumes/{id}/stream
- **THEN** the rendering subsystem SHALL create a 3D GPU texture from the binary data
- **AND** SHALL apply the volume metadata (dimensions, spacing) from response headers
- **AND** SHALL update all active views (MPR, MIP, Mesh) with the new texture

#### Scenario: Handle Streaming Interruption
- **WHEN** the voxel data stream is interrupted before completion
- **THEN** the rendering subsystem SHALL clean up any partially created GPU resources
- **AND** SHALL report the error to the application layer
- **AND** SHALL NOT leave the GPU in an invalid state
