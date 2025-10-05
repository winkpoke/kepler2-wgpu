# 3D Mesh Implementation Plan (Minimal, Non-breaking, Sequential)

This document captures a minimal-impact, opt-in implementation plan to introduce 3D mesh rendering alongside the existing 2D MPR workflows. Each step is small in scope, maintains current functionality, avoids breaking changes, and explains purpose and stability considerations.

Source design reference: <mcfile name="3d-mesh-design.md" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\doc\\3d-mesh-design.md"></mcfile>

---

1) Add a feature gate to isolate mesh-related code
- Change: Define a Cargo feature named "mesh" and guard new modules with cfg(feature = "mesh").
- Files: <mcfile name="Cargo.toml" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\Cargo.toml"></mcfile>
- Purpose: Ensures mesh is opt-in and cannot affect builds or runtime unless enabled.
- Stability: Default build remains unchanged; zero runtime impact unless feature is enabled.
- Validation: Build without the feature; confirm identical behavior.

2) Scaffold mesh module with inert data structures
- Change: Add new module files with basic types only: Mesh, MeshVertex, Material, Camera, Lighting; no rendering logic.
- Files: <mcfile name="mod.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mod.rs"></mcfile> <mcfile name="mesh.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mesh.rs"></mcfile> <mcfile name="material.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\material.rs"></mcfile> <mcfile name="camera.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\camera.rs"></mcfile> <mcfile name="lighting.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\lighting.rs"></mcfile>
- Purpose: Establishes type system for meshes without touching existing render paths.
- Stability: No imports from existing code; all behind feature gate.
- Validation: Build with and without feature; both pass without behavior changes.

3) Add placeholder mesh shaders without integrating them
- Change: Create WGSL shader files (mesh.wgsl, mesh_depth.wgsl) but do not load or compile them yet.
- Files: <mcfile name="mesh.wgsl" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\shader\\mesh.wgsl"></mcfile> <mcfile name="mesh_depth.wgsl" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\shader\\mesh_depth.wgsl"></mcfile>
- Purpose: Prepare assets for 3D pipeline; no impact on current shaders.
- Stability: Not referenced; zero runtime effect.
- Validation: Run app; ensure no file access or shader compilation changes.

4) Introduce MeshView implementing View as a no-op
- Change: Add MeshView with stub methods that do not render or allocate GPU resources.
- Files: <mcfile name="mesh_view.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mesh_view.rs"></mcfile> <mcfile name="view.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\view.rs"></mcfile>
- Purpose: Integrates MeshView into view system contract while keeping behavior inert.
- Stability: Strictly behind feature gate; existing views unchanged.
- Validation: Build both ways; verify no rendering changes.

5) Export MeshView conditionally in view module
- Change: Update view/mod.rs to pub use MeshView only under the feature gate.
- Files: <mcfile name="mod.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\mod.rs"></mcfile>
- Purpose: Allows consumers to access MeshView when opting in; avoids affecting defaults.
- Stability: Default exports unchanged.
- Validation: Build; confirm existing module exports intact.

6) Create MeshRenderContext separate from 2D RenderContext (inert)
- Change: Add mesh_render_context.rs with device/queue references; do not create pipelines yet.
- Files: <mcfile name="render_context.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\render_context.rs"></mcfile> <mcfile name="mesh_render_context.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mesh_render_context.rs"></mcfile>
- Purpose: Isolates 3D state from 2D slice context to prevent interference.
- Stability: Unreferenced unless feature enabled; no side effects.
- Validation: Build ok; no allocations or calls without feature.

7) Prepare optional depth texture helper for 3D pass (unused)
- Change: Add a helper function to create Depth32Float textures.
- Files: <mcfile name="texture.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\texture.rs"></mcfile>
- Purpose: Readies shared depth resource creation for mesh rendering.
- Stability: Do not integrate or call from existing views.
- Validation: Ensure no change to current texture creation paths.

8) Add test cube generator utility (unreferenced)
- Change: Provide a function that returns vertices/indices for a unit cube; no GPU uploads yet.
- Files: <mcfile name="mesh.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mesh.rs"></mcfile>
- Purpose: Enables future validation without external assets.
- Stability: Only compiled under feature; no runtime effect.
- Validation: Build passes; outputs unchanged.

9) Add non-invasive configuration plumbing (default off)
- Change: Introduce enable_mesh: bool in app state; only instantiate MeshView if true.
- Files: <mcfile name="state.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\state.rs"></mcfile> <mcfile name="render_app.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\render_app.rs"></mcfile>
- Purpose: Controlled activation to preserve default behavior and allow gradual testing.
- Stability: Default false; no MeshView instances created by default.
- Validation: Start app; verify identical layout and rendering.

10) Wire MeshView into layout system (opt-in only)
- Change: Allow layout builder to accept MeshView when enable_mesh is true; do not alter default layouts.
- Files: <mcfile name="layout.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\layout.rs"></mcfile>
- Purpose: Introduces MeshView into layout graph in a strictly opt-in way.
- Stability: Existing layouts remain unchanged.
- Validation: Confirm current layouts render as before when disabled.

11) Implement mesh pipeline creation (guarded, default disabled)
- Change: In MeshRenderContext, create mesh pipeline using mesh.wgsl and depth; only instantiate when enable_mesh.
- Files: <mcfile name="mesh_render_context.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mesh_render_context.rs"></mcfile> <mcfile name="mesh.wgsl" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\shader\\mesh.wgsl"></mcfile>
- Purpose: Establishes 3D pipeline ready for rendering; remains inactive by default.
- Stability: Separate pipeline and pass from 2D; no interference.
- Validation: Build with feature enabled and enable_mesh false; ensure pipeline creation is skipped.

12) Add separate mesh render pass before 2D slice pass (only when enabled)
- Change: In render loop, execute mesh pass with depth testing only if MeshView exists; keep 2D pass unchanged.
- Files: <mcfile name="render_app.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\render_app.rs"></mcfile> <mcfile name="shader_tex.wgsl" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\shader\\shader_tex.wgsl"></mcfile>
- Purpose: Implements recommended separate passes strategy.
- Stability: Execution path identical unless enabled; depth resources isolated.
- Validation: With enable_mesh false, confirm identical rendering; with true, ensure 2D MPR renders after mesh pass.

13) Add basic camera control and matrices for MeshView
- Change: Implement camera view/projection matrices; reuse existing math utilities.
- Files: <mcfile name="coord.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\coord.rs"></mcfile> <mcfile name="camera.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\camera.rs"></mcfile>
- Purpose: Enables controlled mesh visualization when activated.
- Stability: Scoped to MeshView; no effect on 2D transforms.
- Validation: Enable mesh, draw test cube, verify camera movement does not affect 2D views.

14) Preserve public API stability (no signature changes)
- Change: Avoid modifications to existing View trait signatures and RenderContext types; keep shared code untouched.
- Files: <mcfile name="view.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\view.rs"></mcfile> <mcfile name="render_context.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\render_context.rs"></mcfile>
- Purpose: Guarantees backward compatibility and stability throughout.
- Stability: Existing code paths remain untouched.
- Validation: Run current MPR workflows; confirm identical behavior and performance.

---

Notes on enabling later:
- Build-time: enable Cargo feature `mesh`.
- Runtime: set `enable_mesh = true` in app state to instantiate MeshView and run the mesh pass.