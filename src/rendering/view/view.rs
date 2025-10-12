//! # Medical Imaging View System
//! 
//! This module provides the core view system for medical imaging visualization in the Kepler WGPU framework.
//! It supports Multi-Planar Reconstruction (MPR) views with different anatomical orientations and provides
//! state management capabilities for seamless view transitions.
//! 
//! ## Key Components
//! 
//! - **View Traits**: Core interfaces for renderable views with position and dimension management
//! - **StatefulView**: Enhanced views that can save and restore their configuration state
//! - **ViewFactory**: Factory pattern for consistent view creation
//! - **MPRView**: Specialized interface for medical imaging views with window/level controls
//! - **GenericMPRView**: Concrete implementation supporting all anatomical orientations
//! - **ViewState**: State snapshot structure for preserving view configurations
//! 
//! ## Medical Imaging Context
//! 
//! The view system is designed specifically for medical imaging applications:
//! - Supports standard anatomical orientations (Transverse, Coronal, Sagittal, Oblique)
//! - Provides window/level controls for CT image display
//! - Handles slice navigation in millimeter units
//! - Maintains accurate spatial relationships and measurements
//! 
//! ## Usage Example
//! 
//! Creating an MPR view for medical imaging:
//! - Use `GenericMPRView::new()` with appropriate parameters
//! - Set orientation (Transverse, Coronal, Sagittal, or Oblique)
//! - Configure scale, translation, position, and dimensions
//! - Use `StatefulView` trait methods to save/restore view state
//! - Use `MPRView` trait methods to control window/level and slice position

#![allow(dead_code)]

use std::any::Any;
use std::sync::Arc;
use super::Renderable;
use crate::core::coord::{array_to_slice, Base};
use crate::core::geometry::GeometryBuilder;
use crate::rendering::content::render_content::RenderContent;
use crate::rendering::view::RenderContext;
use crate::CTVolume;

/// State snapshot for preserving view configuration during mode switches.
/// 
/// This structure captures all essential MPR view parameters that need to be restored
/// when transitioning between different view modes (e.g., mesh to MPR). It ensures
/// that users don't lose their current viewing configuration when switching between
/// different visualization modes.
/// 
/// ## Medical Imaging Context
/// 
/// In medical imaging workflows, users often need to switch between different
/// visualization modes while maintaining their current viewing parameters:
/// - Window/level settings for optimal tissue contrast
/// - Current slice position for anatomical reference
/// - Zoom and pan settings for detailed examination
/// - View positioning and dimensions
#[derive(Debug, Clone)]
pub struct ViewState {
    /// Window level (center) for CT image display - controls brightness
    pub window_level: f32,
    /// Window width for CT image display - controls contrast
    pub window_width: f32,
    /// Current slice position in millimeters along the view normal
    pub slice_mm: f32,
    /// Current zoom scale factor (1.0 = original size)
    pub scale: f32,
    /// Translation in view/model coordinate space [x, y, z]
    pub translate: [f32; 3],
    /// Translation in screen coordinate space [x, y, z] - used for panning
    pub translate_in_screen_coord: [f32; 3],
    /// View position on screen (top-left corner) in pixels
    pub position: (i32, i32),
    /// View dimensions (width, height) in pixels
    pub dimensions: (u32, u32),
}

impl ViewState {
    /// Create a new ViewState with default values for medical imaging.
    /// 
    /// Uses standard CT window/level settings and neutral positioning that work
    /// well for most medical imaging scenarios. The default window/level values
    /// are optimized for soft tissue visualization.
    pub fn new() -> Self {
        Self {
            window_level: 40.0,    // Standard CT soft tissue window level (HU)
            window_width: 400.0,   // Standard CT soft tissue window width (HU)
            slice_mm: 0.0,         // Start at center slice
            scale: 1.0,            // No zoom initially
            translate: [0.0, 0.0, 0.0],  // No model-space translation
            translate_in_screen_coord: [0.0, 0.0, 0.0],  // No screen-space panning
            position: (0, 0),      // Top-left corner
            dimensions: (512, 512), // Standard medical imaging size
        }
    }

    /// Validate that the view state contains reasonable values.
    /// 
    /// Ensures window width is positive, scale is within reasonable bounds, and dimensions are valid.
    /// This prevents invalid states from being restored and causing rendering issues.
    /// 
    /// ## Validation Rules
    /// 
    /// - Window width must be positive (required for proper CT display)
    /// - Scale must be positive and less than 100x (prevents extreme zoom levels)
    /// - Dimensions must be non-zero (required for valid viewport)
    pub fn is_valid(&self) -> bool {
        self.window_width > 0.0 
            && self.scale > 0.0 
            && self.scale < 100.0  // Reasonable scale limit to prevent extreme zoom
            && self.dimensions.0 > 0 
            && self.dimensions.1 > 0
    }
}

impl Default for ViewState {
    fn default() -> Self {
        Self::new()
    }
}

/// Core trait for all renderable views in the medical imaging system.
/// 
/// This trait extends the `Renderable` trait with view-specific functionality
/// including position management, resizing, and type introspection. All views
/// in the system must implement this trait to be managed by the layout system.
/// 
/// ## Design Philosophy
/// 
/// The trait is designed to be minimal but complete, providing only the essential
/// operations needed for view management while allowing concrete implementations
/// to add specialized functionality.
pub trait View: Renderable + Any {
    /// Get the current position of the view on screen (top-left corner in pixels)
    fn position(&self) -> (i32, i32);
    
    /// Get the current dimensions of the view (width, height in pixels)
    fn dimensions(&self) -> (u32, u32);
    
    /// Move the view to a new position on screen
    fn move_to(&mut self, pos: (i32, i32));
    
    /// Resize the view to new dimensions
    fn resize(&mut self, dim: (u32, u32));
    
    /// Get a reference to this view as Any for type introspection
    fn as_any(&self) -> &dyn Any;
    
    /// Get a mutable reference to this view as Any for type introspection
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Attempt to cast this view to an MPRView for medical imaging operations.
    /// Returns None if this view doesn't support MPR functionality.
    fn as_mpr(&mut self) -> Option<&mut dyn MPRView> {
        None
    }
}

/// Enhanced View trait with state management capabilities.
/// 
/// Allows views to save their current state and restore from a saved state,
/// enabling seamless transitions between different view modes. This is particularly
/// important in medical imaging where users need to maintain their viewing
/// configuration when switching between different visualization modes.
/// 
/// ## Use Cases
/// 
/// - Switching between 2D MPR and 3D mesh visualization
/// - Temporarily switching to a different view orientation
/// - Saving user preferences for later sessions
/// - Implementing undo/redo functionality for view changes
pub trait StatefulView: View {
    /// Save the current view state for later restoration.
    /// 
    /// Returns None if the view doesn't support state saving or if the current
    /// state is invalid. Implementations should validate the state before saving
    /// to ensure it can be successfully restored later.
    fn save_state(&self) -> Option<ViewState>;
    
    /// Restore view state from a previously saved snapshot.
    /// 
    /// Returns true if restoration was successful, false if the state was invalid
    /// or incompatible with the current view. Implementations should validate
    /// the incoming state and handle any necessary coordinate transformations.
    fn restore_state(&mut self, state: &ViewState) -> bool;
    
    /// Get a string identifier for the view type.
    /// 
    /// Used for type checking and debugging during view transitions. This helps
    /// ensure that states are only restored to compatible view types.
    fn view_type(&self) -> &'static str;
}

/// Factory trait for creating different types of views.
/// 
/// Centralizes view creation logic and provides a consistent interface for
/// creating views with proper initialization parameters. This pattern ensures
/// that all views are created with the correct dependencies and configuration.
/// 
/// ## Benefits
/// 
/// - Consistent view initialization across the application
/// - Centralized dependency injection for view creation
/// - Easy testing through mock factory implementations
/// - Type-safe view creation with proper error handling
pub trait ViewFactory {
    /// Create a new mesh view with specified position and dimensions.
    /// 
    /// Returns a boxed View trait object ready for rendering 3D mesh data.
    /// The view will be configured for 3D visualization with appropriate
    /// camera settings and rendering pipeline.
    fn create_mesh_view(
        &self, 
        manager: &mut crate::rendering::core::pipeline::PipelineManager, 
        pos: (i32, i32), 
        size: (u32, u32)
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>>;
    
    /// Create a new MPR view with volume data and orientation.
    /// 
    /// Returns a boxed View trait object configured for medical imaging display.
    /// The view will be set up with appropriate shaders, uniforms, and geometry
    /// for the specified anatomical orientation.
    /// 
    /// ## Parameters
    /// 
    /// - `manager`: Pipeline manager for GPU resource management
    /// - `vol`: CT volume data containing the medical imaging dataset
    /// - `orientation`: Anatomical orientation (Transverse, Coronal, Sagittal, Oblique)
    /// - `pos`: Initial position on screen
    /// - `size`: Initial dimensions of the view
    fn create_mpr_view(
        &self, 
        manager: &mut crate::rendering::core::pipeline::PipelineManager, 
        vol: &CTVolume, 
        orientation: Orientation, 
        pos: (i32, i32), 
        size: (u32, u32)
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>>;
}

/// Specialized trait for Multi-Planar Reconstruction (MPR) views in medical imaging.
/// 
/// Provides medical imaging-specific functionality including window/level controls,
/// slice navigation, and spatial transformations. This trait extends the base View
/// trait with operations specific to medical image visualization.
/// 
/// ## Medical Imaging Concepts
/// 
/// - **Window/Level**: Controls image brightness and contrast for optimal tissue visualization
/// - **Slice Navigation**: Moves through the volume in millimeter units along the view normal
/// - **Scale/Pan**: Provides zoom and pan functionality for detailed examination
/// - **Coordinate Systems**: Handles both screen-space and medical-space coordinates
pub trait MPRView: View {
    // === Setters for view parameters ===
    
    /// Set the window level (brightness control) for CT image display
    fn set_window_level(&mut self, window_level: f32);
    
    /// Set the window width (contrast control) for CT image display
    fn set_window_width(&mut self, window_width: f32);
    
    /// Set the current slice position in millimeters along the view normal
    fn set_slice_mm(&mut self, z: f32);
    
    /// Set the zoom scale factor (1.0 = original size)
    fn set_scale(&mut self, scale: f32);
    
    /// Set translation in view/model coordinate space
    fn set_translate(&mut self, translate: [f32; 3]);
    
    /// Set translation in screen coordinate space (for panning)
    fn set_translate_in_screen_coord(&mut self, translate: [f32; 3]);
    
    /// Pan the view in screen space by the specified amounts
    fn set_pan(&mut self, x: f32, y: f32);
    
    /// Pan the view by the specified amounts in millimeters
    fn set_pan_mm(&mut self, x_mm: f32, y_mm: f32);
    
    // === Getters for view parameters ===
    
    /// Returns current window level used by the fragment shader
    fn get_window_level(&self) -> f32;
    
    /// Returns current window width used by the fragment shader
    fn get_window_width(&self) -> f32;
    
    /// Returns current slice position in millimeters along view normal
    fn get_slice_mm(&self) -> f32;
    
    /// Returns current scale factor applied in screen space
    fn get_scale(&self) -> f32;
    
    /// Returns current pan/translation in screen coordinates [x, y, z]
    fn get_translate_in_screen_coord(&self) -> [f32; 3];
    
    /// Returns current translation in view/model coordinates [x, y, z]
    fn get_translate(&self) -> [f32; 3];
}

/// Anatomical orientation for MPR views.
/// 
/// Defines the standard anatomical orientations used in medical imaging.
/// Each orientation provides a different cross-sectional view of the patient's anatomy.
/// 
/// ## Medical Context
/// 
/// - **Transverse (Axial)**: Horizontal slices, looking from feet toward head
/// - **Coronal**: Vertical slices, looking from front to back
/// - **Sagittal**: Vertical slices, looking from side to side
/// - **Oblique**: Custom orientation, not aligned with standard anatomical planes
#[derive(Debug, Clone, Copy)]
pub enum Orientation {
    /// Custom orientation not aligned with standard anatomical planes
    Oblique,
    /// Vertical slices from side to side (left-right separation)
    Sagittal,
    /// Vertical slices from front to back (anterior-posterior separation)
    Coronal,
    /// Horizontal slices from top to bottom (superior-inferior separation)
    Transverse,
}

/// Standard anatomical orientations in order of common usage
pub const ALL_ORIENTATIONS: [Orientation; 4] = [
    Orientation::Transverse,  // Most commonly used in CT
    Orientation::Coronal,
    Orientation::Sagittal,
    Orientation::Oblique,     // Least commonly used
];

impl Orientation {
    /// Build the coordinate system base for this orientation.
    /// 
    /// Creates the appropriate coordinate transformation matrix for the given
    /// anatomical orientation, taking into account the volume's spatial properties
    /// and DICOM coordinate system conventions.
    fn build_base(&self, vol: &CTVolume) -> Base<f32> {
        match self {
            Orientation::Oblique => GeometryBuilder::build_oblique_base(vol),
            Orientation::Sagittal => GeometryBuilder::build_sagittal_base(vol),
            Orientation::Coronal => GeometryBuilder::build_coronal_base(vol),
            Orientation::Transverse => GeometryBuilder::build_transverse_base(vol),
        }
    }

    /// Get the default slice navigation speed for this orientation.
    /// 
    /// Different orientations may have different optimal navigation speeds
    /// based on typical slice thickness and user interaction patterns.
    /// Transverse views typically have thicker slices and can use faster navigation.
    fn default_slice_speed(&self) -> f32 {
        match self {
            Orientation::Transverse => 0.006,  // Faster for thicker axial slices
            _ => 0.0005,  // Slower for thinner coronal/sagittal slices
        }
    }
}

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
pub struct GenericMPRView {
    /// Rendering context containing GPU resources and uniforms
    ctx: RenderContext,
    /// Reference to the 3D volume texture data
    texture: Arc<RenderContent>,
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

impl GenericMPRView {
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
        manager: &mut crate::rendering::core::pipeline::PipelineManager,
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
        let s_speed = orientation.default_slice_speed();

        // Build coordinate system bases for this orientation
        let base_screen = orientation.build_base(vol);
        let base_uv = GeometryBuilder::build_uv_base(vol);

        // Initialize view state
        let pan = [0.0, 0.0, 0.0];  // No initial panning
        let slice = 0.0;  // Start at center slice

        // Create screen-space transformation matrix
        let mut base_screen_cloned = base_screen.clone();
        // Apply transformations in reverse order (matrix multiplication)
        base_screen_cloned.translate([-pan[0], -pan[1], -pan[2]]); 
        base_screen_cloned.translate([0.5, 0.5, 0.0]); // Move back to origin
        base_screen_cloned.scale([scale, scale, 1.0]);  // Apply zoom
        base_screen_cloned.translate([-0.5, -0.5, 0.0]); // Center the transformation

        // Create final transformation matrix from screen to UV coordinates
        let transform_matrix = base_screen_cloned.to_base(&base_uv).transpose();

        // Initialize rendering context with GPU resources
        let view = RenderContext::new(manager, device, &texture, transform_matrix);

        log::info!("Created GenericMPRView with orientation: {:?}, scale: {:?}, translate: {:?}, pos: {:?}, dim: {:?}",
            orientation, scale, translate, pos, dim);
            
        Self {
            ctx: view,
            texture,
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
    fn update_transform_matrix(&mut self) {
        let mut base_screen_cloned = self.base_screen.clone();
        
        // Apply transformations in reverse order due to matrix multiplication
        base_screen_cloned.translate([-self.pan[0], -self.pan[1], -self.pan[2]]); 
        base_screen_cloned.translate([0.5, 0.5, 0.0]); // Move back to origin
        base_screen_cloned.scale([self.scale, self.scale, 1.0]); // Apply current zoom
        base_screen_cloned.translate([-0.5, -0.5, 0.0]); // Center the transformation

        // Create final transformation matrix and update GPU uniforms
        let transform_matrix = base_screen_cloned
            .to_base(&self.base_uv)
            .transpose();
        self.ctx.uniforms.frag.mat = *array_to_slice(&transform_matrix.data);
    }
}

impl Drop for GenericMPRView {
    /// Clean up GPU resources when the view is dropped.
    /// 
    /// Logs the destruction for debugging purposes. The actual GPU resource
    /// cleanup is handled by the RenderContext's Drop implementation.
    fn drop(&mut self) {
        log::debug!("Dropping GenericMPRView - GPU resources will be cleaned up");
    }
}

impl Renderable for GenericMPRView {
    /// Update GPU uniforms with current view state.
    /// 
    /// Called every frame to ensure GPU shaders have the latest view parameters.
    /// Updates both vertex and fragment shader uniforms with current transformation
    /// matrix, slice position, and other rendering parameters.
    fn update(&mut self, queue: &wgpu::Queue) {
        // Update slice position for volume sampling
        self.ctx.uniforms.frag.slice = self.slice;
        
        // Recalculate transformation matrix if view parameters changed
        self.update_transform_matrix();

        // Upload vertex shader uniforms (transformation matrices)
        queue.write_buffer(
            &self.ctx.uniform_vert_buffer,
            0,
            bytemuck::cast_slice(&[self.ctx.uniforms.vert]),
        );
        
        // Upload fragment shader uniforms (slice, window/level, etc.)
        queue.write_buffer(
            &self.ctx.uniform_frag_buffer,
            0,
            bytemuck::cast_slice(&[self.ctx.uniforms.frag]),
        );
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
        // Set the rendering pipeline for MPR visualization
        render_pass.set_pipeline(&self.ctx.render_pipeline);

        // Configure viewport to this view's screen region
        let (x, y) = (self.pos.0 as f32, self.pos.1 as f32);
        let (width, height) = (self.dim.0, self.dim.1);
        render_pass.set_viewport(x, y, width as f32, height as f32, 0.0, 1.0);
        
        // Bind GPU resources
        render_pass.set_bind_group(0, &self.ctx.texture_bind_group, &[]);      // Volume texture
        render_pass.set_bind_group(1, &self.ctx.uniform_vert_bind_group, &[]); // Vertex uniforms
        render_pass.set_bind_group(2, &self.ctx.uniform_frag_bind_group, &[]); // Fragment uniforms
        
        // Bind geometry buffers
        render_pass.set_vertex_buffer(0, self.ctx.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.ctx.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        
        // Draw the screen-aligned quad
        render_pass.draw_indexed(0..self.ctx.num_indices, 0, 0..1);
        Ok(())
    }
}

impl View for GenericMPRView {
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
    
    /// Cast this view to an MPRView since GenericMPRView implements MPRView.
    fn as_mpr(&mut self) -> Option<&mut dyn MPRView> {
        Some(self)
    }
}

impl MPRView for GenericMPRView {
    /// Set the window level (brightness) for CT image display.
    fn set_window_level(&mut self, window_level: f32) {
        self.ctx.uniforms.frag.window_level = window_level;
    }
    
    /// Set the window width (contrast) for CT image display.
    fn set_window_width(&mut self, window_width: f32) {
        self.ctx.uniforms.frag.window_width = window_width;
    }
    
    /// Set the current slice position in millimeters.
    /// 
    /// Converts millimeter units to internal coordinate system units
    /// using the volume's scale factors for accurate positioning.
    fn set_slice_mm(&mut self, z: f32) {
        let [_, _, scale_z] = self.base_screen.get_scale_factors();
        self.pan[2] = z / scale_z;
    }
    
    /// Set the zoom scale factor.
    fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }
    
    /// Set translation in view/model coordinate space.
    fn set_translate(&mut self, translate: [f32; 3]) {
        self.translate = translate;
    }
    
    /// Set translation in screen coordinate space (for panning).
    fn set_translate_in_screen_coord(&mut self, translate: [f32; 3]) {
        self.pan = translate;
    }
    
    /// Pan the view in screen space.
    fn set_pan(&mut self, x: f32, y: f32) {
        self.pan[0] = x;
        self.pan[1] = y;
    }
    
    /// Pan the view by millimeter amounts.
    /// 
    /// Converts millimeter units to screen coordinate units using
    /// the volume's scale factors for accurate positioning.
    fn set_pan_mm(&mut self, x_mm: f32, y_mm: f32) {
        let [scale_x, scale_y, _] = self.base_screen.get_scale_factors();
        self.pan[0] = x_mm / scale_x;
        self.pan[1] = y_mm / scale_y;
    }
    
    /// Retrieve current window level from fragment uniforms for state snapshotting.
    fn get_window_level(&self) -> f32 { 
        self.ctx.uniforms.frag.window_level 
    }
    
    /// Retrieve current window width from fragment uniforms for state snapshotting.
    fn get_window_width(&self) -> f32 { 
        self.ctx.uniforms.frag.window_width 
    }
    
    /// Convert internal pan.z (in screen units) back to millimeters using base scale factors.
    fn get_slice_mm(&self) -> f32 {
        let [_, _, scale_z] = self.base_screen.get_scale_factors();
        self.pan[2] * scale_z
    }
    
    /// Return current screen-space scale factor.
    fn get_scale(&self) -> f32 { 
        self.scale 
    }
    
    /// Return current screen-space translation vector.
    fn get_translate_in_screen_coord(&self) -> [f32; 3] { 
        self.pan 
    }
    
    /// Return current view/model-space translation vector.
    fn get_translate(&self) -> [f32; 3] { 
        self.translate 
    }
}

impl StatefulView for GenericMPRView {
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
            log::debug!("Saved MPR view state: window_level={}, window_width={}, scale={}, slice_mm={}", 
                state.window_level, state.window_width, state.scale, state.slice_mm);
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
        
        // Update transform matrix to reflect new state
        self.update_transform_matrix();
        
        log::debug!("Restored MPR view state: window_level={}, window_width={}, scale={}, slice_mm={}", 
            state.window_level, state.window_width, state.scale, state.slice_mm);
        true
    }
    
    /// Return the view type identifier for this MPR view.
    fn view_type(&self) -> &'static str {
        "GenericMPRView"
    }
}

// Optional: keep type aliases for old names
pub type ObliqueView = GenericMPRView;
pub type SagittalView = GenericMPRView;
pub type TransverseView = GenericMPRView;
pub type CoronalView = GenericMPRView;