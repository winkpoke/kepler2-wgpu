# Dynamic Replacement of Bottom-Left View with Mesh in 4×4 Grid — Implementation Plan

Purpose
- Dynamically replace the third view (bottom-left position) in a 4×4 grid layout with a Mesh visualization when enabled, while preserving the original MPR view state and maintaining all other views’ positions and behavior.
- Target both native and WASM builds (wasm-pack build -t web), working on Windows/macOS/Linux.

Context and Existing Architecture
- Layout is managed by a grid strategy in `src/view/layout.rs` via `Layout<GridLayout>`.
- MPR views are implemented in `src/view/view.rs` as `GenericMPRView`, adhering to the `View` trait and an `MPRView` trait for medical imaging controls.
- Mesh visualization view exists in `src/mesh/mesh_view.rs`, already implementing `View` and `Renderable` with viewport-based rendering.
- Rendering setup and depth attachment are controlled in `src/state.rs`.

Target Slot (Bottom-Left) Indexing
- GridLayout uses row-major order starting at 0.
- Bottom-left index for an r×c grid is: `(rows - 1) * cols + 0`.
- For 4×4, bottom-left index is 12.
- Compute this index from the active `GridLayout` at runtime to avoid hard-coding and remain robust to future layout changes.

1) View Management
- MPR view state management
  - Define `MprViewState` capturing window_level, window_width, pan([f32;3]), scale, translate([f32;3]).
  - Extend `MPRView` trait with `export_state(&self) -> MprViewState` and `import_state(&mut self, state: MprViewState)`.
  - Implement these in `GenericMPRView` by reading/writing uniforms and camera/transform fields so restoration is visually identical and immediate.
- View registry and swapping
  - Introduce a lightweight view registry in `State` tracking `slot_index -> ViewKind` and cached data for swap:
    - `ViewKind`: `MPR(Orientation) | Mesh | Other`.
    - Cached `Box<dyn View>` for the original MPR view at the bottom-left slot.
    - Cached `MprViewState` for the same slot as fallback for full reconstruction.
  - Add `Layout::replace_view_at(index, view)`:
    - Replace `views[index]` in place without reordering.
    - Recompute `(pos, size)` using `strategy.calculate_position_and_size(index, total_views, dim)`.
    - Call `move_to` and `resize` on the new view to match the grid cell.

2) Mesh Visualization Integration
- Styling and consistency
  - Ensure MeshView matches rendering conventions used by `GenericMPRView` (viewport set per view by layout, same render pass sequencing).
  - Use existing conditional depth attachment from `State::render`; MeshView benefits from depth testing while MPR views render unaffected.
- Sizing and positioning
  - MeshView already implements `move_to`/`resize` and uses viewport inside `render`. With `Layout::replace_view_at`, placement will be correct in the bottom-left cell.

3) Feature Toggle Implementation
- Clean toggle mechanism
  - Implement `State::set_mesh_enabled(&mut self, enabled: bool)`:
    - Compute bottom-left index from `GridLayout` (rows/cols → `(rows-1)*cols`).
    - When enabling:
      - Cache and remove the existing MPR view at bottom-left via the registry; export its `MprViewState`.
      - Create or reuse `MeshView` and `MeshRenderContext`; call `Layout::replace_view_at(index, Box::new(mesh_view))`.
      - Set `enable_mesh = true`.
    - When disabling:
      - Replace the MeshView at bottom-left with the cached MPR view using `Layout::replace_view_at`.
      - Restore its `MprViewState` via `import_state`.
      - Set `enable_mesh = false`.
  - Native keybinding: map ‘M’ to call `set_mesh_enabled`; avoid full volume reload for performance and state preservation.
  - Web UI toggle: add a checkbox/button in `static/index.html` to trigger the toggle via WASM bindings.
- Conditional rendering
  - Keep render pass depth attachment optional:
    - Attach when `enable_mesh == true` (MeshView needs depth); omit otherwise.

4) State Preservation
- On mesh enable:
  - Export MPR view state from bottom-left and cache the original `Box<dyn View>`.
- On mesh disable:
  - Restore the cached MPR view and import its state for seamless continuity.
- Fallback reconstruction:
  - If the cached view is unavailable (edge-case), reconstruct `GenericMPRView` from `last_volume` and `Orientation` metadata, then import the saved `MprViewState`.

5) Testing Strategy
- Unit tests
  - `Layout::replace_view_at` correctness: slot replacement does not reorder other views; recomputes viewport for replaced index only.
  - `MprViewState` round-trip on `GenericMPRView`: exported values match uniforms/fields; import restores visuals.
  - Grid indexing: property-based tests for bottom-left index across various (rows, cols) ensure in-bounds and correctness.
- Visual regression (native)
  - Headless render to texture (when possible), compute checksums/hashes:
    - Baseline with all MPR views.
    - Toggle mesh on: verify only the bottom-left cell changes; others remain stable.
    - Toggle mesh off: verify restored baseline.
- End-to-end
  - Native: simulate toggles via `set_mesh_enabled(true/false)`; assert registry state, slot contents, and restored visuals.
  - WASM: build with `wasm-pack build -t web`, manual browser validation:
    - Toggle Mesh via web UI; confirm correct placement and depth/culling.

Performance and Robustness Considerations
- Avoid full layout teardown or volume reload during toggles; use in-place swap.
- Cache/reuse `MeshRenderContext` and depth texture via `TexturePool`; recreate on resize only.
- Preserve and reuse `last_volume` for fallback MPR reconstruction; avoid heavy reinitialization.
- Ensure zero-sized surface safeguards (WASM) when initializing depth textures.

Step-by-Step Tasks
1) Add `MprViewState` and `export_state`/`import_state` to `MPRView`; implement in `GenericMPRView`.
2) Implement `Layout::replace_view_at(index, view)` for in-place swapping.
3) Add view registry and caches in `State` (kind + cached view + cached state for bottom-left).
4) Implement `State::set_mesh_enabled(bool)` to perform conditional swap at bottom-left.
5) Wire up native keybinding and web UI toggle (without triggering volume reload).
6) Add unit tests for replace/swap and state round-trip; add visual regression harness (native); validate in WASM.

Deliverables
- API additions in `view.rs` for MPR state persistence.
- New layout swap API in `layout.rs`.
- Toggle logic and registry in `state.rs`.
- Web UI toggle updates in `static/index.html`.
- Tests and validation scripts harness (native), and manual browser validation steps.