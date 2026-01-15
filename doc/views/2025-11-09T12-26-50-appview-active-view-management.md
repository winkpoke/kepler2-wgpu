# AppView Active View Management Wrappers

Date: 2025-11-09T12-26-50

Summary:
- Added minimal, incremental helpers on AppView to manage view lifecycle and layout strategy:
  - replace_view_at(index, new_view): wrapper over LayoutContainer::replace_view_at
  - set_one_cell_layout(): switch to OneCellLayout
  - set_grid_layout(rows, cols, spacing): switch to GridLayout
  - is_one_cell_layout(): detect single-view mode
  - active_index(): returns Some(0) in OneCellLayout, None otherwise

Notes:
- These helpers centralize strategy changes and view replacement logic within AppView, keeping layout management consistent and reducing direct exposure of DynamicLayout internals.
- No visual/UI changes are introduced by these helpers alone; they provide groundwork for future interaction orchestration and linked view synchronization.

Example usage (reference only):

```rust,no_run
use kepler_wgpu::application::AppView;
use kepler_wgpu::rendering::view::{View, OneCellLayout, GridLayout};

fn configure_layout(app_view: &mut AppView) {
    // Switch to single-view mode
    app_view.set_one_cell_layout();

    if app_view.is_one_cell_layout() {
        assert_eq!(app_view.active_index(), Some(0));
    }

    // Switch to a 2x2 grid with spacing 2
+    app_view.set_grid_layout(2, 2, 2);
}
```

Logging:
- Default logging level remains INFO. Use DEBUG logs during development when invoking these helpers.
- TRACE logs are gated by the trace-logging feature and should be sampled if used within render loops.

WASM:
- No changes required for WebAssembly builds. Logs continue to use console_log for browser output.