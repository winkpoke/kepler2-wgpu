# Change: Refactor AppView Encapsulation

## Why
Currently, the `App` struct is tightly coupled with view implementations (`MprView`, etc.). It manually iterates over views and performs downcasting to manage state (capture/restore) and set properties (window level, slice position, etc.). This violates separation of concerns and makes `App` unnecessarily complex and brittle.

## What Changes
- **Encapsulate State Management**: Move logic for capturing and restoring view state (scale, pan, orientation, etc.) from `App` to `AppView`.
- **Centralize View Interaction**: Move view property setters (`set_window_level`, `set_slice_mm`, etc.) from `App` to `AppView`, hiding type-checking and error-handling details.

## Impact
- **Affected Specs**: `application`
- **Affected Code**: `src/application/app.rs`, `src/application/appview.rs`
- **Behavior**: No user-visible behavior change; internal structural refactoring only.
