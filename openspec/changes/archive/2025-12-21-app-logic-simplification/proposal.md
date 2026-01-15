# Change: App Logic Simplification

## Why
The `App` struct in `src/application/app.rs` has accumulated repetitive logic for layout management, view property updates, and redundant graphics context delegation. This complexity reduces maintainability, increases the risk of bugs (e.g., inconsistencies in view updates), and clutters the API. Additionally, critical paths like data loading currently panic on failure instead of returning errors.

Simplifying `App` will:
1.  **Reduce Code Duplication**: Consolidate repetitive view iteration logic into helpers.
2.  **Improve Robustness**: Handle errors gracefully during data loading and view creation.
3.  **Clean Up API**: Remove redundant pass-through methods that expose internal `GraphicsContext` details unnecessarily.
4.  **Centralize Layout Logic**: Move complex layout state management into dedicated helpers or the `AppView`.

## What Changes
1.  **Simplify View Property Setters**: Introduce `apply_to_mesh_view` helper to encapsulate view iteration and downcasting.
2.  **Refactor Graphics Delegation**: Remove redundant accessor methods (`device()`, `queue()`, etc.) and expose `graphics()` and `graphics_mut()` directly.
3.  **Error Handling**: Update `load_data_from_ct_volume` to return `Result<Arc<RenderContent>, KeplerError>` instead of panicking.
4.  **Consolidate Layout Construction**: (Planned) Extract layout rebuilding logic into `rebuild_layout` helper to simplify `set_mesh_mode_enabled` and `set_one_cell_layout`.

## Impact
-   **Affected Specs**: `application` (API cleanup).
-   **Affected Code**:
    -   `src/application/app.rs`: Significant refactoring of methods.
    -   `src/application/render_app.rs`: Updates to call sites for graphics accessors (e.g., `state.device()` -> `state.graphics().device`).
