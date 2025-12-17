# Unified Rendering Architecture Plan

**Date:** 2025-12-17  
**Status:** Proposed  
**Context:** Unification of Mesh, MPR, and MIP rendering views.

## 1. Executive Summary

This document outlines a plan to unify the rendering architecture for **Mesh**, **MPR** (Multi-Planar Reconstruction), and **MIP** (Maximum Intensity Projection) views. The current implementation suffers from code duplication in resource management, inconsistent bind group layouts, and disparate uniform handling. A unified architecture will improve maintainability, reduce code size, and facilitate the addition of new rendering modes (e.g., Volume Rendering).

## 2. Current Architecture Analysis

### 2.1. Duplication
- **Resource Management**: `MprView` and `MipView` independently manage WGPU bind groups, buffers, and pipeline states, despite sharing the same underlying data (Volume Texture).
- **Uniforms**: `MprViewWgpuImpl` and `MipUniforms` define similar fields (`window_width`, `window_level`, `is_packed_rg8`, `bias`) in different structures and bind group slots.
- **Geometry**: 
  - MPR uses a vertex buffer for a quad.
  - MIP uses a hardcoded triangle strip.
  - Both effectively render a "screen-space" quad.

### 2.2. Inconsistency
- **Bind Group Layouts**:
  - **MPR**: Uses 3 bind groups (0: Texture, 1: Vert Uniforms, 2: Frag Uniforms).
  - **MIP**: Uses 2 bind groups (0: Texture, 1: Uniforms).
- **Rendering Logic**: 
  - `MeshView` uses a `Camera` struct.
  - MPR/MIP manage matrices and slice indices manually.

## 3. Proposed Architecture: Unified Render System

The proposal follows a **Phase-based Refactoring Plan** to unify the rendering architecture without breaking existing functionality.

### Phase 1: Standardization (Data & Resources)
**Goal:** Make MPR and MIP share the same data structures and resource layouts.

1.  **Create `VolumeUniforms`**:
    -   Define a shared struct in `src/rendering/core/volume_uniforms.rs`.
    -   **Fields**: `window_width`, `window_level`, `is_packed_rg8`, `bias`, `transform_matrix` (4x4), `slice_depth` (for MPR), `ray_params` (for MIP).
    -   This ensures both renderers speak the same "language".

2.  **Standardize Bind Group Layouts**:
    -   Adopt a unified layout convention for all Volume Views:
        -   **Group 0 (Data)**: Volume Texture (3D) + Sampler. (Already handled by `RenderContent`).
        -   **Group 1 (View)**: Camera/View Matrix, Projection, Viewport Info.
        -   **Group 2 (Settings)**: Window/Level, Rendering Mode (MIP vs MPR), Slice Index.

3.  **Unified Geometry**:
    -   Create `src/rendering/core/geometry_primitives.rs`.
    -   Provide a standard **Unit Quad** (Vertex Buffer + Index Buffer) to be reused by MPR, MIP, and any future post-processing effects.

### Phase 2: Unified Backend (Logic)
**Goal:** Separate the "View" (window management) from the "Renderer" (WGPU commands).

1.  **Introduce `RenderBackend` Trait**:
    -   Create `src/rendering/view/backend.rs`.
    -   Defines the contract for drawing content:
        ```rust
        trait RenderBackend {
            fn update(&mut self, queue: &wgpu::Queue);
            fn render(&self, pass: &mut wgpu::RenderPass, viewport: Viewport);
            fn resize(&mut self, width: u32, height: u32);
        }
        ```

2.  **Implement Specific Backends**:
    -   `MprBackend`: Wraps current `MprViewWgpuImpl` logic but implements the trait.
    -   `MipBackend`: Wraps current MIP logic.
    -   `MeshBackend`: Wraps `BasicMeshContext`.

3.  **Generic `VolumeView<B: RenderBackend>`**:
    -   Refactor `MprView` and `MipView` into a single `VolumeView` class that holds a backend.
    -   The `VolumeView` handles user interaction (mouse events, window/level changes) and updates the backend.

### Phase 3: Shared Context
**Goal:** Centralize WGPU pipeline creation.

1.  **`VolumeRenderContext`**:
    -   Replace `MprRenderContext` and `MipRenderContext`.
    -   Manages the shared Pipelines and Bind Group Layouts.
    -   Allows switching between MIP and MPR modes instantly by just swapping the pipeline, as the Bind Groups (Data & Uniforms) would now be compatible.

## 4. Execution Plan

1.  **Refactor Uniforms**: Extract common fields from `MprViewWgpuImpl` and `MipUniforms` into a shared struct.
2.  **Align Bind Groups**: Modify MIP shader to match MPR's 3-group layout (or vice-versa; 2 groups is likely more efficient if grouped correctly).
3.  **Extract Geometry**: Move quad generation out of `mpr_render_context.rs` into a shared utility.
4.  **Trait Definition**: Define the `RenderBackend` trait and incrementally migrate views to implement it.
