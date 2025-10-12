OBJ Loading and Rendering Architecture (High-Level Design)

Overview

This document outlines a high-level architecture to read Wavefront OBJ meshes and render them using the existing kepler2-wgpu display pipeline. The design adds an OBJ parser module, an intermediate mesh representation (MeshData), a conversion adapter (MeshAdapter) to the existing vertex format, and an integration layer that plugs into MeshView/BasicMeshContext without altering current rendering subsystems.

Goals and Constraints

- Seamless integration with existing rendering and view components (MeshView, BasicMeshContext, shaders, buffers).
- Non-invasive: no breaking changes to current APIs; introduced modules should be additive.
- Robust error handling: clear, actionable errors when parsing fails or data is incompatible.
- Performance-aware: support large OBJ files with streaming-friendly parsing and efficient memory usage.
- Cross-target compatibility: native and WASM builds should follow the same API and behavior.
- Incremental scope: start with core OBJ features (v, vt, vn, f triangles), add materials (MTL) and advanced features later.

Architecture Components

1) OBJ Parser Module

- Responsibilities:
  - Parse minimal OBJ features: vertices (v), texture coordinates (vt), normals (vn), faces (f) with triangles and safe handling of quads (converted to triangles).
  - Handle separate indices for v/vt/vn (OBJ uses triplets like v/vt/vn).
  - Support multiple objects/groups to enable submesh segmentation.
  - Defer material (usemtl/mtllib) support to a subsequent phase, with stub hooks for later integration.

- Behavior:
  - Streaming/line-based parsing to avoid loading whole files into memory when not necessary.
  - Validate index ranges and face topology; reject polygons with invalid references.
  - Normalize whitespace; ignore comments (#); tolerate mixed ordering of statements.

- Output:
  - A MeshData structure (see below) that captures positions, normals, uvs, indices, and optional submesh/group segmentation.

2) Intermediate Representation: MeshData

- Structure:
  - positions: Vec<[f32; 3]>
  - normals: Option<Vec<[f32; 3]>>
  - uvs: Option<Vec<[f32; 2]>>
  - indices: Vec<u32>  // unified, triangle list
  - vertex_triplets: Option<Vec<(u32, Option<u32>, Option<u32>)>> // original OBJ triplets for advanced merging
  - groups: Vec<Submesh>
  - materials: Option<Vec<MaterialRef>> // placeholder for later MTL integration
  - metadata: MeshMeta { name, source_path, scale_hint, winding, coordinate_convention }

- Submesh:
  - name: String
  - index_range: (start: u32, count: u32)
  - material: Option<MaterialRef>

- Rationale:
  - Decouples parser specifics from rendering requirements.
  - Enables flexible conversion policies (e.g., vertex duplication strategies, tangent computation).

3) Conversion Adapter: MeshAdapter

- Responsibilities:
  - Convert MeshData into the vertex/index buffers expected by the existing mesh pipeline.
  - Apply vertex duplication when the same position requires distinct normals/uvs per face (common in OBJ). This mirrors the unit_cube’s 24-vertex approach: create unique vertices per face/corner to ensure correct face shading and texture seams.
  - Ensure correct winding order (likely counter-clockwise) to match current backface culling.
  - Compute missing data when feasible:
    - If normals are absent, compute per-face or per-vertex normals as specified by a policy.
    - Optionally compute tangents/bitangents if/when the shader pipeline supports them.

- Output:
  - Mesh (or the project’s concrete vertex format and index buffer), ready for upload via BasicMeshContext.

4) Integration Layer

- Loader API:
  - High-level convenience function to load and render an OBJ with minimal code changes.

  Example API (sketch):

  ```rust
  // Synchronous native; async-aware for WASM via futures.
  pub struct ObjLoadOptions {
      pub compute_missing_normals: bool,
      pub triangulate_quads: bool,
      pub deduplicate_vertices: bool, // when safe; default off for face-accurate shading
      pub scale: Option<f32>,
      pub coordinate_space: CoordinateSpace, // e.g., Y-up, Z-forward
  }

  pub fn load_obj_to_mesh(
      path: &std::path::Path,
      device: &wgpu::Device,
      queue: &wgpu::Queue,
      options: &ObjLoadOptions,
  ) -> Result<MeshHandle, CoreError>;

  // Alternatively: return Mesh and let caller push into BasicMeshContext.
  pub fn load_obj_mesh(
      path: &std::path::Path,
      options: &ObjLoadOptions,
  ) -> Result<Mesh, CoreError>;
  ```

- Placement:
  - Parser: src/rendering/mesh/obj_parser.rs
  - MeshData: src/rendering/mesh/mesh_data.rs
  - MeshAdapter: src/rendering/mesh/mesh_adapter.rs
  - Loader/Integration: src/rendering/mesh/obj_loader.rs

- Usage:
  - Existing views (MeshView) and contexts (BasicMeshContext) remain unchanged; the loader produces Mesh/handles that those systems already understand.

5) Error Handling

- Error Types:
  - ObjParseError: syntax errors, invalid indices, unsupported features (with helpful messages and line numbers).
  - MeshConversionError: issues encountered when converting MeshData to Mesh (e.g., overflow, invalid winding).
  - IoError: file access failures, not found, permission denied.
  - ValidationError: detected inconsistencies (e.g., empty geometry, degenerate triangles).

- Strategy:
  - Return rich errors with context (file path, line number, token).
  - Provide recovery suggestions (e.g., enable triangulate_quads, compute_missing_normals).
  - Maintain parity across native/WASM (surface errors via Result and log diagnostics).

6) Performance Optimizations

- Parsing:
  - Line-by-line streaming to minimize peak memory.
  - Avoid temporary allocations by reusing buffers and reserving capacity.
  - Fast path for common patterns (triangles, grouped sections).

- Conversion:
  - Pre-allocate vertex/index buffers based on counts.
  - Optional deduplication of vertex triplets using hash maps when requested.
  - Batched buffer uploads with staging to minimize WGPU submission overhead.

- Rendering:
  - Maintain submesh segmentation to allow selective rendering and frustum culling.
  - Consider instancing for repeated objects in future iterations.

7) Compatibility Plan

- Coordinate conventions:
  - Normalize to the project’s standard (document expected up-axis and forward-axis). Provide options to reorient if OBJ assumes different axes.

- Winding order and culling:
  - Consistent triangle winding to match current culling settings; adapter enforces orientation.

- Shader expectations:
  - Provide vertex attributes that the current mesh shader pipeline requires: position, normal, uv, color (if used), tangent (optional).

- Buffer lifecycle:
  - Use existing buffer creation and update paths to ensure compatibility with the current lifecycle (aligns with previous fixes documented in buffer-lifecycle-fix.md).

8) Testing Plan

- Unit tests (parser):
  - v/vt/vn parsing, face triplets, triangulation of quads, invalid index handling.

- Unit tests (adapter):
  - Vertex duplication policy correctness, normal computation (when missing), winding enforcement.

- Integration tests:
  - Load small sample OBJs (cube, uv-mapped quad, smooth sphere) and render via MeshView.
  - Verify vertex counts align with adapter policy (e.g., cube -> 24 vertices).
  - Cross-target: run on native and WASM targets.

- Performance tests:
  - Large OBJ file parsing time and memory profile.

9) Documentation and Examples

- Provide a minimal example demonstrating usage:

  ```rust
  let options = ObjLoadOptions {
      compute_missing_normals: true,
      triangulate_quads: true,
      deduplicate_vertices: false,
      scale: None,
      coordinate_space: CoordinateSpace::default(),
  };

  let mesh = load_obj_mesh(std::path::Path::new("assets/models/cube.obj"), &options)?;
  let handle = BasicMeshContext::upload_mesh(&device, &queue, &mesh)?;
  mesh_view.add_mesh(handle);
  ```

API Sketch (Detailed)

```rust
#[derive(Clone, Copy)]
pub enum CoordinateSpace { YUpZForward, ZUpYForward, Custom }

#[derive(Default)]
pub struct MeshMeta {
    pub name: Option<String>,
    pub source_path: Option<String>,
    pub scale_hint: Option<f32>,
    pub winding: Option<Winding>,
    pub coordinate_convention: Option<CoordinateSpace>,
}

pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Option<Vec<[f32; 3]>>,
    pub uvs: Option<Vec<[f32; 2]>>,
    pub indices: Vec<u32>,
    pub vertex_triplets: Option<Vec<(u32, Option<u32>, Option<u32>)>>,
    pub groups: Vec<Submesh>,
    pub materials: Option<Vec<MaterialRef>>,
    pub meta: MeshMeta,
}

pub struct Submesh {
    pub name: String,
    pub index_range: (u32, u32),
    pub material: Option<MaterialRef>,
}

pub struct MaterialRef {
    pub name: String,
}

pub enum ObjParseErrorKind {
    SyntaxError,
    InvalidIndex,
    UnsupportedFeature,
}

pub struct ObjParseError {
    pub kind: ObjParseErrorKind,
    pub message: String,
    pub line: Option<usize>,
}

pub fn parse_obj<R: std::io::BufRead>(reader: R) -> Result<MeshData, ObjParseError>;

pub enum MeshConversionErrorKind { InvalidWinding, Overflow, MissingPositions }
pub struct MeshConversionError { pub kind: MeshConversionErrorKind, pub message: String }

pub fn mesh_data_to_mesh(data: &MeshData, policy: &ObjLoadOptions) -> Result<Mesh, MeshConversionError>;
```

File Placement Plan

- src/rendering/mesh/obj_parser.rs: Line-by-line parser implementation.
- src/rendering/mesh/mesh_data.rs: MeshData and related types.
- src/rendering/mesh/mesh_adapter.rs: Conversion to existing Mesh/vertex format.
- src/rendering/mesh/obj_loader.rs: Public APIs, convenience functions, and integration glue.
- doc/obj-loading-and-rendering-architecture.md: This design document.

Future Enhancements

- Material (MTL) support: parse mtllib/usemtl, load textures, bind materials to submeshes.
- Smoothing groups: compute vertex normals according to group boundaries.
- Tangent space generation: for normal mapping when shaders support it.
- Caching: persist converted MeshData/Mesh for faster reloads.
- Partial/async loading: progressively display large scenes.

Adoption Strategy

- Phase 1: Implement minimal parser and adapter; render small sample OBJs.
- Phase 2: Add error robustness, performance optimizations, and tests.
- Phase 3: Integrate materials and advanced features (smoothing, tangents, caching).

Notes on Compatibility with Existing Display Systems

- The adapter preserves the project’s existing face-wise vertex duplication behavior (e.g., 24 vertices for a cube) to maintain correct lighting and shading.
- Buffer creation and upload mirror current lifecycle practices to avoid regressions.
- Coordinate system and winding order are enforced at conversion time to align with current shader expectations.