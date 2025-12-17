# Change: Refactor App Model Responsibilities

## Why
Currently, `App` (the controller) handles too much low-level data transformation logic (e.g., converting CT voxels to texture bytes, handling Hounsfield Unit offsets) and manages application configuration state (`enable_mesh`, `enable_float_volume_texture`). This violates the separation of concerns, makes `App` harder to read, and complicates testing of data logic since `App` is coupled to WGPU resources.

Moving this logic to `AppModel` will:
1.  Decouple data preparation from rendering orchestration.
2.  Enable unit testing of data transformation logic without WGPU context.
3.  Clarify the role of `App` as a coordinator rather than a logic handler.

## What Changes
-   **Move State**: `enable_mesh` and `enable_float_volume_texture` flags move from `App` to `AppModel`.
-   **Move Constants**: `HU_OFFSET` moves to `AppModel` (internal to data preparation).
-   **Encapsulate Logic**: `AppModel` gains a new method `get_volume_render_data()` that returns the byte buffer and format information for texture creation.
-   **Simplify App**: `App::load_data_from_ct_volume` is refactored to delegate data processing to `AppModel`.

## Impact
-   **Affected Specs**: `application` (new capability definition).
-   **Affected Code**:
    -   `src/application/app.rs`: Removal of data logic and state fields.
    -   `src/application/app_model.rs`: Addition of state fields and data processing methods.
