# 3D Mesh Implementation Plan (Minimal, Non-breaking, Sequential)

This document captures a minimal-impact, opt-in implementation plan to introduce 3D mesh rendering alongside the existing 2D MPR workflows. Each step is small in scope, maintains current functionality, avoids breaking changes, and explains purpose and stability considerations.

Source design reference: <mcfile name="3d-mesh-design.md" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\doc\\3d-mesh-design.md"></mcfile>

## Current Baseline (as of the current codebase)
- Rendering loop and application orchestration
  - <mcfile name="state.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\state.rs"></mcfile> initializes WGPU (instance, device, queue, surface), manages window resize and update(), and sequences render() for the onscreen frame.
  - <mcfile name="render_app.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\render_app.rs"></mcfile> owns the event loop and delegates to State; receives user events from <mcfile name="gl_canvas.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\gl_canvas.rs"></mcfile>.
- 2D slice pipeline and render context
  - <mcfile name="render_context.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\render_context.rs"></mcfile> defines the 2D pipeline, bind groups, and shader uniforms; uses <mcfile name="shader_tex.wgsl" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\shader\\shader_tex.wgsl"></mcfile> to sample the volume texture and apply window/level.
- Views and layout
  - <mcfile name="view.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\view.rs"></mcfile> provides View and MPRView traits plus GenericMPRView implementation for slice rendering and interactions.
  - <mcfile name="layout.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\layout.rs"></mcfile> positions views using GridLayout and OneCellLayout, calling update() and render() on each view.
- Data and textures
  - <mcfile name="render_content.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\render_content.rs"></mcfile> creates 3D textures from CT data (Rg8Unorm packed path, with optional R16Float when supported).
- Math helpers
  - <mcfile name="coord.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\coord.rs"></mcfile> provides matrix and transform utilities used by views and rendering.
- Status summary
  - Mesh feature gate, initial mesh modules, and placeholder shaders: Status: Implemented (scaffold only).
- Camera, Lighting, MeshView integration, mesh pipeline and render pass: Status: Not Yet Implemented.
  - 2D MPR rendering is the only active pipeline today.

---

1) Add a feature gate to isolate mesh-related code
Status: Implemented
- Change: Defined a Cargo feature named "mesh" and guarded new modules with cfg(feature = "mesh").
- Files: <mcfile name="Cargo.toml" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\Cargo.toml"></mcfile> <mcfile name="lib.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\lib.rs"></mcfile>
- Purpose: Ensures mesh is opt-in and cannot affect builds or runtime unless enabled.
- Stability: Default build remains unchanged; zero runtime impact unless feature is enabled.
- Validation: cargo check (feature OFF) passed; cargo check --features mesh (feature ON) passed; no behavior changes.

2) Scaffold mesh module with inert data structures
Status: Implemented
- Change: Added new module files with basic types only: Mesh, MeshVertex, Material, Camera, Lighting; no rendering logic. Also added MeshView (no-op) and MeshRenderContext (inert) as scaffolding.
- Files: <mcfile name="mod.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mod.rs"></mcfile> <mcfile name="mesh.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mesh.rs"></mcfile> <mcfile name="material.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\material.rs"></mcfile> <mcfile name="camera.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\camera.rs"></mcfile> <mcfile name="lighting.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\lighting.rs"></mcfile> <mcfile name="mesh_view.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mesh_view.rs"></mcfile> <mcfile name="mesh_render_context.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mesh_render_context.rs"></mcfile>
- Purpose: Establishes type system for meshes without touching existing render paths.
- Stability: No imports from existing 2D codepaths; all behind feature gate.
- Validation: Build with and without feature; both pass without behavior changes.

3) Add placeholder mesh shaders without integrating them
Status: Implemented
- Change: Created WGSL shader files (mesh.wgsl, mesh_depth.wgsl) but did not load or compile them yet.
- Files: <mcfile name="mesh.wgsl" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\shader\\mesh.wgsl"></mcfile> <mcfile name="mesh_depth.wgsl" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\shader\\mesh_depth.wgsl"></mcfile>
- Purpose: Prepare assets for 3D pipeline; no impact on current shaders.
- Stability: Not referenced; zero runtime effect.
- Validation: Run app; ensure no file access or shader compilation changes.

4) Introduce MeshView implementing View as a no-op
Status: Implemented
- Change: Added MeshView that implements Renderable and View with inert methods; no rendering or GPU allocations.
- Files: <mcfile name="mesh_view.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mesh_view.rs"></mcfile> <mcfile name="view.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\view.rs"></mcfile>
- Purpose: Integrates MeshView into view system contract while keeping behavior inert.
- Stability: Strictly behind feature gate; existing views unchanged.
- Validation: cargo check --features mesh passed; MeshView remains no-op.

5) Export MeshView conditionally in view module
Status: Implemented
- Change: Updated view/mod.rs to pub use MeshView only under the feature gate.
- Files: <mcfile name="mod.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\mod.rs"></mcfile> <mcfile name="mesh_view.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mesh_view.rs"></mcfile>
- Purpose: Allows consumers to access MeshView when opting in; avoids affecting defaults.
- Stability: Default exports unchanged when feature is OFF.
- Validation: cargo check (feature OFF) passed; cargo check --features mesh (feature ON) passed; existing exports intact.

6) Create MeshRenderContext separate from 2D RenderContext (inert)
Status: Implemented
- Change: Added mesh_render_context.rs with device/queue references; no pipelines created yet.
- Files: <mcfile name="render_context.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\render_context.rs"></mcfile> <mcfile name="mesh_render_context.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mesh_render_context.rs"></mcfile>
- Purpose: Isolates 3D state from 2D slice context to prevent interference.
- Stability: Strictly behind feature gate and currently unreferenced; no side effects.
- Validation: cargo check (feature OFF) passed; cargo check --features mesh (feature ON) passed.

7) Prepare optional depth texture helper for 3D pass (unused)
Status: Not Yet Implemented
- Change: Add a helper function to create Depth32Float textures.
- Files: <mcfile name="texture.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\texture.rs"></mcfile> (to be added)
- Purpose: Readies shared depth resource creation for mesh rendering.
- Stability: Do not integrate or call from existing views.
- Validation: Ensure no change to current texture creation paths.

8) Add test cube generator utility (unreferenced)
Status: Not Yet Implemented
- Change: Provide a function that returns vertices/indices for a unit cube; no GPU uploads yet.
- Files: <mcfile name="mesh.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mesh.rs"></mcfile>
- Purpose: Enables future validation without external assets.
- Stability: Only compiled under feature; no runtime effect.
- Validation: Build passes; outputs unchanged.

9) Add non-invasive Mesh Mode configuration (default off)
Status: Not Yet Implemented
- Change: Introduce mesh_mode_enabled: bool in app state; when true, render MeshView in a full-screen path without modifying existing views or layout.
- Files: <mcfile name="state.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\state.rs"></mcfile> <mcfile name="render_app.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\render_app.rs"></mcfile>
- Purpose: Controlled activation to preserve default behavior and allow gradual testing while keeping the third view unchanged.
- Stability: Default false; no changes to view/layout; render path branches only.
- Validation: Start app; verify identical layout and rendering when disabled; full-screen mesh when enabled.

10) Render MeshView in a separate full-screen path (opt-in only)
Status: Not Yet Implemented
- Change: Do not modify the layout builder. Instead, render MeshView in a dedicated full-screen pass when Mesh Mode is enabled.
- Files: <mcfile name="render_app.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\render_app.rs"></mcfile>
- Purpose: Integrates mesh visualization without altering any view indices or grid cells; the third view remains unchanged.
- Stability: Existing layouts remain unchanged.
- Validation: Confirm current layouts render as before when disabled; full-screen mesh appears when enabled.

11) Implement mesh pipeline creation (guarded, default disabled)
Status: Not Yet Implemented
- Change: In MeshRenderContext, create mesh pipeline using mesh.wgsl and depth; only instantiate when Mesh Mode is enabled.
- Files: <mcfile name="mesh_render_context.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mesh_render_context.rs"></mcfile> <mcfile name="mesh.wgsl" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\shader\\mesh.wgsl"></mcfile>
- Purpose: Establishes 3D pipeline ready for rendering; remains inactive by default.
- Stability: Separate pipeline and pass from 2D; no interference.
- Validation: Build with feature enabled and Mesh Mode disabled; ensure pipeline creation is skipped.

12) Add separate mesh render pass before 2D slice pass (only when enabled)
Status: Not Yet Implemented
- Change: In render loop, execute mesh pass with depth testing only if MeshView exists; keep 2D pass unchanged.
- Files: <mcfile name="render_app.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\render_app.rs"></mcfile> <mcfile name="shader_tex.wgsl" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\shader\\shader_tex.wgsl"></mcfile>
- Purpose: Implements recommended separate passes strategy.
- Stability: Execution path identical unless enabled; depth resources isolated.
- Validation: With Mesh Mode disabled, confirm identical rendering; with enabled, ensure 2D MPR renders after the mesh pass (for multi-pass design) or grid-only when disabled (for full-screen mode).

13) Add basic camera control and matrices for MeshView
Status: Not Yet Implemented
- Change: Implement camera view/projection matrices; reuse existing math utilities.
- Files: <mcfile name="coord.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\coord.rs"></mcfile> <mcfile name="camera.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\camera.rs"></mcfile>
- Purpose: Enables controlled mesh visualization when activated.
- Stability: Scoped to MeshView; no effect on 2D transforms.
- Validation: Enable mesh, draw test cube, verify camera movement does not affect 2D views.

14) Preserve public API stability (no signature changes)
Status: Ongoing Guideline (No changes required today)
- Change: Avoid modifications to existing View trait signatures and RenderContext types; keep shared code untouched.
- Files: <mcfile name="view.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\view.rs"></mcfile> <mcfile name="render_context.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\render_context.rs"></mcfile>
- Purpose: Guarantees backward compatibility and stability throughout.
- Stability: Existing code paths remain untouched.
- Validation: Run current MPR workflows; confirm identical behavior and performance.

---

Notes on enabling later:
- Build-time: enable Cargo feature `mesh`.
- Runtime: toggle Mesh Mode via `set_mesh_mode_enabled(true)` to instantiate MeshView and run the full-screen mesh path (or mesh pass in multi-pass design).

## Stability, Integration, and Test Plan

Goals
- Preserve current 2D MPR behavior and performance.
- Ensure mesh-related code is fully inert unless both build-time feature and runtime flag are enabled.
- Prevent panics or hard failures; on any mesh-only error, gracefully skip mesh work and continue 2D rendering.

Global stability principles
- Strict gating: build-time feature via Cargo and runtime flag in <mcfile name="state.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\state.rs"></mcfile> with default false.
- Separation of concerns: keep 3D mesh pipeline/state in <mcfile name="mesh_render_context.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\mesh\mesh_render_context.rs"></mcfile> and do not alter existing 2D <mcfile name="render_context.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\view\render_context.rs"></mcfile> or <mcfile name="shader_tex.wgsl" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\shader\shader_tex.wgsl"></mcfile>.
- Pass isolation: execute the mesh pass (when enabled) before the 2D pass in <mcfile name="render_app.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\render_app.rs"></mcfile> using a separate render pass and depth target to avoid state leaks.

Preflight capability checks (when feature is enabled)
- Verify required device features and formats (e.g., Depth32Float) before creating mesh resources; if unsupported, log and disable mesh for this run.
- Validate swapchain surface state from <mcfile name="state.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\state.rs"></mcfile> and ensure proper reconfiguration on SurfaceError Lost/Outdated, identical to current behavior.

Resource management lifecycle
- Lazy initialization: create mesh pipeline and depth texture only on first activation when Mesh Mode is enabled.
- Deterministic drop: free mesh GPU resources when Mesh Mode is disabled or MeshView is torn down; do not alter or depend on the layout system.
- Depth resource creation helper: centralized in <mcfile name="texture.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\texture.rs"></mcfile> to keep pipeline code lean and testable.

Render pass ordering and encoder usage
- Use a single command encoder per frame with distinct render passes: [Mesh pass] then [2D slice pass].
- Do not modify existing bind groups or pipeline layouts for 2D; mesh uses its own pipeline layout and bind groups.
- Avoid MSAA initially; revisit only after baseline stability.

Error handling and logging
- No panics for mesh-only failures. Return Result/Option and log at appropriate levels (info/warn/error) using existing logging.
- On error during mesh resource creation or draw, skip mesh pass for the frame and continue with 2D rendering.
- Maintain current surface error handling in <mcfile name="render_app.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\render_app.rs"></mcfile>.

Cross-platform considerations (native + wasm32)
- Respect window/canvas attach path already used in <mcfile name="render_app.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\render_app.rs"></mcfile>.
- Avoid features known to vary across platforms; if Depth32Float is not available, log and disable mesh for this run.

Comprehensive test matrix
- Build configurations
  - Feature OFF: build and run; confirm identical behavior and logs.
  - Feature ON, runtime OFF: build and run; MeshView not instantiated; identical behavior.
  - Feature ON, runtime ON: instantiate MeshView and run mesh pass; confirm 2D pass renders identically afterward.
- Functional scenarios
  - Resize events: repeated resizes; ensure both passes adapt without errors.
  - CT data formats: RG8 path vs optional R16Float path in <mcfile name="render_content.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\render_content.rs"></mcfile>; mesh pass must not alter these.
  - Input events: verify all <mcfile name="gl_canvas.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\gl_canvas.rs"></mcfile> UserEvents produce identical 2D results with mesh disabled and enabled.
  - Layout interactions: add/remove views; confirm deterministic drop and no resource leaks.
- Stability checks
  - No validator errors from WGPU; no SurfaceError other than transient handled cases.
  - No memory growth beyond expected textures/buffers; depth texture size matches surface.
  - Frame timing similar to baseline when mesh disabled; acceptable overhead when enabled.

Backout and fallback strategy
- If any mesh step causes instability, disable runtime flag at startup or remove MeshView from layout to instantly restore baseline.
- Keep code paths separate so a rollback does not require changes to 2D modules.

Monitoring & diagnostics checklist
- Log creation and drop of MeshView and MeshRenderContext.
- Log feature and runtime gating states at app start in <mcfile name="render_app.rs" path="c:\Users\admin\OneDrive\文档\2024\Imaging\kepler-wgpu\src\render_app.rs"></mcfile>.
- Log depth texture creation parameters and surface resize events.

Per-step validation amplification (applies to steps 1–13)
- For each step:
  - Build with feature OFF and ON; with runtime OFF and ON.
  - Run resize and render loop; confirm 2D behavior unchanged when mesh inactive.
  - On any error, ensure mesh pass is skipped and app continues.

## Structural relationship diagram (hierarchy and dependencies)

Status note: All mesh-related modules, shaders, conditional exports, runtime flags, and render pass wiring described below are Not Yet Implemented and will be added under the feature gate and opt-in runtime control.

Hierarchy (modules, assets, and gating):
- Build-time feature gate
  - <mcfile name="Cargo.toml" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\Cargo.toml"></mcfile>
    - Defines Cargo feature "mesh"
    - Guards all mesh-related modules and conditional exports via `cfg(feature = "mesh")`
- Mesh module (guarded by feature)
  - <mcfile name="mod.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mod.rs"></mcfile>
  - <mcfile name="mesh.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mesh.rs"></mcfile>
  - <mcfile name="material.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\material.rs"></mcfile>
  - <mcfile name="camera.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\camera.rs"></mcfile>
  - <mcfile name="lighting.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\lighting.rs"></mcfile>
  - <mcfile name="mesh_view.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mesh_view.rs"></mcfile>
  - <mcfile name="mesh_render_context.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mesh_render_context.rs"></mcfile>
- Shader assets (prepared, initially inert)
  - <mcfile name="mesh.wgsl" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\shader\\mesh.wgsl"></mcfile>
  - <mcfile name="mesh_depth.wgsl" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\shader\\mesh_depth.wgsl"></mcfile>
- Shared helpers and math
  - <mcfile name="texture.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\texture.rs"></mcfile> (Depth32Float creation helper)
  - <mcfile name="coord.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\coord.rs"></mcfile> (matrices, transforms)
- View system integration
  - <mcfile name="mod.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\mod.rs"></mcfile> (conditional `pub use MeshView`)
  - <mcfile name="view.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\view.rs"></mcfile> (View trait)
- Runtime control and orchestration
  - <mcfile name="state.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\state.rs"></mcfile> (Mesh Mode flag and toggles)
  - <mcfile name="render_app.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\render_app.rs"></mcfile> (render loop; pass ordering)
  - 2D pipeline remains independent: <mcfile name="render_context.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\render_context.rs"></mcfile> + <mcfile name="shader_tex.wgsl" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\shader\\shader_tex.wgsl"></mcfile>

Dependency graph (labeled relationships):
- Build-time gating
  - <mcfile name="Cargo.toml" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\Cargo.toml"></mcfile>
    -> guards { src/mesh/*, <mcfile name="mod.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\mod.rs"></mcfile> conditional export } [label: cfg(feature = "mesh")]
- Runtime gating and view instantiation
  - <mcfile name="state.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\state.rs"></mcfile> (Mesh Mode) ->
    <mcfile name="render_app.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\render_app.rs"></mcfile> [label: instantiate MeshView only when Mesh Mode is enabled]
  - [Removed: no inclusion into layout graph; MeshView renders in full-screen path]
  - <mcfile name="mesh_view.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mesh_view.rs"></mcfile>
    -> <mcfile name="mesh_render_context.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mesh_render_context.rs"></mcfile> [label: owns GPU state]
    -> <mcfile name="mesh.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mesh.rs"></mcfile> [label: geometry data]
    -> <mcfile name="material.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\material.rs"></mcfile> [label: material params]
    -> <mcfile name="camera.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\camera.rs"></mcfile> [label: view/proj matrices]
    -> <mcfile name="lighting.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\lighting.rs"></mcfile> [label: light params]
- Mesh pipeline and resources
  - <mcfile name="mesh_render_context.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\mesh\\mesh_render_context.rs"></mcfile>
    -> <mcfile name="mesh.wgsl" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\shader\\mesh.wgsl"></mcfile> [label: vertex/fragment]
    -> <mcfile name="mesh_depth.wgsl" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\shader\\mesh_depth.wgsl"></mcfile> [label: depth-only]
    -> <mcfile name="texture.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\texture.rs"></mcfile> [label: Depth32Float creation]
    -> <mcfile name="coord.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\coord.rs"></mcfile> [label: matrix utilities]
- Render execution order (non-breaking)
  - <mcfile name="render_app.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\render_app.rs"></mcfile>
    -> [Mesh pass runs first when MeshView exists; otherwise skipped]
    -> <mcfile name="shader_tex.wgsl" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\shader\\shader_tex.wgsl"></mcfile> [label: 2D slice pass remains unchanged]

Stability constraints reflected in the relationships:
- The 2D slice pipeline (<mcfile name="render_context.rs" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\view\\render_context.rs"></mcfile> + <mcfile name="shader_tex.wgsl" path="c:\\Users\\admin\\OneDrive\\文档\\2024\\Imaging\\kepler-wgpu\\src\\shader\\shader_tex.wgsl"></mcfile>) is structurally independent from all mesh modules.
- All mesh modules and exports are guarded by the Cargo feature and runtime flag, ensuring no change to default builds or behavior.