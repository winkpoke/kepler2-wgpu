# WASM View Manipulation API Design

**Date**: 2025-10-28T08:25:41  
**Status**: Design Phase  
**Priority**: High  
**Estimated Timeline**: 3-5 days  

## Executive Summary

This document outlines the design for a comprehensive WASM API that enables JavaScript to manipulate medical imaging views in the Kepler WGPU framework. The API provides functionality for initial layout configuration, view swapping, and view replacement with type safety and proper error handling.

## Current System Analysis

### Existing Architecture

Based on the codebase analysis, the current system has:

1. **Layout System**: `Layout<GridLayout>` with 2x2 grid (4 slots)
2. **View Types**: 
   - `MprView` (Multi-Planar Reconstruction) - 3 orientations (Axial, Coronal, Sagittal)
   - `MeshView` (3D mesh rendering)
   - `MipView` (Maximum Intensity Projection)
3. **Current Slot Assignment**:
   - Slot 0: Axial MPR view
   - Slot 1: Coronal MPR view  
   - Slot 2: Sagittal MPR view OR MeshView (feature-dependent)
   - Slot 3: Reserved for future use
4. **Event System**: `UserEvent` enum with `EventLoopProxy` for communication
5. **WASM Bindings**: Existing `GLCanvas` with `wasm_bindgen` for basic operations

### Current Limitations

- No API for dynamic view arrangement
- Fixed layout configuration (2x2 grid)
- Limited view swapping capabilities
- No programmatic view replacement

## API Design

### Core Principles

1. **Type Safety**: Strong typing with Rust enums and proper error handling
2. **Medical Accuracy**: Preserve medical imaging requirements and coordinate systems
3. **Performance**: Minimize GPU resource recreation and memory allocations
4. **Consistency**: Follow existing WASM binding patterns in the codebase
5. **Extensibility**: Design for future view types and layout strategies

### View Type Enumeration

```rust
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewType {
    /// Multi-Planar Reconstruction - Axial orientation
    MprAxial,
    /// Multi-Planar Reconstruction - Coronal orientation  
    MprCoronal,
    /// Multi-Planar Reconstruction - Sagittal orientation
    MprSagittal,
    /// 3D Mesh rendering view
    Mesh,
    /// Maximum Intensity Projection view
    Mip,
    /// Empty slot (no view)
    Empty,
}
```

### Layout Configuration

```rust
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    /// Number of rows in the grid
    pub rows: u32,
    /// Number of columns in the grid  
    pub cols: u32,
    /// Spacing between views in pixels
    pub spacing: u32,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug, Clone)]
pub struct ViewSlotConfig {
    /// Slot index (0-based)
    pub slot: u32,
    /// Type of view to place in this slot
    pub view_type: ViewType,
}
```

### Error Handling

```rust
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug, Clone)]
pub enum ViewManipulationError {
    /// Invalid slot index
    InvalidSlot,
    /// View type not supported
    UnsupportedViewType,
    /// No volume data loaded
    NoVolumeData,
    /// GPU resource creation failed
    ResourceCreationFailed,
    /// Layout configuration invalid
    InvalidLayout,
    /// View already exists at slot
    SlotOccupied,
    /// Cannot swap view with itself
    SelfSwap,
}
```

## API Methods

### 1. Initial Layout Configuration

```rust
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl GLCanvas {
    /// Set the initial view arrangement in the layout.
    /// 
    /// This method configures the grid layout and populates it with the specified views.
    /// Must be called before loading volume data for optimal performance.
    /// 
    /// # Arguments
    /// * `layout_config` - Grid configuration (rows, cols, spacing)
    /// * `view_configs` - Array of view slot configurations
    /// 
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(ViewManipulationError)` on failure
    /// 
    /// # Example Layout Configurations
    /// 
    /// **Standard Medical Imaging (2x2)**:
    /// ```
    /// ┌─────────────┬─────────────┐
    /// │   Axial     │  Coronal    │
    /// │  (Slot 0)   │  (Slot 1)   │
    /// ├─────────────┼─────────────┤
    /// │  Sagittal   │    Mesh     │
    /// │  (Slot 2)   │  (Slot 3)   │
    /// └─────────────┴─────────────┘
    /// ```
    /// 
    /// **Single View Focus (1x1)**:
    /// ```
    /// ┌─────────────────────────────┐
    /// │                             │
    /// │         Mesh View           │
    /// │         (Slot 0)            │
    /// │                             │
    /// └─────────────────────────────┘
    /// ```
    pub fn set_initial_layout(
        &self, 
        layout_config: &LayoutConfig,
        view_configs: &[ViewSlotConfig]
    ) -> Result<(), ViewManipulationError>;
}
```

### 2. View Swapping

```rust
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl GLCanvas {
    /// Swap the positions of two views in the layout.
    /// 
    /// This method exchanges the views at two different slots, preserving their
    /// internal state (window/level, slice position, zoom, pan, etc.).
    /// 
    /// # Arguments
    /// * `slot_a` - First slot index
    /// * `slot_b` - Second slot index
    /// 
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(ViewManipulationError)` on failure
    /// 
    /// # State Preservation
    /// 
    /// The following view states are preserved during swapping:
    /// - Window/Level settings (for MPR views)
    /// - Slice position and orientation
    /// - Zoom scale and pan translation
    /// - Rotation settings (for Mesh views)
    /// 
    /// # Performance Notes
    /// 
    /// - GPU resources are moved, not recreated
    /// - Uniform buffers are updated with new positions
    /// - No texture data is copied
    pub fn swap_views(&self, slot_a: u32, slot_b: u32) -> Result<(), ViewManipulationError>;
}
```

### 3. View Replacement

```rust
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl GLCanvas {
    /// Replace a view at the specified slot with a new view type.
    /// 
    /// This method removes the existing view and creates a new view of the
    /// specified type. The new view will be initialized with default settings.
    /// 
    /// # Arguments
    /// * `slot` - Target slot index
    /// * `new_view_type` - Type of view to create
    /// 
    /// # Returns
    /// * `Ok(ViewType)` - The previous view type that was replaced
    /// * `Err(ViewManipulationError)` on failure
    /// 
    /// # Resource Management
    /// 
    /// - Previous view GPU resources are properly cleaned up
    /// - New view resources are created with current volume data
    /// - Bind groups and uniform buffers are recreated as needed
    /// 
    /// # Default Initialization
    /// 
    /// New views are created with these defaults:
    /// - **MPR Views**: Window/Level from volume metadata, center slice, 1.0x zoom
    /// - **Mesh View**: Orthogonal projection, rotation enabled, default lighting
    /// - **MIP View**: Maximum intensity projection with default transfer function
    pub fn replace_view_at(
        &self, 
        slot: u32, 
        new_view_type: ViewType
    ) -> Result<ViewType, ViewManipulationError>;
}
```

### 4. Layout Query and Status

```rust
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl GLCanvas {
    /// Get the current layout configuration.
    /// 
    /// Returns the current grid layout settings including rows, columns, and spacing.
    pub fn get_layout_config(&self) -> LayoutConfig;
    
    /// Get the view type at the specified slot.
    /// 
    /// # Arguments
    /// * `slot` - Slot index to query
    /// 
    /// # Returns
    /// * `Ok(ViewType)` - The view type at the slot
    /// * `Err(ViewManipulationError)` - If slot index is invalid
    pub fn get_view_type_at(&self, slot: u32) -> Result<ViewType, ViewManipulationError>;
    
    /// Get all current view types in slot order.
    /// 
    /// Returns an array of view types corresponding to each slot in the layout.
    /// Empty slots are represented as `ViewType::Empty`.
    pub fn get_all_view_types(&self) -> Vec<ViewType>;
    
    /// Get the total number of available slots in the current layout.
    pub fn get_slot_count(&self) -> u32;
}
```

## Implementation Strategy

### Phase 1: Core Infrastructure (Days 1-2)

1. **Add New UserEvent Variants**:
   ```rust
   pub enum UserEvent {
       // ... existing variants
       SetInitialLayout(LayoutConfig, Vec<ViewSlotConfig>),
       SwapViews(u32, u32),
       ReplaceViewAt(u32, ViewType),
       GetLayoutConfig(oneshot::Sender<LayoutConfig>),
       GetViewTypeAt(u32, oneshot::Sender<Result<ViewType, ViewManipulationError>>),
       GetAllViewTypes(oneshot::Sender<Vec<ViewType>>),
   }
   ```

2. **Extend GLCanvas with New Methods**: Implement the WASM-bound methods using the existing event proxy pattern

3. **Add View Management to State**: Extend the `State` struct with view manipulation logic

### Phase 2: Layout Management (Days 2-3)

1. **Dynamic Layout Strategy**: Extend `Layout<T>` to support runtime strategy changes
2. **View Factory Pattern**: Implement centralized view creation with proper resource management
3. **State Preservation**: Add view state serialization/deserialization for swapping

### Phase 3: Error Handling and Validation (Day 3)

1. **Input Validation**: Comprehensive validation for slot indices, layout configurations
2. **Resource Cleanup**: Proper GPU resource cleanup for replaced views
3. **Error Propagation**: Convert Rust errors to JavaScript-friendly error messages

### Phase 4: JavaScript Integration (Days 4-5)

1. **TypeScript Definitions**: Create `.d.ts` files for type safety
2. **JavaScript Wrapper Functions**: High-level convenience functions
3. **Demo Integration**: Update `index.html` with interactive examples

## JavaScript API Usage Examples

### Example 1: Standard Medical Imaging Layout

```javascript
// Set up standard 2x2 medical imaging layout
const layoutConfig = {
    rows: 2,
    cols: 2,
    spacing: 2
};

const viewConfigs = [
    { slot: 0, view_type: "MprAxial" },
    { slot: 1, view_type: "MprCoronal" },
    { slot: 2, view_type: "MprSagittal" },
    { slot: 3, view_type: "Mesh" }
];

try {
    await canvas.set_initial_layout(layoutConfig, viewConfigs);
    console.log("Layout configured successfully");
} catch (error) {
    console.error("Failed to set layout:", error);
}
```

### Example 2: Dynamic View Swapping

```javascript
// Swap axial and mesh views
try {
    await canvas.swap_views(0, 3); // Swap slot 0 (axial) with slot 3 (mesh)
    console.log("Views swapped successfully");
} catch (error) {
    console.error("Failed to swap views:", error);
}
```

### Example 3: View Replacement

```javascript
// Replace sagittal view with MIP view
try {
    const previousType = await canvas.replace_view_at(2, "Mip");
    console.log(`Replaced ${previousType} with MIP view`);
} catch (error) {
    console.error("Failed to replace view:", error);
}
```

### Example 4: Layout Inspection

```javascript
// Query current layout state
const layoutConfig = await canvas.get_layout_config();
const allViews = await canvas.get_all_view_types();

console.log(`Layout: ${layoutConfig.rows}x${layoutConfig.cols}`);
console.log("Current views:", allViews);

// Check specific slot
try {
    const viewType = await canvas.get_view_type_at(0);
    console.log(`Slot 0 contains: ${viewType}`);
} catch (error) {
    console.error("Invalid slot:", error);
}
```

## Performance Considerations

### GPU Resource Management

1. **Resource Pooling**: Reuse depth textures and uniform buffers where possible
2. **Lazy Initialization**: Create view resources only when needed
3. **Efficient Cleanup**: Proper disposal of GPU resources for replaced views

### Memory Optimization

1. **Shared Resources**: Use `Arc<RenderContent>` for volume data sharing
2. **State Preservation**: Minimize data copying during view swaps
3. **Batch Operations**: Group multiple layout changes into single operations

### Cross-Platform Compatibility

1. **WASM Constraints**: Ensure all operations work within WASM limitations
2. **WebGPU Validation**: Proper error handling for WebGPU validation failures
3. **Browser Compatibility**: Test across different browsers and devices

## Testing Strategy

### Unit Tests

1. **View Creation**: Test each view type creation with various configurations
2. **Layout Validation**: Test layout configuration validation logic
3. **Error Handling**: Test all error conditions and edge cases

### Integration Tests

1. **End-to-End Workflows**: Test complete layout setup and manipulation workflows
2. **State Preservation**: Verify view state is preserved during swaps
3. **Resource Cleanup**: Ensure no memory leaks during view replacement

### WASM-Specific Tests

1. **JavaScript Integration**: Test WASM bindings from JavaScript
2. **Error Propagation**: Verify Rust errors are properly converted to JavaScript
3. **Performance**: Benchmark layout operations in browser environment

## Security Considerations

1. **Input Validation**: Validate all parameters from JavaScript
2. **Resource Limits**: Prevent excessive resource allocation
3. **Error Information**: Avoid exposing internal system details in error messages

## Future Extensibility

### Planned Enhancements

1. **Custom Layout Strategies**: Support for non-grid layouts (e.g., picture-in-picture)
2. **View Animations**: Smooth transitions during view changes
3. **Layout Presets**: Predefined layout configurations for common use cases
4. **View Synchronization**: Synchronized navigation across multiple views

### Plugin Architecture

1. **Custom View Types**: Support for external view implementations
2. **Layout Plugins**: Custom layout strategies via plugin system
3. **Event Hooks**: Callbacks for view manipulation events

## Migration Strategy

### Backward Compatibility

1. **Existing API**: Maintain compatibility with current WASM bindings
2. **Gradual Migration**: Allow incremental adoption of new API
3. **Feature Flags**: Use feature flags to enable new functionality

### Documentation Updates

1. **API Documentation**: Comprehensive documentation for all new methods
2. **Migration Guide**: Step-by-step guide for updating existing code
3. **Examples**: Complete examples for common use cases

## Success Criteria

### Functional Requirements

- [ ] Set initial layout with arbitrary view arrangements
- [ ] Swap views between any two slots
- [ ] Replace views with different types
- [ ] Query current layout state
- [ ] Proper error handling and validation

### Performance Requirements

- [ ] Layout changes complete within 100ms
- [ ] No memory leaks during view manipulation
- [ ] GPU resource usage remains optimal
- [ ] Smooth rendering during transitions

### Quality Requirements

- [ ] Type-safe JavaScript API with TypeScript definitions
- [ ] Comprehensive error messages
- [ ] 100% test coverage for new functionality
- [ ] Cross-browser compatibility (Chrome, Firefox, Safari, Edge)

## Conclusion

This WASM view manipulation API design provides a comprehensive solution for dynamic medical imaging layout management. The design prioritizes type safety, performance, and medical imaging accuracy while maintaining consistency with the existing codebase architecture.

The phased implementation approach ensures incremental delivery of functionality while maintaining system stability. The API design is extensible and provides a solid foundation for future enhancements in medical imaging visualization.

## Related Documents

- [View Layout Refactoring Plan](views/view-layout-refactoring-plan.md)
- [Rendering Architecture Design](redering/rendering-architecture-design.md)
- [MPR Aspect Ratio Fix Implementation Plan](views/2025-10-28T07-45-52-mpr-aspect-ratio-fix-implementation-plan.md)
- [WGPU 27 and Winit 0.30 Upgrade Plan](2025-10-28T07-54-10-wgpu-27-winit-030-upgrade-plan.md)