# rendering Specification

## Purpose
TBD - created by archiving change refactor-mesh-rendering-cleanup. Update Purpose after archive.
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

