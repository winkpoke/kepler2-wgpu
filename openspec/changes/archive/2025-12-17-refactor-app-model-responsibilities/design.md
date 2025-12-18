# Design: App Model Refactoring

## Problem
`App` struct currently mixes WGPU resource management (Window, Surface, Device, Queue) with Application Logic (Mesh toggle, Texture format preference, Data conversion). This coupling makes it impossible to test data logic without initializing a WGPU environment, which is heavy and platform-dependent.

## Solution
We move the "Business Logic" of the application into `AppModel`.

### `AppModel` Responsibilities
1.  **State Holder**: Stores `enable_mesh`, `enable_float_volume_texture`.
2.  **Data Processor**: 
    -   Contains the `HU_OFFSET` constant.
    -   Implements `get_volume_render_data()` which takes the loaded `CTVolume` and produces the `Vec<u8>` ready for texture upload.
    -   Handles the decision branch for `R16Float` (f16 conversion) vs `Rg8Unorm` (window/level offset).

### `App` Responsibilities
1.  **Coordinator**: Orchestrates the loop between Input, Update, and Render.
2.  **Resource Manager**: Owns `GraphicsContext` (Device, Queue, Surface).
3.  **View Manager**: Owns `AppView` (Layout, ViewFactory).

## Data Flow
**Old Flow:**
`App::load_data` -> Access `vol.voxel_data` -> Apply Math -> Create Texture -> Update Layout.

**New Flow:**
`App::load_data` -> `AppModel::load_volume`
`App::load_data` -> `AppModel::get_volume_render_data` -> (returns `Vec<u8>`, `is_float`)
`App` -> Create Texture (using returned bytes) -> Update Layout.

## Dependencies
-   `AppModel` needs `half` crate for f16 conversion (already used in `App`, just moving usage).
-   `AppModel` needs `bytemuck` for casting (already used).
