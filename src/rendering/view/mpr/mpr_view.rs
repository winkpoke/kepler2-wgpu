use std::sync::Arc;

use glam::{Mat4, Vec3};

use crate::{
    Renderable, View,
    core::{Base, GeometryBuilder, WindowLevel, error::{KeplerResult, MprError}}, 
    data::CTVolume,
     rendering::{Orientation, RenderContent, StatefulView, ViewState}
};

use super::{MprRenderContext, MprViewWgpuImpl};
use crate::rendering::view::layout::compute_aspect_fit;

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
    /// WGPU implementation containing per-view GPU resources
    wgpu_impl: MprViewWgpuImpl,
    /// Current slice position (internal units)
    slice: f32,
    /// Screen-space coordinate system base
    base_screen: Mat4,
    /// UV texture coordinate system base
    base_uv: Mat4,
    /// Current zoom scale factor
    scale: f32,
    /// Pan translation in screen coordinates
    pan: Vec3,
    /// View position on screen (top-left corner)
    pos: (i32, i32),
    /// View dimensions (width, height)
    dim: (u32, u32),
    /// Window/level parameters for CT display
    window_level: WindowLevel,
    /// Anatomical orientation for this view (Axial, Coronal, Sagittal)
    orientation: Orientation,
    /// Physical in-plane content width in millimeters
    content_w_mm: f32,
    /// Physical in-plane content height in millimeters
    content_h_mm: f32,
    /// Uniform viewport padding in pixels
    padding_px: u32,
}

impl MprView {
    /// Medical imaging parameter bounds for validation
    const MIN_SCALE: f32 = 0.01;        // 1% zoom minimum
    const MAX_SCALE: f32 = 100.0;       // 100x zoom maximum
    const MAX_PAN_DISTANCE: f32 = 10000.0; // Maximum pan distance in mm
    
    /// Validate and clamp medical imaging parameters for safety and correctness
    fn validate_and_clamp_params(
        scale: f32,
        translate: Vec3,
        pos: (i32, i32),
        dim: (u32, u32),
    ) -> ((f32, Vec3), (i32, i32), (u32, u32)) {
        // Validate and clamp scale
        let validated_scale = if !scale.is_finite() || scale <= 0.0 {
            log::warn!("Invalid scale {} replaced with default 1.0", scale);
            1.0
        } else {
            let clamped = scale.clamp(Self::MIN_SCALE, Self::MAX_SCALE);
            if (clamped - scale).abs() > f32::EPSILON {
                log::warn!("Scale {} clamped to {}", scale, clamped);
            }
            clamped
        };
        
        // Validate and clamp translation
        let validated_translate = if !translate.is_finite() {
             log::warn!("Invalid translate coordinate {} replaced with 0.0", translate);
             Vec3::ZERO
        } else {
             translate.clamp(Vec3::splat(-Self::MAX_PAN_DISTANCE), Vec3::splat(Self::MAX_PAN_DISTANCE))
        };
        
        // Validate position bounds
        const MAX_POSITION: i32 = 100_000;
        const MIN_POSITION: i32 = -100_000;
        let validated_pos = (
            pos.0.clamp(MIN_POSITION, MAX_POSITION),
            pos.1.clamp(MIN_POSITION, MAX_POSITION)
        );
        
        // Validate dimensions
        const MAX_DIMENSION: u32 = 16384;
        const MIN_DIMENSION: u32 = 1;
        let validated_dim = (
            dim.0.clamp(MIN_DIMENSION, MAX_DIMENSION),
            dim.1.clamp(MIN_DIMENSION, MAX_DIMENSION)
        );
        
        ((validated_scale, validated_translate), validated_pos, validated_dim)
    }

    /// Create a new MPR view with the specified parameters.
    /// 
    /// # Arguments
    /// 
    /// * `render_context` - Shared GPU rendering context
    /// * `device` - WGPU device for creating GPU resources
    /// * `texture` - Shared texture content for rendering
    /// * `vol` - CT volume data for coordinate system setup
    /// * `orientation` - Anatomical orientation (Transverse, Coronal, Sagittal, Oblique)
    /// * `window_level` - Window/level configuration with bias settings
    /// * `scale` - Initial zoom scale factor (must be positive and finite)
    /// * `translate` - Initial translation in view coordinates (must be finite)
    /// * `pos` - Initial position on screen (top-left corner in pixels)
    /// * `dim` - Initial dimensions (width, height in pixels, must be non-zero)
    /// 
    /// # Returns
    /// 
    /// A new MprView with validated and clamped parameters
    pub fn new(
        render_context: Arc<MprRenderContext>,
        device: &wgpu::Device,
        texture: Arc<RenderContent>,
        vol: &CTVolume,
        orientation: Orientation,
        window_level: WindowLevel,
        scale: f32,
        translate: [f32; 3],
        pos: (i32, i32),
        dim: (u32, u32),
    ) -> Self {
        let translate_vec = Vec3::from_array(translate);

        // Validate and clamp all input parameters
        let ((validated_scale, validated_translate), validated_pos, validated_dim) = 
            Self::validate_and_clamp_params(scale, translate_vec, pos, dim);
        // Build coordinate system bases for this orientation
        let base_screen_legacy = orientation.build_base(vol);
        let base_uv_legacy = GeometryBuilder::build_uv_base(vol);

        let base_screen = base_screen_legacy.matrix;
        let base_uv = base_uv_legacy.matrix;

        // Initialize view state with validated parameters
        let pan = validated_translate;
        let slice = 0.0; // Start at center slice

        // Create screen-space transformation matrix
        // Apply transformations in reverse order (matrix multiplication)
        // M_final = M_initial * T_pan * T_center * S_scale * T_uncenter
        let t_pan = Mat4::from_translation(-pan);
        let t_center = Mat4::from_translation(Vec3::new(0.5, 0.5, 0.0));
        let s_scale = Mat4::from_scale(Vec3::splat(validated_scale).with_z(1.0));
        let t_uncenter = Mat4::from_translation(Vec3::new(-0.5, -0.5, 0.0));
        
        let transform_matrix_screen = base_screen * t_pan * t_center * s_scale * t_uncenter;

        // Create final transformation matrix from screen to UV coordinates
        let transform_matrix = base_uv.inverse() * transform_matrix_screen;

        // Create WGPU implementation with shared context
        let wgpu_impl = MprViewWgpuImpl::new(
            render_context,
            device,
            texture,
            transform_matrix,
        );

        log::info!("Created MprView with orientation: {:?}, scale: {:?}, translate: {:?}, pos: {:?}, dim: {:?}",
            orientation, validated_scale, validated_translate, validated_pos, validated_dim);

        // Compute physical in-plane content size in millimeters for aspect fitting
        let (nx, ny, nz) = (vol.dimensions.0 as f32, vol.dimensions.1 as f32, vol.dimensions.2 as f32);
        let (nx, ny, nz) = (nx * vol.voxel_spacing.0, ny * vol.voxel_spacing.1, nz * vol.voxel_spacing.2);
        let d = (nx + ny + nz) / 3.0;
        // Use voxel counts to match isotropic in-plane scaling of current geometry bases
        let (content_w_mm, content_h_mm) = match orientation {
            Orientation::Transverse => (d, d),
            Orientation::Coronal => (d, d),
            Orientation::Sagittal => (d, d),
            Orientation::Oblique => (d, d),
        };
        log::info!("[{:?}]: Content size in mm: {:?}", orientation, (content_w_mm, content_h_mm));

        Self {
            wgpu_impl,
            slice,
            base_screen,
            base_uv,
            scale: validated_scale,
            pan,
            pos: validated_pos,
            dim: validated_dim,
            window_level,  // Use provided WindowLevel with configured bias
            orientation,   // Store orientation for cross-sectional linking
            content_w_mm,
            content_h_mm,
            padding_px: 0,
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
    fn update_transform_matrix(&mut self) {
        // M_final = M_initial * T_pan * T_center * S_scale * T_uncenter
        let t_pan = Mat4::from_translation(-self.pan);
        let t_center = Mat4::from_translation(Vec3::new(0.5, 0.5, 0.0));
        let s_scale = Mat4::from_scale(Vec3::splat(self.scale).with_z(1.0));
        let t_uncenter = Mat4::from_translation(Vec3::new(-0.5, -0.5, 0.0));
        
        let transform_matrix_screen = self.base_screen * t_pan * t_center * s_scale * t_uncenter;
        
        // Create final transformation matrix and update GPU uniforms
        let transform_matrix = self.base_uv.inverse() * transform_matrix_screen;
        
        // Set the transformation matrix using the new architecture
        self.wgpu_impl.set_matrix(transform_matrix.to_cols_array());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Function-level comment: 验证参数校验与钳制逻辑在异常与边界输入下的行为
    #[test]
    fn test_validate_and_clamp_params() {
        let scale = -5.0; // 非法，需替换为默认并钳制
        let translate = Vec3::new(f32::INFINITY, -20_000.0, 0.0);
        let pos = (200_000, -200_000);
        let dim = (0, 200_000);
        let ((s, t), p, d) = MprView::validate_and_clamp_params(scale, translate, pos, dim);
        assert!(s >= MprView::MIN_SCALE && s <= MprView::MAX_SCALE);
        assert!(t.x.is_finite() && t.x.abs() <= MprView::MAX_PAN_DISTANCE);
        assert!(t.y.abs() <= MprView::MAX_PAN_DISTANCE);
        assert_eq!(p.0, 100_000);
        assert_eq!(p.1, -100_000);
        assert!(d.0 >= 1 && d.1 >= 1);
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
        // Synchronize window/level settings with WGPU implementation only if dirty
        if self.window_level.is_dirty() {
            // Upload raw window/level (HU); bias handled in shader for RG8 path
            let window_width = self.window_level.window_width();
            let window_level = self.window_level.window_level();
            self.wgpu_impl.set_window_level(window_level);
            self.wgpu_impl.set_window_width(window_width);
            self.window_level.mark_clean();
        }
        
        // Set slice position for volume sampling
        self.wgpu_impl.set_slice(self.slice);

        // Recalculate transformation matrix if view parameters changed
        self.update_transform_matrix();
        
        // Update GPU buffers with all current uniform values
        self.wgpu_impl.update_uniforms_buffers(queue);
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
        // Set the rendering pipeline for MPR visualization (from per-view implementation)
        render_pass.set_pipeline(&self.wgpu_impl.render_context.render_pipeline);

        // Configure viewport to this view's screen region
        // Compute fitted viewport to preserve in-plane aspect ratio
        let (x0, y0) = (self.pos.0 as f32, self.pos.1 as f32);
        let (w, h) = (self.dim.0, self.dim.1);
        if let Some(fit) = compute_aspect_fit(w, h, self.content_w_mm, self.content_h_mm, self.padding_px) {
            render_pass.set_viewport(x0 + fit.x, y0 + fit.y, fit.w, fit.h, 0.0, 1.0);
        } else {
            // Fallback safe viewport
            render_pass.set_viewport(x0, y0, 1.0, 1.0, 0.0, 1.0);
        }

        // Bind GPU resources
        // Volume texture (from per-view implementation)
        render_pass.set_bind_group(0, &self.wgpu_impl.texture_bind_group, &[]);
        
        // Per-view uniforms (from WGPU implementation)
        render_pass.set_bind_group(1, &self.wgpu_impl.uniform_vert_bind_group, &[]); // Vertex uniforms
        render_pass.set_bind_group(2, &self.wgpu_impl.uniform_frag_bind_group, &[]); // Fragment uniforms

        // Bind geometry buffers (from shared context)
        render_pass.set_vertex_buffer(0, self.wgpu_impl.render_context.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.wgpu_impl.render_context.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        // Draw the screen-aligned quad
        render_pass.draw_indexed(0..self.wgpu_impl.render_context.num_indices, 0, 0..1);
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
        
        // Validate position bounds (allow negative positions for off-screen views)
        const MAX_POSITION: i32 = 100_000;
        const MIN_POSITION: i32 = -100_000;
        
        if pos.0 < MIN_POSITION || pos.0 > MAX_POSITION || 
           pos.1 < MIN_POSITION || pos.1 > MAX_POSITION {
            log::warn!("Invalid position {:?}, clamping to bounds", pos);
            self.pos = (
                pos.0.clamp(MIN_POSITION, MAX_POSITION),
                pos.1.clamp(MIN_POSITION, MAX_POSITION)
            );
        } else {
            self.pos = pos;
        }
    }

    /// Resize this view to new dimensions.
    fn resize(&mut self, dim: (u32, u32)) {
        log::trace!("View resize: {:#?}", dim);
        
        // Validate dimensions (must be positive and reasonable)
        const MAX_DIMENSION: u32 = 16384; // 16K resolution limit
        const MIN_DIMENSION: u32 = 1;     // Minimum 1 pixel
        
        if dim.0 == 0 || dim.1 == 0 || dim.0 > MAX_DIMENSION || dim.1 > MAX_DIMENSION {
            log::warn!("Invalid dimensions {:?}, clamping to bounds", dim);
            self.dim = (
                dim.0.clamp(MIN_DIMENSION, MAX_DIMENSION),
                dim.1.clamp(MIN_DIMENSION, MAX_DIMENSION)
            );
        } else {
            self.dim = dim;
        }
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
    /// 
    /// # Arguments
    /// 
    /// * `window_level` - The window level value (must be finite)
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` - If the value is valid
    /// * `Err(MprError::InvalidWindowLevel)` - If the value is NaN or infinite
    pub fn set_window_level(&mut self, window_level: f32) -> KeplerResult<()> {
        self.window_level.set_window_level(window_level)
    }

    /// Set the window width (contrast) for CT image display.
    /// 
    /// # Arguments
    /// 
    /// * `window_width` - The window width value (must be positive and finite)
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` - If the value is valid
    /// * `Err(MprError::InvalidWindowWidth)` - If the value is not positive or is NaN/infinite
    pub fn set_window_width(&mut self, window_width: f32) -> KeplerResult<()> {
        self.window_level.set_window_width(window_width)
    }

    /// Set the current slice position in millimeters.
    ///
    /// Converts millimeter units to internal coordinate system units
    /// using the volume's scale factors for accurate positioning.
    /// 
    /// # Arguments
    /// 
    /// * `z` - Slice position in millimeters (must be finite)
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` - If the position is valid
    /// * `Err(MprError::InvalidSlicePosition)` - If the position is NaN or infinite
    pub fn set_slice_mm(&mut self, z: f32) -> KeplerResult<()> {
        if !z.is_finite() {
            log::error!("Invalid slice position: {} (must be finite)", z);
            return Err(MprError::InvalidSlicePosition(z).into());
        }
        
        let scale_z = self.base_screen.col(2).length();
        
        // Validate scale factor
        if !scale_z.is_finite() || scale_z == 0.0 {
            log::error!("Invalid scale factor for Z axis: {}", scale_z);
            return Err(MprError::InvalidTransformation.into());
        }
        
        let new_pan_z = z / scale_z;
        
        // Validate the result
        if !new_pan_z.is_finite() {
            log::error!("Invalid pan calculation result: {}", new_pan_z);
            return Err(MprError::InvalidSlicePosition(z).into());
        }
        
        log::debug!("Setting slice position to: {} mm (pan_z: {})", z, new_pan_z);
        self.pan.z = new_pan_z;
        Ok(())
    }

    pub fn set_slice(&mut self, z: f32) -> KeplerResult<()> {
        if !z.is_finite() {
            log::error!("Invalid slice position: {} (must be finite)", z);
            return Err(MprError::InvalidSlicePosition(z).into());
        }
        self.pan.z = z;
        Ok(())
    }

    /// Set the zoom scale factor.
    /// 
    /// # Arguments
    /// 
    /// * `scale` - Scale factor (must be positive and finite)
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` - If the scale is valid
    /// * `Err(MprError::InvalidScale)` - If the scale is not positive or is NaN/infinite
    pub fn set_scale(&mut self, scale: f32) -> KeplerResult<()> {
        if !scale.is_finite() || scale <= 0.0 {
            log::error!("Invalid scale: {} (must be positive and finite)", scale);
            return Err(MprError::InvalidScale(scale).into());
        }
        
        let clamped_scale = scale.clamp(Self::MIN_SCALE, Self::MAX_SCALE);
        if (clamped_scale - scale).abs() > f32::EPSILON {
            log::warn!("Scale {} clamped to {}", scale, clamped_scale);
        }
        
        log::debug!("Setting scale to: {}", clamped_scale);
        self.scale = clamped_scale;
        Ok(())
    }

    /// Set translation in screen coordinate space (for panning).
    /// 
    /// # Arguments
    /// 
    /// * `pan` - Pan coordinates [x, y, z] (all values must be finite)
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` - If all coordinates are valid
    /// * `Err(MprError::InvalidPanCoordinates)` - If any coordinate is NaN or infinite
    pub fn set_translate_in_screen_coord(&mut self, pan: [f32; 3]) -> KeplerResult<()> {
        let pan_vec = Vec3::from_array(pan);
        if !pan_vec.is_finite() {
             log::error!("Invalid pan coordinates: {} (must be finite)", pan_vec);
             return Err(MprError::InvalidPanCoordinates(pan).into());
        }
        
        let clamped_pan = pan_vec.clamp(Vec3::splat(-Self::MAX_PAN_DISTANCE), Vec3::splat(Self::MAX_PAN_DISTANCE));
        if (clamped_pan - pan_vec).length_squared() > f32::EPSILON {
             log::warn!("Pan coordinates {} clamped to {}", pan_vec, clamped_pan);
        }
        
        log::debug!("Setting pan coordinates to: {}", clamped_pan);
        self.pan = clamped_pan;
        Ok(())
    }

    /// Pan the view in screen space.
    /// 
    /// # Arguments
    /// 
    /// * `x` - X coordinate (must be finite)
    /// * `y` - Y coordinate (must be finite)
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` - If coordinates are valid
    /// * `Err(MprError::InvalidPanCoordinates)` - If any coordinate is NaN or infinite
    pub fn set_pan(&mut self, x: f32, y: f32) -> KeplerResult<()> {
        if !x.is_finite() {
            log::error!("Invalid pan X coordinate: {} (must be finite)", x);
            return Err(MprError::InvalidPanCoordinates([x, y, self.pan.z]).into());
        }
        
        if !y.is_finite() {
            log::error!("Invalid pan Y coordinate: {} (must be finite)", y);
            return Err(MprError::InvalidPanCoordinates([x, y, self.pan.z]).into());
        }
        
        let clamped_x = x.clamp(-Self::MAX_PAN_DISTANCE, Self::MAX_PAN_DISTANCE);
        let clamped_y = y.clamp(-Self::MAX_PAN_DISTANCE, Self::MAX_PAN_DISTANCE);
        
        if (clamped_x - x).abs() > f32::EPSILON {
            log::warn!("Pan X coordinate {} clamped to {}", x, clamped_x);
        }
        if (clamped_y - y).abs() > f32::EPSILON {
            log::warn!("Pan Y coordinate {} clamped to {}", y, clamped_y);
        }
        
        log::debug!("Setting pan to: ({}, {})", clamped_x, clamped_y);
        self.pan.x = clamped_x;
        self.pan.y = clamped_y;
        Ok(())
    }

    /// Pan the view by millimeter amounts.
    ///
    /// Converts millimeter units to screen coordinate units using
    /// the volume's scale factors for accurate positioning.
    /// 
    /// # Arguments
    /// 
    /// * `x_mm` - X coordinate in millimeters (must be finite)
    /// * `y_mm` - Y coordinate in millimeters (must be finite)
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` - If coordinates are valid
    /// * `Err(MprError::InvalidPanCoordinates)` - If any coordinate is NaN or infinite
    /// * `Err(MprError::InvalidTransformation)` - If scale factors are invalid
    pub fn set_pan_mm(&mut self, x_mm: f32, y_mm: f32) -> KeplerResult<()> {
        if !x_mm.is_finite() {
            log::error!("Invalid pan X coordinate (mm): {} (must be finite)", x_mm);
            return Err(MprError::InvalidPanCoordinates([x_mm, y_mm, 0.0]).into());
        }
        
        if !y_mm.is_finite() {
            log::error!("Invalid pan Y coordinate (mm): {} (must be finite)", y_mm);
            return Err(MprError::InvalidPanCoordinates([x_mm, y_mm, 0.0]).into());
        }
        
        let scale_x = self.base_screen.col(0).length();
        let scale_y = self.base_screen.col(1).length();
        
        // Validate scale factors
        if !scale_x.is_finite() || scale_x == 0.0 {
            log::error!("Invalid scale factor for X axis: {}", scale_x);
            return Err(MprError::InvalidTransformation.into());
        }
        
        if !scale_y.is_finite() || scale_y == 0.0 {
            log::error!("Invalid scale factor for Y axis: {}", scale_y);
            return Err(MprError::InvalidTransformation.into());
        }
        
        let new_pan_x = x_mm / scale_x;
        let new_pan_y = y_mm / scale_y;
        
        // Validate results
        if !new_pan_x.is_finite() || !new_pan_y.is_finite() {
            log::error!("Invalid pan calculation results: ({}, {})", new_pan_x, new_pan_y);
            return Err(MprError::InvalidPanCoordinates([new_pan_x, new_pan_y, self.pan.z]).into());
        }
        
        log::debug!("Setting pan to: ({} mm, {} mm) -> ({}, {})", x_mm, y_mm, new_pan_x, new_pan_y);
        self.pan.x = new_pan_x;
        self.pan.y = new_pan_y;
        Ok(())
    }

    /// Retrieve current window level for state snapshotting.
    pub fn get_window_level(&self) -> f32 {
        self.window_level.window_level()
    }

    /// Retrieve current window width for state snapshotting.
    pub fn get_window_width(&self) -> f32 {
        self.window_level.window_width()
    }

    /// Convert internal pan.z (in screen units) back to millimeters using base scale factors.
    pub fn get_slice_mm(&self) -> f32 {
        let scale_z = self.base_screen.col(2).length();
        self.pan.z * scale_z
    }

    /// Return current screen-space scale factor.
    pub fn get_scale(&self) -> f32 {
        self.scale
    }

    /// Return current screen-space translation vector.
    pub fn get_translate_in_screen_coord(&self) -> [f32; 3] {
        self.pan.to_array()
    }

    /// Get the anatomical orientation of this MPR view.
    /// 
    /// Returns the orientation (Axial, Coronal, Sagittal) that determines
    /// which anatomical plane this view displays. This is essential for
    /// cross-sectional view linking functionality.
    pub fn get_orientation(&self) -> &Orientation {
        &self.orientation
    }

    pub fn get_base(&self) -> Mat4 {
        // Apply the same transformation chain as update_transform_matrix
        // Note: Transformations are applied in reverse order due to matrix multiplication
        let t_pan = Mat4::from_translation(-self.pan);
        let t_center = Mat4::from_translation(Vec3::new(0.5, 0.5, 0.0));
        let s_scale = Mat4::from_scale(Vec3::splat(self.scale).with_z(1.0));
        let t_uncenter = Mat4::from_translation(Vec3::new(-0.5, -0.5, 0.0));
        
        self.base_screen * t_pan * t_center * s_scale * t_uncenter
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
    pub fn screen_coord_to_world(&self, coord: [f32; 3]) -> [f32; 3] {
        log::debug!("Converting logical coord to mm: {:?}", coord);
        
        let current_base = self.get_base();

        // Convert to millimeters using the transformed matrix
        let result = current_base.transform_point3(Vec3::from_array(coord));
        
        log::debug!("Converted coord {:?} to mm: {}", coord, result);
        result.to_array()
    }

    pub fn world_coord_to_screen(&self, world_coord: [f32; 3]) -> [f32; 3] {
        let current_base = self.get_base();
        let transform_matrix = current_base.inverse();
        let result = transform_matrix.transform_point3(Vec3::from_array(world_coord));
        result.to_array()
    }

    /// set Center of the view at point [x, y, z]
    /// Centers the view at the specified point in millimeter coordinates.
    /// 
    /// This method performs coordinate transformation from world space (mm) to screen space
    /// and updates the pan values accordingly. It includes comprehensive input validation
    /// and safe matrix operations.
    /// 
    /// # Arguments
    /// 
    /// * `p_mm` - Point in millimeter coordinates to center the view on
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` - If the operation succeeds
    /// * `Err(MprError)` - If input validation fails or matrix operations are invalid
    /// 
    /// # Safety
    /// 
    /// This method validates all inputs and handles matrix inversion failures gracefully.
    /// It will not panic on invalid coordinates or singular matrices.
    pub fn set_center_at_point_in_mm(&mut self, p_mm: [f32; 3]) -> KeplerResult<[f32; 3]> {
        // Input validation: check for NaN and infinite values
        for (i, &coord) in p_mm.iter().enumerate() {
            if !coord.is_finite() {
                log::error!("Invalid coordinate at index {}: {} (must be finite)", i, coord);
                return Err(MprError::CoordinateOutOfBounds(p_mm).into());
            }
        }

        log::debug!("set_center_at_point_in_mm: target point={:?}", p_mm);
        log::debug!("set_center_at_point_in_mm: current pan={}", self.pan);
        
        self.pan = Vec3::ZERO;
        // let z = -self.pan[2];
        // let center = [0.5, 0.5, z];
        let center = [0.5, 0.5, 0.0];
        let center_mm = self.screen_coord_to_world(center);
        
        log::debug!("set_center_at_point_in_mm: center_mm={:?}", center_mm);
        
        // Calculate shift vector with validation
        let shift = [
            center_mm[0] - p_mm[0],
            center_mm[1] - p_mm[1],
            center_mm[2] - p_mm[2],
        ];
        
        // Validate shift vector
        for (i, &s) in shift.iter().enumerate() {
            if !s.is_finite() {
                log::error!("Invalid shift calculation at index {}: {}", i, s);
                return Err(MprError::InvalidTransformation.into());
            }
        }
        
        log::debug!("set_center_at_point_in_mm: shift={:?}", shift);
        
        let current_base = self.get_base();
        
        // Safe matrix operations with proper error handling
        let mut transform_matrix = current_base;
        
        // Clear translation components (set to zero)
        transform_matrix.w_axis.x = 0.0;
        transform_matrix.w_axis.y = 0.0;
        transform_matrix.w_axis.z = 0.0;
        
        // Attempt matrix inversion with proper error handling
        let inverse_matrix = transform_matrix.inverse();
        if !inverse_matrix.is_finite() {
            log::error!("Failed to invert transformation matrix - matrix is singular");
            return Err(MprError::InvalidTransformation.into());
        }
        
        // Apply transformation
        let shift_vec = Vec3::from_array(shift);
        let result = inverse_matrix.transform_point3(shift_vec);
        
        // Validate transformation result
        if !result.is_finite() {
             log::error!("Invalid transformation result: {}", result);
             return Err(MprError::InvalidTransformation.into());
        }
        
        log::debug!("set_center_at_point_in_mm: transformation result={}", result);
        
        // Update pan values with bounds checking
        // let new_pan = [
        //     self.pan[0] + result[0],
        //     self.pan[1] + result[1],
        //     self.pan[2] + result[2],
        // ];

        let new_pan = Vec3::new(
            result.x / self.scale,
            result.y / self.scale,
            result.z,
        );
        
        // Validate new pan values
        if !new_pan.is_finite() {
            log::error!("Invalid pan value: {}", new_pan);
            return Err(MprError::InvalidPanCoordinates(new_pan.to_array()).into());
        }
        
        self.pan = new_pan;
        log::debug!("set_center_at_point_in_mm: updated pan={}", self.pan);
        
        Ok(shift)
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
            bias: self.window_level.bias(),
            slice_mm: self.get_slice_mm(),
            scale: self.get_scale(),
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
        let _ = self.set_window_level(state.window_level);
        let _ = self.set_window_width(state.window_width);
        let _ = self.window_level.set_bias(state.bias);
        let _ = self.set_slice_mm(state.slice_mm);
        let _ = self.set_scale(state.scale);
        let _ = self.set_translate_in_screen_coord(state.translate_in_screen_coord);
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
