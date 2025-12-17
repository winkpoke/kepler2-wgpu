# rendering Specification

## Purpose
Defines the rendering subsystem architecture, ensuring a clean, streamlined mesh rendering pipeline with fully configurable lighting and minimal overhead.
## Requirements
### Requirement: Clean Architecture for Mesh Rendering
The mesh rendering subsystem MUST provide a streamlined, single-context architecture without dead code or unused abstractions.

#### Scenario: Basic Rendering
Given a simple mesh (e.g., a cube)
When the application initializes the `MeshView`
Then it should use `BasicMeshContext` exclusively
And no `MeshRenderContext` should be instantiated or compiled.

### Requirement: Configurable Lighting
The lighting system MUST be fully configurable via the `Lighting` struct without hardcoded fallbacks in the conversion logic.

#### Scenario: Custom Lighting
Given a `Lighting` configuration with red ambient light
When the uniforms are generated via `to_basic_uniforms()`
Then the output `BasicLightingUniforms` should contain the red ambient light value
And not the hardcoded default.

