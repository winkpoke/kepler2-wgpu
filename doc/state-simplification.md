# State Struct Simplification

## Overview
The deprecated `State` struct and its associated fields have been removed from the `App` struct to simplify the codebase and eliminate unused state management.

## Changes Made

### Removed Fields
- `last_volume: Option<Arc<CTVolume>>` - No longer needed as volume state is managed by `AppModel`
- `texture_pool: MeshTexturePool` - Replaced with temporary per-frame texture pool creation
- `toggle_enabled: bool` - Functionality moved to per-view flags in `AppView`

### Implementation Changes
- **Rendering**: Instead of maintaining a persistent `texture_pool` in `App`, a temporary `MeshTexturePool` is created for each frame render
- **Mesh Toggle**: The mesh toggle functionality now works correctly by directly using the `enable_mesh` flag in `App::render` and rebuilding layout immediately when toggled
- **State Management**: Volume ownership moved to `AppModel`, UI toggles managed by `AppView`

## Benefits
- **Simplified Architecture**: Eliminated unused state fields and reduced complexity
- **Better Separation of Concerns**: Clear ownership between `AppModel` (data) and `AppView` (UI)
- **Performance**: Per-frame texture pool creation is lightweight and avoids persistent state overhead
- **Maintainability**: Reduced coupling between components

## Technical Details
The temporary texture pool approach creates a new `MeshTexturePool` for each frame render call. This is efficient because:
- Texture creation is only done when needed (when size changes)
- No persistent state to manage or synchronize
- Simpler lifecycle management
- Reduces memory footprint

## Migration Path
This change is internal and requires no API changes. The functionality remains the same while the implementation is simplified.