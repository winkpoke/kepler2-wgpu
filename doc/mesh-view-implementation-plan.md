# Mesh View Replacement Mode — Replace Third View When Enabled

Purpose
- Provide a mesh visualization that replaces the current third view (bottom-left) only when the mesh feature is enabled at build-time and selected at initialization; with no runtime toggling.
- Support both native and WASM builds (wasm-pack build -t web) on Windows/macOS/Linux.

Context and Constraints
- Existing grid layout remains in use; only the third view’s content is selected at initialization between MPR and MeshView.
- Keep the view indices stable and do not reflow or resize other views; the replacement must respect the same position and dimensions of the third slot.
- No runtime mode switching; selection is determined at startup (e.g., compile-time Cargo feature and/or static configuration).
- Scope: This plan applies exclusively to the current 2x2 grid layout (Top-Left, Top-Right, Bottom-Left, Bottom-Right).

Design Overview
- When built with the `mesh` Cargo feature and configured to enable MeshView for the third slot at initialization, the application places a MeshView instance into the third view (bottom-left).
- When not configured to use MeshView at initialization (or when the `mesh` feature is disabled), the application uses the original MPR view (e.g., GenericMPRView) in the third slot.
- The layout object (GridLayout) continues to compute positions and sizes; whichever view is selected for the third slot reads the slot’s viewport to render correctly.

Implementation Plan
1) Initialization-time selection (no runtime toggle)
   - At application startup, decide whether the third slot uses MeshView or the original MPR view:
     - If the binary is compiled with the Cargo feature `mesh` and the static configuration indicates MeshView should be used, instantiate MeshView for the third slot.
     - Otherwise, instantiate the original MPR view for the third slot.
   - Do not provide runtime toggles; switching requires restarting with a different configuration or compile-time flags.

2) Slot indexing and layout integration
   - Define a constant (e.g., `MESH_SLOT_INDEX`) for the third slot in the current grid:
     - For 2x2 grid: `MESH_SLOT_INDEX = 2` (0-based: [0 TL, 1 TR, 2 BL, 3 BR]).

   - Initialize the view at `MESH_SLOT_INDEX` during startup based on selection without affecting other slots.
   - Ensure both MeshView and MPR views honor `position()` and `dimensions()` when rendering, using the slot’s viewport.

3) MeshView rendering integration
   - MeshView implements `View` and uses `render_pass.set_viewport(...)` with the third slot’s position and dimensions.
   - Ensure a depth attachment is available for MeshView; create lazily and reuse via a TexturePool.
   - Keep MeshView’s camera, lighting, and materials consistent; adopt existing math utilities from `coord.rs`.

4) Testing and validation
   - Visual (native):
     - Build with `mesh` feature and the configuration selecting MeshView at initialization → confirm the third slot shows the mesh; other views unchanged.
     - Build without the `mesh` feature (or configured to use MPR) → confirm the third slot shows the original MPR view; other views unchanged.
   - WASM: build with `wasm-pack build -t web`; manual browser validation:
     - Verify the third slot renders MeshView when compiled/configured accordingly; otherwise renders MPR.
   - Stability: ensure initialization does not recreate pipelines unnecessarily; cache MeshRenderContext and depth textures.

Performance and Robustness
- Avoid unnecessary pipeline creation; cache `MeshRenderContext` and depth textures.
- Handle zero-sized surfaces (especially on WASM) before creating depth or swapchain-dependent resources.
- Keep a clear separation: MPR views continue their normal path; the third slot selection is localized and does not affect other views.

Deliverables
- `state.rs`: implement initialization-time selection for the third slot; remove any runtime toggle paths.
- `view/layout.rs`: ensure a helper exists or is used to assign a view at a specific index during startup without affecting others.
- `mesh_view.rs` / `mesh_render_context.rs`: confirm slot-based viewport handling and depth usage.
- Validation notes for native and WASM builds (wasm-pack build -t web).