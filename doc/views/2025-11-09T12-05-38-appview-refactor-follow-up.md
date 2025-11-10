# AppView Refactor Follow-up: Redirecting Layout and View Factory

This document summarizes a minimal, incremental follow-up to the AppView refactor to ensure consistency across the rendering core by routing layout and view factory access through `AppView`.

## Overview

- Centralized layout and view creation by accessing them via `state.app_view.layout` and `state.app_view.view_factory`.
- Removed lingering direct references to `state.layout` and `state.view_factory` within `src/rendering/core/state.rs` and updated `src/application/render_app.rs` accordingly.
- Ensured trait method resolution for resizing by explicitly calling `LayoutContainer::resize(&mut self.layout, dim)` from `AppView::resize`.

## Rationale

By consolidating layout and view creation through `AppView`, we maintain a clear ownership model for UI state and view lifecycle management, improving maintainability and avoiding divergence after refactoring.

## Code Examples

Rust examples are illustrative only and not executed during documentation generation.

```rust
/// Accessing layout and view factory via AppView
/// ```no_run
/// use kepler_wgpu::application::appview::AppView;
/// use kepler_wgpu::rendering::view::DynamicLayout;
/// 
/// fn example(state: &mut kepler_wgpu::rendering::core::state::State) {
///     // Previously: state.layout.add_view(...)
///     // Now: route through AppView
///     let layout = &mut state.app_view.layout;
///     // layout.add_view(...);
/// 
///     let factory = &mut state.app_view.view_factory;
///     // let view = factory.create_default_view(...);
/// }
/// ```
```

```rust
/// AppView resize using LayoutContainer trait
/// ```no_run
/// use kepler_wgpu::rendering::LayoutContainer;
/// use kepler_wgpu::application::appview::AppView;
/// 
/// fn resize_app_view(app_view: &mut AppView, width: u32, height: u32) {
///     LayoutContainer::resize(&mut app_view.layout, (width, height));
/// }
/// ```
```

## Build and Test Status

- Native build: success (`cargo build`).
- Tests: 27 passed; 2 failed due to missing external environment paths; unrelated to refactor.
- WebAssembly build: success (`wasm-pack build -t web`).

## Logging and Performance Notes

- Default logging level remains INFO; no changes to logging configuration.
- For wasm builds, logs continue to route to the browser console via `console_log`.
- No changes to GPU synchronization or render loop behavior; efficiency constraints remain intact.

## Impact

- No UI changes; internal API consistency improved.
- Prepares the codebase for further incremental features while keeping both native and wasm targets functional.