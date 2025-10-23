use std::sync::Arc;

use crate::{
    core::{array_to_slice, Base, GeometryBuilder},
    data::CTVolume,
    rendering::{Orientation, RenderContent, StatefulView, ViewState},
    Renderable, View,
};

use super::{MprRenderContext, MprViewWgpuImpl};

/// Generic Multi-Planar Reconstruction (MPR) view implementation.
///
/// This is the main implementation of MPR functionality that supports all anatomical
/// orientations. It handles the complete rendering pipeline for medical imaging
/// including coordinate transformations, texture sampling, and window/level processing.
///
/// ## Architecture
///
/// The view maintains several coordinate systems:
/// - **Screen coordinates**: Pixel positions on the display
/// - **UV coordinates**: Texture sampling coordinates (0-1 range)
/// - **Medical coordinates**: Real-world millimeter measurements
/// - **Volume coordinates**: Voxel indices in the 3D dataset
///
/// ## Rendering Pipeline
///
/// 1. **Geometry Setup**: Creates screen-aligned quad for the view
/// 2. **Coordinate Transformation**: Maps screen space to volume space
/// 3. **Texture Sampling**: Samples the 3D volume at the current slice
/// 4. **Window/Level Processing**: Applies CT display windowing
/// 5. **Final Rendering**: Outputs the processed image to the view
pub struct MprView {
    /// Shared render context containing pipeline and shared GPU resources
    render_context: Arc<MprRenderContext>,
    /// WGPU implementation containing per-view GPU resources
    wgpu_impl: Arc<MprViewWgpuImpl>,
    /// Render content containing texture and bind groups
    content: Arc<RenderContent>,
    /// Current slice position (internal units)
    slice: f32,
    /// Screen-space coordinate system base
    base_screen: Base<f32>,
    /// UV texture coordinate system base
    base_uv: Base<f32>,
    /// Current zoom scale factor
    scale: f32,
    /// Translation in view/model coordinates
    translate: [f32; 3],
    /// Pan translation in screen coordinates
    pan: [f32; 3],
    /// View position on screen (top-left corner)
    pos: (i32, i32),
    /// View dimensions (width, height)
    dim: (u32, u32),
}

impl MprView {
    /// Create a new GenericMPRView with the specified parameters.
    ///
    /// Initializes all GPU resources, coordinate systems, and rendering state
    /// needed for MPR visualization. The view is immediately ready for rendering
    /// after creation.
    ///
    /// ## Parameters
    ///
    /// - `manager`: Pipeline manager for GPU resource allocation
    /// - `device`: WGPU device for buffer and texture creation
    /// - `texture`: 3D volume texture containing the medical imaging data
    /// - `vol`: CT volume metadata for coordinate system setup
    /// - `orientation`: Anatomical orientation for this view
    /// - `scale`: Initial zoom level (1.0 = original size)
    /// - `translate`: Initial translation in view coordinates
    /// - `pos`: Initial position on screen
    /// - `dim`: Initial view dimensions
    ///
    /// ## Coordinate System Setup
    ///
    /// The constructor sets up multiple coordinate systems:
    /// 1. Screen-space base for user interaction
    /// 2. UV-space base for texture sampling
    /// 3. Transform matrix linking screen to texture coordinates
    pub fn new(
        render_context: Arc<MprRenderContext>,
        device: &wgpu::Device,
        texture: Arc<RenderContent>,
        vol: &CTVolume,
        orientation: Orientation,
        scale: f32,
        translate: [f32; 3],
        pos: (i32, i32),
        dim: (u32, u32),
    ) -> Self {
        // Get default slice navigation speed for this orientation
        let _s_speed = orientation.default_slice_speed();

        // Build coordinate system bases for this orientation
        let base_screen = orientation.build_base(vol);
        let base_uv = GeometryBuilder::build_uv_base(vol);

        // Initialize view state
        let pan = [0.0, 0.0, 0.0]; // No initial panning
        let slice = 0.0; // Start at center slice

        // Create screen-space transformation matrix
        let mut base_screen_cloned = base_screen.clone();
        // Apply transformations in reverse order (matrix multiplication)
        base_screen_cloned.translate([-pan[0], -pan[1], -pan[2]]);
        base_screen_cloned.translate([0.5, 0.5, 0.0]); // Move back to origin
        base_screen_cloned.scale([scale, scale, 1.0]); // Apply zoom
        base_screen_cloned.translate([-0.5, -0.5, 0.0]); // Center the transformation

        // Create final transformation matrix from screen to UV coordinates
        let transform_matrix = base_screen_cloned.to_base(&base_uv).transpose();

        // Create WGPU implementation with shared context
        let wgpu_impl = Arc::new(MprViewWgpuImpl::new(
            render_context.clone(),
            device,
            texture.clone(),
            transform_matrix,
        ));

        log::info!("Created GenericMPRView with orientation: {:?}, scale: {:?}, translate: {:?}, pos: {:?}, dim: {:?}",
            orientation, scale, translate, pos, dim);

        Self {
            render_context,
            wgpu_impl,
            content: texture,
            slice,
            base_screen,
            base_uv,
            scale,
            translate,
            pan,
            pos,
            dim,
        }
    }

    /// Update the transformation matrix based on current view parameters.
    ///
    /// Recalculates the screen-to-UV coordinate transformation matrix whenever
    /// view parameters change (scale, pan, etc.). This ensures that texture
    /// sampling coordinates remain accurate for the current view state.
    ///
    /// ## Transformation Order
    ///
    /// 1. **Center**: Move to coordinate system center
    /// 2. **Scale**: Apply zoom transformation
    /// 3. **Uncenter**: Move back from center
    /// 4. **Pan**: Apply screen-space translation
    /// 5. **Project**: Transform to UV texture coordinates
    fn update_transform_matrix(&mut self, queue: &wgpu::Queue) {
        let mut base_screen_cloned = self.base_screen.clone();

        // Apply transformations in reverse order due to matrix multiplication
        base_screen_cloned.translate([-self.pan[0], -self.pan[1], -self.pan[2]]);
        base_screen_cloned.translate([0.5, 0.5, 0.0]); // Move back to origin
        base_screen_cloned.scale([self.scale, self.scale, 1.0]); // Apply current zoom
        base_screen_cloned.translate([-0.5, -0.5, 0.0]); // Center the transformation

        // Create final transformation matrix and update GPU uniforms
        let transform_matrix = base_screen_cloned.to_base(&self.base_uv).transpose();
        
        // Update the transformation matrix using the new architecture
        if let Some(wgpu_impl) = Arc::get_mut(&mut self.wgpu_impl) {
            wgpu_impl.update_matrix(queue, *array_to_slice(&transform_matrix.data));
        }
    }
}

impl Drop for MprView {
    /// Clean up GPU resources when the view is dropped.
    ///
    /// Logs the destruction for debugging purposes. The actual GPU resource
    /// cleanup is handled by the RenderContext's Drop implementation.
    fn drop(&mut self) {
        log::debug!("Dropping GenericMPRView - GPU resources will be cleaned up");
    }
}

impl Renderable for MprView {
    /// Update GPU uniforms with current view state.
    ///
    /// Called every frame to ensure GPU shaders have the latest view parameters.
    /// Updates both vertex and fragment shader uniforms with current transformation
    /// matrix, slice position, and other rendering parameters.
    fn update(&mut self, queue: &wgpu::Queue) {
        // Update slice position for volume sampling
        if let Some(wgpu_impl) = Arc::get_mut(&mut self.wgpu_impl) {
            wgpu_impl.update_slice(queue, self.slice);
        }

        // Recalculate transformation matrix if view parameters changed
        self.update_transform_matrix(queue);
    }

    /// Render the MPR view to the current render pass.
    ///
    /// Sets up the rendering pipeline, configures the viewport for this view's
    /// screen region, binds all necessary GPU resources, and issues the draw call
    /// to render the medical imaging slice.
    ///
    /// ## Rendering Steps
    ///
    /// 1. **Pipeline Setup**: Bind the MPR rendering pipeline
    /// 2. **Viewport**: Configure screen region for this view
    /// 3. **Resources**: Bind textures and uniform buffers
    /// 4. **Geometry**: Bind vertex and index buffers
    /// 5. **Draw**: Issue indexed draw call for the quad
    fn render(&mut self, render_pass: &mut wgpu::RenderPass) -> Result<(), wgpu::SurfaceError> {
        // Set the rendering pipeline for MPR visualization (from shared context)
        render_pass.set_pipeline(&self.render_context.render_pipeline);

        // Configure viewport to this view's screen region
        let (x, y) = (self.pos.0 as f32, self.pos.1 as f32);
        let (width, height) = (self.dim.0, self.dim.1);
        render_pass.set_viewport(x, y, width as f32, height as f32, 0.0, 1.0);

        // Bind GPU resources
        // Volume texture (from per-view implementation)
        render_pass.set_bind_group(0, &self.wgpu_impl.texture_bind_group, &[]);
        
        // Per-view uniforms (from WGPU implementation)
        render_pass.set_bind_group(1, &self.wgpu_impl.uniform_vert_bind_group, &[]); // Vertex uniforms
        render_pass.set_bind_group(2, &self.wgpu_impl.uniform_frag_bind_group, &[]); // Fragment uniforms

        // Bind geometry buffers (from shared context)
        render_pass.set_vertex_buffer(0, self.render_context.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.render_context.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        // Draw the screen-aligned quad
        render_pass.draw_indexed(0..self.render_context.num_indices, 0, 0..1);
        Ok(())
    }
}

impl View for MprView {
    /// Get the current position of this view on screen.
    fn position(&self) -> (i32, i32) {
        log::trace!("View position: {:#?}", self.pos);
        self.pos
    }

    /// Get the current dimensions of this view.
    fn dimensions(&self) -> (u32, u32) {
        log::trace!("View dimensions: {:#?}", self.dim);
        self.dim
    }

    /// Move this view to a new position on screen.
    fn move_to(&mut self, pos: (i32, i32)) {
        log::trace!("View move_to: {:#?}", pos);
        self.pos = pos;
    }

    /// Resize this view to new dimensions.
    fn resize(&mut self, dim: (u32, u32)) {
        log::trace!("View resize: {:#?}", dim);
        self.dim = dim;
    }

    /// Get a reference to this view as Any for type introspection.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// Get a mutable reference to this view as Any for type introspection.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl MprView {
    /// Set the window level (brightness) for CT image display.
    pub fn set_window_level(&mut self, window_level: f32) {
        // Update the internal state - will be synced to GPU in next update() call
        if let Some(wgpu_impl) = Arc::get_mut(&mut self.wgpu_impl) {
            wgpu_impl.uniforms.frag.window_level = window_level;
        }
    }

    /// Set the window width (contrast) for CT image display.
    pub fn set_window_width(&mut self, window_width: f32) {
        // Update the internal state - will be synced to GPU in next update() call
        if let Some(wgpu_impl) = Arc::get_mut(&mut self.wgpu_impl) {
            wgpu_impl.uniforms.frag.window_width = window_width;
        }
    }

    /// Set the current slice position in millimeters.
    ///
    /// Converts millimeter units to internal coordinate system units
    /// using the volume's scale factors for accurate positioning.
    pub fn set_slice_mm(&mut self, z: f32) {
        let [_, _, scale_z] = self.base_screen.get_scale_factors();
        self.pan[2] = z / scale_z;
    }

    /// Set the zoom scale factor.
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    /// Set translation in view/model coordinate space.
    pub fn set_translate(&mut self, translate: [f32; 3]) {
        self.translate = translate;
    }

    /// Set translation in screen coordinate space (for panning).
    pub fn set_translate_in_screen_coord(&mut self, translate: [f32; 3]) {
        self.pan = translate;
    }

    /// Pan the view in screen space.
    pub fn set_pan(&mut self, x: f32, y: f32) {
        self.pan[0] = x;
        self.pan[1] = y;
    }

    /// Pan the view by millimeter amounts.
    ///
    /// Converts millimeter units to screen coordinate units using
    /// the volume's scale factors for accurate positioning.
    pub fn set_pan_mm(&mut self, x_mm: f32, y_mm: f32) {
        let [scale_x, scale_y, _] = self.base_screen.get_scale_factors();
        self.pan[0] = x_mm / scale_x;
        self.pan[1] = y_mm / scale_y;
    }

    /// Retrieve current window level from fragment uniforms for state snapshotting.
    pub fn get_window_level(&self) -> f32 {
        self.wgpu_impl.uniforms.frag.window_level
    }

    /// Retrieve current window width from fragment uniforms for state snapshotting.
    pub fn get_window_width(&self) -> f32 {
        self.wgpu_impl.uniforms.frag.window_width
    }

    /// Convert internal pan.z (in screen units) back to millimeters using base scale factors.
    pub fn get_slice_mm(&self) -> f32 {
        let [_, _, scale_z] = self.base_screen.get_scale_factors();
        self.pan[2] * scale_z
    }

    /// Return current screen-space scale factor.
    pub fn get_scale(&self) -> f32 {
        self.scale
    }

    /// Return current screen-space translation vector.
    pub fn get_translate_in_screen_coord(&self) -> [f32; 3] {
        self.pan
    }

    /// Return current view/model-space translation vector.
    pub fn get_translate(&self) -> [f32; 3] {
        self.translate
    }

    /// Convert logical screen coordinates [0,1] to millimeters using the complete transform chain.
    ///
    /// This function applies the same transformation sequence as `update_transform_matrix`
    /// to ensure consistency between CPU coordinate conversion and GPU rendering.
    ///
    /// ## Transformation Order
    ///
    /// 1. **Center**: Move to coordinate system center
    /// 2. **Scale**: Apply zoom transformation  
    /// 3. **Uncenter**: Move back from center
    /// 4. **Pan**: Apply screen-space translation
    /// 5. **Project**: Transform to world millimeter coordinates
    ///
    /// ## Parameters
    ///
    /// * `coord` - Logical screen coordinates in [0,1] range
    ///
    /// ## Returns
    ///
    /// World coordinates in millimeters
    pub fn get_screen_coord_in_mm(&self, coord: [f32; 3]) -> [f32; 3] {
        log::debug!("Converting logical coord to mm: {:?}", coord);
        
        // Clone the base screen matrix to apply transformations
        let mut base_screen_cloned = self.base_screen.clone();

        // Apply the same transformation chain as update_transform_matrix
        // Note: Transformations are applied in reverse order due to matrix multiplication
        base_screen_cloned.translate([-self.pan[0], -self.pan[1], -self.pan[2]]);
        base_screen_cloned.translate([0.5, 0.5, 0.0]); // Move back to origin
        base_screen_cloned.scale([self.scale, self.scale, 1.0]); // Apply current zoom
        base_screen_cloned.translate([-0.5, -0.5, 0.0]); // Center the transformation

        // Convert to millimeters using the transformed matrix
        let transform_matrix = base_screen_cloned.get_matrix();
        let result = transform_matrix.multiply_point3(coord);
        
        log::debug!("Converted coord {:?} to mm: {:?}", coord, result);
        result
    }

    /// set Center of the view at point [x, y, z]
    pub fn set_center_at_point_in_mm(&mut self, p_mm: [f32;3]) {
        // log pan before
        log::info!("set_center_at_point_in_mm: pan={:?}", self.pan);
        let z = -self.pan[2];
        log::info!("set_center_at_point_in_mm: z={:?}", z);
        let center = [0.5, 0.5, z];
        let center_mm = self.get_screen_coord_in_mm(center);
        log::info!("set_center_at_point_in_mm: center_mm={:?}", center_mm);
        let shift = [
            center_mm[0] - p_mm[0],
            center_mm[1] - p_mm[1],
            center_mm[2] - p_mm[2],
        ];
        log::info!("set_center_at_point_in_mm: shift={:?}", shift);
        let [scale_x, scale_y, scale_z] = self.base_screen.get_scale_factors();
        log::info!("set_center_at_point_in_mm: scale={:?}", [scale_x, scale_y, scale_z]);
        // Apply the shift by adding it to the current pan
        self.pan[0] += shift[0] / scale_x;
        self.pan[1] += shift[1] / scale_y; 
        self.pan[2] += shift[2] / scale_z;
    }
}

impl StatefulView for MprView {
    /// Save current MPR view state including window/level, position, scale, and translation.
    ///
    /// Captures all essential parameters needed to restore the view to its current configuration.
    /// This is particularly important for medical imaging workflows where users need to maintain
    /// their viewing settings when switching between different visualization modes.
    fn save_state(&self) -> Option<ViewState> {
        let state = ViewState {
            window_level: self.get_window_level(),
            window_width: self.get_window_width(),
            slice_mm: self.get_slice_mm(),
            scale: self.get_scale(),
            translate: self.get_translate(),
            translate_in_screen_coord: self.get_translate_in_screen_coord(),
            position: self.position(),
            dimensions: self.dimensions(),
        };

        if state.is_valid() {
            log::debug!(
                "Saved MPR view state: window_level={}, window_width={}, scale={}, slice_mm={}",
                state.window_level,
                state.window_width,
                state.scale,
                state.slice_mm
            );
            Some(state)
        } else {
            log::warn!("Failed to save MPR view state: invalid state values");
            None
        }
    }

    /// Restore MPR view state from a saved snapshot.
    ///
    /// Updates all view parameters and triggers transform matrix recalculation.
    /// Validates the incoming state to ensure it contains reasonable values
    /// before applying the changes.
    fn restore_state(&mut self, state: &ViewState) -> bool {
        if !state.is_valid() {
            log::warn!("Cannot restore MPR view state: invalid state values");
            return false;
        }

        // Restore all view parameters
        self.set_window_level(state.window_level);
        self.set_window_width(state.window_width);
        self.set_slice_mm(state.slice_mm);
        self.set_scale(state.scale);
        self.set_translate(state.translate);
        self.set_translate_in_screen_coord(state.translate_in_screen_coord);
        self.move_to(state.position);
        self.resize(state.dimensions);

        // Note: Transform matrix will be updated in the next update() call

        log::debug!(
            "Restored MPR view state: window_level={}, window_width={}, scale={}, slice_mm={}",
            state.window_level,
            state.window_width,
            state.scale,
            state.slice_mm
        );
        true
    }

    /// Return the view type identifier for this MPR view.
    fn view_type(&self) -> &'static str {
        "GenericMPRView"
    }
}

// Optional: keep type aliases for old names
pub type ObliqueView = MprView;
pub type SagittalView = MprView;
pub type TransverseView = MprView;
pub type CoronalView = MprView;
