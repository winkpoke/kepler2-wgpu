# AppModel Refactoring Plan

This document outlines the refactoring plan to move data-related logic from the main `App` struct into `AppModel`. This will improve separation of concerns and make the codebase cleaner and easier to maintain.

## Analysis

- **`AppModel`**: Currently a pure data container for `CTVolume`.
- **`AppView`**: Centralizes view and layout management.
- **`App`**: Acts as a central orchestrator but holds a mix of rendering-specific state, application data, and data-loading logic.

## Refactoring Plan

### 1. Move Data-Loading Logic to `AppModel`

-   **Task:** Relocate all data-loading functionality (e.g., for DICOM, MHD, meshes) from `App` into `AppModel`.
-   **Rationale:** `AppModel` should be the single source of truth for all application data.

### 2. Transfer Mesh Data Ownership to `AppModel`

-   **Task:** Move the `Mesh` data into `AppModel` (e.g., as an `Option<Mesh>`).
-   **Rationale:** The mesh is application data, and its state should be managed by `AppModel`.

### 3. Centralize Volume-Related Properties in `AppModel`

-   **Task:** Move `WindowLevel` and other data-centric properties (like MPR slice indices) into `AppModel`.
-   **Rationale:** These properties directly relate to how the `CTVolume` is interpreted and should be managed alongside the data.

### 4. Consolidate Application State in `AppModel`

-   **Task:** Move application-level settings that are not directly tied to rendering (e.g., `toggle_enabled`) into `AppModel`.
-   **Rationale:** Centralize all non-transient application state in `AppModel`.