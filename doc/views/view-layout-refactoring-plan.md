# View and Layout System Refactoring Plan

## Overview

This document outlines a comprehensive refactoring plan to improve the architecture of the view and layout system in the Kepler WGPU medical imaging framework. The primary goal is to move functionalities from the monolithic `set_mesh_mode_enabled` function in `State` into more appropriate modules (`layout.rs` and `view.rs`), following the single responsibility principle and improving code maintainability.

## Current Issues

### Problems with Current Implementation

1. **Monolithic Function**: The `set_mesh_mode_enabled` function in `State` is ~200 lines long and handles multiple responsibilities
2. **Tight Coupling**: View creation, state management, and layout logic are tightly coupled in the state module
3. **Code Duplication**: Depth texture creation and volume texture creation logic is duplicated across multiple functions
4. **Limited Reusability**: View switching logic is specific to mesh mode and cannot be reused for other view transitions
5. **Testing Challenges**: The large function is difficult to unit test due to multiple dependencies

### Current Function Responsibilities

The `set_mesh_mode_enabled` function currently handles:
- Depth texture creation and validation
- MPR state snapshotting and restoration
- Volume texture creation (both R16Float and Rg8Unorm paths)
- View creation and configuration
- Layout position and size calculations
- Context caching and cleanup

## Proposed Refactoring

### 1. Enhanced Layout System (`layout.rs`)

#### New Methods for Layout Management

```rust
impl<T: LayoutStrategy> Layout<T> {
    /// Replace a view at the specified index with a new view
    pub fn replace_view_at(&mut self, index: usize, new_view: Box<dyn View>) -> Option<Box<dyn View>>
    
    /// Toggle between two view types at a specific index
    pub fn toggle_view_type_at<F>(&mut self, index: usize, view_factory: F) -> bool
    
    /// Get mutable reference to a view for state operations
    pub fn get_view_mut(&mut self, index: usize) -> Option<&mut Box<dyn View>>
    
    /// Check if a view at the given index is of a specific type
    pub fn is_view_type<T: 'static>(&self, index: usize) -> bool
}
```

#### Benefits
- **Encapsulation**: Layout-specific operations are contained within the layout module
- **Reusability**: View replacement logic can be used for any view type transitions
- **Type Safety**: Better type checking for view operations
- **Consistency**: All layout operations follow the same patterns

### 2. Enhanced View System (`view.rs`)

#### State Management Structures

```rust
/// State snapshot for preserving view configuration during mode switches
#[derive(Debug, Clone)]
pub struct ViewState {
    pub window_level: f32,
    pub window_width: f32,
    pub slice_mm: f32,
    pub scale: f32,
    pub translate: [f32; 3],
    pub translate_in_screen_coord: [f32; 3],
}

/// Enhanced View trait with state management capabilities
pub trait StatefulView: View {
    fn save_state(&self) -> Option<ViewState>;
    fn restore_state(&mut self, state: &ViewState) -> bool;
    fn view_type(&self) -> &'static str;
}

/// Factory trait for creating different types of views
pub trait ViewFactory {
    fn create_mesh_view(&self, manager: &mut PipelineManager, pos: (i32, i32), size: (u32, u32)) -> Box<dyn View>;
    fn create_mpr_view(&self, manager: &mut PipelineManager, vol: &CTVolume, orientation: Orientation, pos: (i32, i32), size: (u32, u32)) -> Box<dyn View>;
}
```

#### Benefits
- **State Preservation**: Standardized way to save and restore view states
- **Factory Pattern**: Centralized view creation logic
- **Extensibility**: Easy to add new view types and state properties
- **Testability**: State management can be tested independently

### 3. New ViewManager Module (`view_manager.rs`)

#### Centralized View Transition Management

```rust
/// Manages view transitions and state preservation
pub struct ViewManager {
    saved_states: HashMap<usize, ViewState>,
    factory: Box<dyn ViewFactory>,
}

impl ViewManager {
    /// Save the current state of a view before switching
    pub fn save_view_state(&mut self, index: usize, view: &dyn View)
    
    /// Create a new view and restore its state if available
    pub fn create_view_with_state<F>(&mut self, index: usize, view_creator: F) -> Box<dyn View>
    
    /// Clear saved state for a specific slot
    pub fn clear_state(&mut self, index: usize)
}
```

#### Benefits
- **Centralized Management**: All view state operations in one place
- **Memory Efficiency**: States are only stored when needed
- **Flexibility**: Can handle any number of view slots and types

### 4. Simplified State Implementation

#### Refactored `set_mesh_mode_enabled` Function

```rust
impl State {
    /// Enable or disable mesh mode using the enhanced layout system
    pub fn set_mesh_mode_enabled(&mut self, manager: &mut PipelineManager, enabled: bool) {
        // Early validation (10 lines)
        if self.enable_mesh == enabled { return; }
        // ... validation logic
        
        if enabled {
            // Ensure prerequisites (5 lines)
            if !self.ensure_depth_texture() { return; }
            
            // Save state and create mesh view (5 lines)
            self.save_current_view_state(index);
            let mesh_view = self.create_mesh_view(manager);
            self.layout.replace_view_at(index, mesh_view);
        } else {
            // Create MPR view and restore state (5 lines)
            let vol = self.last_volume.as_ref().unwrap();
            let mpr_view = self.create_mpr_view(manager, vol);
            self.layout.replace_view_at(index, mpr_view);
            self.clear_mesh_context_cache();
        }
    }
    
    // Helper methods (extracted from original function)
    fn ensure_depth_texture(&mut self) -> bool { /* ... */ }
    fn create_mesh_view(&mut self, manager: &mut PipelineManager) -> Box<dyn View> { /* ... */ }
    fn create_mpr_view(&self, manager: &mut PipelineManager, vol: &CTVolume) -> Box<dyn View> { /* ... */ }
}
```

## Implementation Plan

### Phase 1: Foundation (Week 1)
1. **Create ViewState structure** in `view.rs`
2. **Implement StatefulView trait** for GenericMPRView
3. **Add basic state management methods** to existing views
4. **Write unit tests** for state management

### Phase 2: Layout Enhancements (Week 1)
1. **Add view replacement methods** to Layout
2. **Implement view type checking** utilities
3. **Add layout-specific error handling**
4. **Write integration tests** for layout operations

### Phase 3: ViewManager Module (Week 2)
1. **Create ViewManager structure** and implementation
2. **Implement ViewFactory trait** and concrete factories
3. **Add state persistence** and restoration logic
4. **Write comprehensive tests** for view transitions

### Phase 4: State Refactoring (Week 2)
1. **Extract helper methods** from set_mesh_mode_enabled
2. **Integrate ViewManager** into State
3. **Simplify main function** using new abstractions
4. **Update existing tests** and add new ones

### Phase 5: Documentation and Testing (Week 3)
1. **Update API documentation** for all new components
2. **Add performance benchmarks** for view transitions
3. **Create integration tests** for complete workflows
4. **Update user documentation** and examples

## Benefits of Refactoring

### Code Quality Improvements
- **Reduced Complexity**: Main function goes from ~200 lines to ~30 lines
- **Single Responsibility**: Each module has a clear, focused purpose
- **Better Testability**: Components can be tested in isolation
- **Improved Maintainability**: Changes are localized to appropriate modules

### Performance Benefits
- **Reduced Memory Allocation**: State objects are reused when possible
- **Faster View Transitions**: Optimized creation and caching strategies
- **Better Resource Management**: Centralized cleanup and lifecycle management

### Developer Experience
- **Clearer APIs**: Well-defined interfaces for common operations
- **Better Error Messages**: More specific error types and handling
- **Easier Extension**: Simple to add new view types and transitions
- **Consistent Patterns**: All view operations follow the same patterns

## Compatibility and Migration

### Backward Compatibility
- **Existing APIs preserved**: All current public methods remain unchanged
- **Gradual migration**: Old code continues to work during transition
- **Optional features**: New functionality is additive, not replacing

### Migration Strategy
1. **Implement new system alongside existing code**
2. **Add feature flags** for testing new functionality
3. **Gradually migrate** existing code to use new APIs
4. **Remove deprecated code** after thorough testing

## Testing Strategy

### Unit Tests
- **State management**: Save/restore operations for all view types
- **Layout operations**: View replacement and positioning logic
- **Factory methods**: View creation with various parameters
- **Error handling**: Edge cases and failure scenarios

### Integration Tests
- **Complete workflows**: Full mesh mode toggle scenarios
- **Performance tests**: View transition timing and memory usage
- **Cross-platform tests**: Ensure functionality on native and WASM
- **Stress tests**: Rapid view switching and resource management

### Validation Criteria
- **Functionality**: All existing features work identically
- **Performance**: No regression in view transition speed
- **Memory**: No memory leaks or excessive allocation
- **Stability**: No crashes or undefined behavior

## Future Extensibility

### Planned Enhancements
- **Animation Support**: Smooth transitions between view modes
- **View Presets**: Saved configurations for common setups
- **Dynamic Layouts**: Runtime layout strategy switching
- **View Plugins**: External view types and custom renderers

### Architecture Benefits
- **Plugin System**: Easy to add new view types without core changes
- **Configuration**: JSON/YAML-based view and layout configuration
- **Scripting**: Lua/Python bindings for custom view logic
- **Remote Control**: Network-based view management for collaboration

## Conclusion

This refactoring plan addresses the current architectural issues while providing a solid foundation for future enhancements. The modular approach ensures that each component has a clear responsibility, making the codebase more maintainable and extensible.

The implementation can be done incrementally, ensuring that existing functionality remains stable throughout the transition. The enhanced architecture will support the growing complexity of the medical imaging framework while maintaining high performance and reliability standards.

## Related Documents

- [Rendering Architecture Design](rendering-architecture-design.md)
- [Layout Improvements](layout-improvements.md)
- [Mesh Rendering Implementation Plan](mesh-rendering-implementation-plan.md)
- [Source Code Reorganization Plan](source-code-reorganization-plan.md)