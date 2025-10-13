# MPR Architecture Transition Plan

## Overview

This document outlines a step-by-step transition plan to migrate the current MPR (Multi-Planar Reconstruction) implementation to the modular architecture described in `mprview_design.md`. The plan ensures minimal disruption, maintains compilation at each step, and preserves all existing functionality.

## Current State Analysis

### Current Architecture
```
MprView {
    ctx: RenderContext,           // Contains ALL GPU resources
    texture: Arc<RenderContent>,  // 3D volume texture
    // ... view state fields
}

RenderContext {
    render_pipeline: Arc<wgpu::RenderPipeline>,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    texture_bind_group: wgpu::BindGroup,
    uniform_*_buffer: wgpu::Buffer,
    uniform_*_bind_group: wgpu::BindGroup,
    uniforms: Uniforms,
}
```

### Target Architecture
```
MprView {
    wgpu_impl: Arc<MprViewWgpuImpl>,  // Shared GPU implementation
    // ... view state fields
}

MprViewWgpuImpl {
    render_context: MprRenderContext,     // Global GPU state
    render_content: Arc<RenderContent>,   // Shared texture data
    // ... per-view GPU resources
}

MprRenderContext {
    pipeline: wgpu::RenderPipeline,       // Global pipeline
    bind_group_layouts: [...],            // Global layouts
    vertex_buffer: wgpu::Buffer,          // Global geometry
    index_buffer: wgpu::Buffer,
}
```

---

## Transition Steps

### Step 1: Create MprRenderContext Struct
**Goal:** Separate global GPU state from per-view state  
**Files:** `src/rendering/view/mpr/render_context.rs`  
**Risk:** Low - New struct, no existing code changes

#### Actions:
1. Create `MprRenderContext` struct with global GPU resources:
   - `wgpu::RenderPipeline`
   - `wgpu::BindGroupLayout`s for textures and uniforms
   - Vertex and index buffers for quad geometry
   - `num_indices` counter

2. Add constructor `MprRenderContext::new()` that:
   - Takes `PipelineManager`, `device`, and target format
   - Creates pipeline and bind group layouts
   - Creates shared vertex/index buffers
   - Returns initialized context

3. Keep existing `RenderContext` unchanged for now

#### Validation:
- Code compiles without errors
- No existing functionality affected
- New struct can be instantiated in tests

---

### Step 2: Extract Pipeline Logic to MprRenderContext
**Goal:** Move pipeline creation logic from RenderContext to MprRenderContext  
**Files:** `src/rendering/view/mpr/render_context.rs`  
**Risk:** Low - Logic extraction, no interface changes

#### Actions:
1. Move pipeline creation logic from `RenderContext::new()` to `MprRenderContext::new()`
2. Move bind group layout creation to `MprRenderContext`
3. Move vertex/index buffer creation to `MprRenderContext`
4. Update `RenderContext::new()` to accept `&MprRenderContext` parameter
5. Make `RenderContext` reference layouts from `MprRenderContext`

#### Validation:
- All existing MPR views still compile and render correctly
- No visual changes in output
- Memory usage remains similar

---

### Step 3: Create MprViewWgpuImpl Struct
**Goal:** Create the per-view GPU implementation struct  
**Files:** `src/rendering/view/mpr/mod.rs` (new file)  
**Risk:** Low - New struct, no existing code changes

#### Actions:
1. Create new `mod.rs` file in `src/rendering/view/mpr/`
2. Define `MprViewWgpuImpl` struct with:
   - `render_context: MprRenderContext`
   - `render_content: Arc<RenderContent>`
   - `vertex_uniform_buffer: wgpu::Buffer`
   - `fragment_uniform_buffer: wgpu::Buffer`
   - `texture_bind_group: wgpu::BindGroup`
   - `vertex_uniform_bind_group: wgpu::BindGroup`
   - `fragment_uniform_bind_group: wgpu::BindGroup`
   - `base_screen: Base<f32>`
   - `base_uv: Base<f32>`

3. Add constructor `MprViewWgpuImpl::new()` that:
   - Takes `MprRenderContext`, `Arc<RenderContent>`, device, and transform matrix
   - Creates per-view uniform buffers and bind groups
   - Stores coordinate system bases
   - Returns initialized implementation

4. Add methods for accessing internal components:
   - `render_context()` -> `&MprRenderContext`
   - `render_content()` -> `&Arc<RenderContent>`
   - `update_uniforms()` for uploading uniform data

#### Validation:
- Code compiles without errors
- New struct can be instantiated in tests
- No existing functionality affected

---

### Step 4: Refactor RenderContext to Use MprViewWgpuImpl
**Goal:** Transform current RenderContext into MprViewWgpuImpl pattern  
**Files:** `src/rendering/view/mpr/render_context.rs`, `src/rendering/view/mpr/mpr_view.rs`  
**Risk:** Medium - Significant refactoring, but isolated to MPR module

#### Actions:
1. Update `RenderContext::new()` to create `MprViewWgpuImpl` internally
2. Make `RenderContext` a wrapper around `MprViewWgpuImpl`
3. Delegate all `RenderContext` methods to `MprViewWgpuImpl`
4. Maintain exact same public interface for `RenderContext`
5. Update imports in `mpr_view.rs` to use new module structure

#### Validation:
- All existing MPR views compile and render identically
- No changes to `MprView` interface yet
- Performance remains the same

---

### Step 5: Update MprView to Use Arc<MprViewWgpuImpl>
**Goal:** Change MprView to use Arc-wrapped implementation  
**Files:** `src/rendering/view/mpr/mpr_view.rs`  
**Risk:** Medium - Changes to MprView structure and methods

#### Actions:
1. Replace `ctx: RenderContext` with `wgpu_impl: Arc<MprViewWgpuImpl>`
2. Update `MprView::new()` to:
   - Accept `Arc<MprViewWgpuImpl>` parameter
   - Remove RenderContext creation logic
   - Store Arc reference

3. Update all `MprView` methods to access GPU resources through `wgpu_impl`
4. Update `update()` and `render()` methods to use new interface
5. Maintain exact same public interface for `MprView`

#### Validation:
- Code compiles without errors
- All MPR view methods work correctly
- No visual changes in rendering

---

### Step 6: Update Constructor Calls in state.rs
**Goal:** Update all MprView creation sites to use new architecture  
**Files:** `src/rendering/core/state.rs`  
**Risk:** Medium - Changes to view creation logic

#### Actions:
1. Create shared `MprRenderContext` in `State::load_data_from_ct_volume()`
2. Update `MprView::new()` calls to:
   - First create `MprViewWgpuImpl` with shared context
   - Wrap in `Arc::new()`
   - Pass to `MprView::new()`

3. Update helper methods like `create_mpr_view_for_slot()` to use new pattern
4. Ensure proper resource sharing between multiple MPR views

#### Validation:
- All MPR views in 2x2 grid render correctly
- Resource sharing works (multiple views, same texture)
- Memory usage is optimized through sharing

---

### Step 7: Build and Test Comprehensive Functionality
**Goal:** Ensure all MPR functionality works with new architecture  
**Files:** All MPR-related files  
**Risk:** Low - Validation step

#### Actions:
1. Run `cargo build` to ensure compilation
2. Run `cargo test` to verify all tests pass
3. Test all MPR view interactions:
   - Window/level adjustments
   - Slice navigation
   - Pan and zoom operations
   - View state save/restore
   - Orientation switching

4. Test resource sharing scenarios:
   - Multiple MPR views with same texture
   - Memory usage optimization
   - Performance consistency

#### Validation:
- All tests pass
- All interactive features work correctly
- Performance is maintained or improved
- Memory usage is optimized

---

### Step 8: Cleanup and Documentation
**Goal:** Remove old code and update documentation  
**Files:** Various  
**Risk:** Low - Cleanup step

#### Actions:
1. Remove old `RenderContext` struct if no longer needed
2. Clean up unused imports and dead code
3. Update module exports in `mod.rs` files
4. Update code comments and documentation
5. Add examples of new architecture usage

#### Validation:
- Code compiles cleanly without warnings
- Documentation is accurate and helpful
- Examples work correctly

---

## Risk Mitigation

### Compilation Safety
- Each step maintains compilation
- Changes are isolated to specific modules
- Existing interfaces preserved until final steps

### Functionality Preservation
- All public APIs remain unchanged until step 5
- Visual output identical throughout transition
- All medical imaging features preserved

### Rollback Strategy
- Each step can be reverted independently
- Git commits at each major milestone
- Backup of working state before each step

### Testing Strategy
- Compile after each step
- Visual verification of MPR rendering
- Automated tests for core functionality
- Manual testing of interactive features

---

## Benefits After Transition

1. **Resource Sharing**: Multiple MPR views share `MprRenderContext` and `RenderContent`
2. **Memory Efficiency**: Reduced GPU memory usage through Arc-based sharing
3. **Maintainability**: Clear separation of concerns between global and per-view state
4. **Scalability**: Easy to add new MPR view types or orientations
5. **Consistency**: Same architectural patterns as MIP rendering system
6. **Thread Safety**: Arc-based sharing enables multi-threaded access

---

## Timeline Estimate

- **Step 1-2**: 2-3 hours (struct creation and logic extraction)
- **Step 3-4**: 3-4 hours (implementation struct and refactoring)
- **Step 5-6**: 2-3 hours (MprView updates and constructor changes)
- **Step 7-8**: 1-2 hours (testing and cleanup)

**Total**: 8-12 hours of focused development time

---

*This transition plan ensures a smooth migration to the modular MPR architecture while maintaining all existing functionality and improving resource efficiency.*