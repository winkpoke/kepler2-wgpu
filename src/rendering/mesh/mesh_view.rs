#![allow(dead_code)]

use cgmath::SquareMatrix;

use super::{mesh::Mesh, material::Material, camera::Camera, lighting::Lighting, performance::{QualityController, QualityLevel, PerformanceStats}, basic_mesh_context::BasicMeshContext};
use crate::{core::coord::Matrix4x4, rendering::view::{Renderable, View}, core::timing::{Instant, DurationExt}};
use std::sync::Arc;

/// Function-level comment: Error types specific to mesh rendering operations
#[derive(Debug)]
pub enum MeshRenderError {
    /// Context not attached to the view
    ContextNotAttached,
    /// Buffer validation failed
    BufferValidationFailed(String),
    /// Pipeline creation or binding failed
    PipelineError(String),
    /// Viewport configuration error
    ViewportError(String),
    /// Resource allocation failed
    ResourceError(String),
}

impl std::fmt::Display for MeshRenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeshRenderError::ContextNotAttached => write!(f, "Mesh render context not attached"),
            MeshRenderError::BufferValidationFailed(msg) => write!(f, "Buffer validation failed: {}", msg),
            MeshRenderError::PipelineError(msg) => write!(f, "Pipeline error: {}", msg),
            MeshRenderError::ViewportError(msg) => write!(f, "Viewport error: {}", msg),
            MeshRenderError::ResourceError(msg) => write!(f, "Resource error: {}", msg),
        }
    }
}

impl std::error::Error for MeshRenderError {}

/// Function-level comment: Rendering statistics and performance metrics for mesh rendering
#[derive(Debug, Default)]
pub struct RenderStats {
    pub frame_count: u64,
    pub error_count: u64,
    pub last_render_time_ms: f32,
    pub average_render_time_ms: f32,
    pub buffer_validation_failures: u64,
    pub pipeline_errors: u64,
}

/// Function-level comment: Fallback rendering modes when primary rendering fails
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FallbackMode {
    /// Normal rendering with all features
    Normal,
    /// Simplified rendering without complex shaders
    Simplified,
    /// Wireframe rendering only
    Wireframe,
    /// Disabled - skip rendering entirely
    Disabled,
}

impl Default for FallbackMode {
    fn default() -> Self {
        FallbackMode::Normal
    }
}

pub struct MeshView {
    pub mesh: Option<Mesh>,
    pub material: Option<Material>,
    pub camera: Option<Camera>,
    pub lighting: Option<Lighting>,
    ctx: Option<std::sync::Arc<BasicMeshContext>>,
    pos: (i32, i32),
    dim: (u32, u32),
    /// Performance and error tracking
    stats: RenderStats,
    /// Current fallback mode
    fallback_mode: FallbackMode,
    /// Error recovery state
    consecutive_errors: u32,
    /// Last successful render timestamp
    last_success_time: Instant,
    /// Performance monitoring and automatic quality adjustment
    quality_controller: QualityController,
    /// Y-axis rotation state
    rotation_enabled: bool,
    /// Current rotation angle in radians
    rotation_angle: f32,
    /// Rotation speed in radians per second (default: π/2 = 90 degrees/second)
    rotation_speed: f32,
    /// Last frame time for rotation calculation
    last_frame_time: Instant,
}

impl Default for MeshView {
    fn default() -> Self {
        use std::f32::consts::PI;
        Self {
            mesh: None,
            material: None,
            camera: None,
            lighting: None,
            ctx: None,
            pos: (0, 0),
            dim: (0, 0),
            stats: RenderStats::default(),
            fallback_mode: FallbackMode::default(),
            consecutive_errors: 0,
            last_success_time: Instant::now(),
            quality_controller: QualityController::default(),
            rotation_enabled: true,
            rotation_angle: 0.0,
            rotation_speed: PI / 2.0, // 90 degrees per second - reasonable default speed
            last_frame_time: Instant::now(),
        }
    }
}

impl MeshView {
    pub fn new() -> Self { 
        Self::default()
    }
    
    /// Function-level comment: Attaches a basic mesh render context for GPU operations
    pub fn attach_context(&mut self, ctx: std::sync::Arc<BasicMeshContext>) {
        self.ctx = Some(ctx);
        log::debug!("MeshView::attach_context - Basic context attached successfully");
        // Reset error state when new context is attached
        self.reset_error_state();
    }
    
    /// Function-level comment: Get current rendering statistics for performance monitoring.
    pub fn get_stats(&self) -> &RenderStats {
        &self.stats
    }
    
    /// Function-level comment: Get current fallback mode for debugging.
    pub fn get_fallback_mode(&self) -> FallbackMode {
        self.fallback_mode
    }
    
    /// Function-level comment: Force a specific fallback mode for testing or recovery.
    pub fn set_fallback_mode(&mut self, mode: FallbackMode) {
        self.fallback_mode = mode;
        log::info!("Mesh rendering fallback mode set to: {:?}", mode);
    }
    
    /// Function-level comment: Reset error state and attempt to return to normal rendering.
    pub fn reset_error_state(&mut self) {
        self.consecutive_errors = 0;
        self.fallback_mode = FallbackMode::Normal;
        log::info!("Mesh rendering error state reset");
    }
    
    /// Function-level comment: Get current quality level from the performance controller.
    pub fn get_quality_level(&self) -> QualityLevel {
        self.quality_controller.get_quality_level()
    }
    
    /// Function-level comment: Manually set quality level (disables automatic adjustment temporarily).
    pub fn set_quality_level(&mut self, quality: QualityLevel) {
        self.quality_controller.set_quality_level(quality);
    }
    
    /// Function-level comment: Get comprehensive performance statistics.
    pub fn get_performance_stats(&self) -> PerformanceStats {
        self.quality_controller.get_performance_stats()
    }
    
    /// Function-level comment: Get buffer memory usage statistics for monitoring and optimization.
    pub fn get_memory_stats(&self) -> (u64, u64, f32, f32) {
        if let Some(ctx) = &self.ctx {
            ctx.get_memory_stats()
        } else {
            (0, 0, 0.0, 0.0)
        }
    }

    /// Function-level comment: Enable or disable Y-axis rotation animation.
    /// When enabled, the mesh will continuously rotate around the Y-axis at the configured speed.
    pub fn set_rotation_enabled(&mut self, enabled: bool) {
        if enabled != self.rotation_enabled {
            self.rotation_enabled = enabled;
            if enabled {
                // Reset timing when enabling rotation to prevent jumps
                self.last_frame_time = Instant::now();
                log::info!("Mesh Y-axis rotation enabled at {:.1}°/s", self.rotation_speed.to_degrees());
            } else {
                log::info!("Mesh Y-axis rotation disabled");
            }
        }
    }

    /// Function-level comment: Check if Y-axis rotation is currently enabled.
    pub fn is_rotation_enabled(&self) -> bool {
        self.rotation_enabled
    }

    /// Function-level comment: Set the rotation speed in radians per second.
    /// Positive values rotate counter-clockwise when viewed from above (standard Y-up convention).
    /// Common values: π/4 (45°/s), π/2 (90°/s), π (180°/s), 2π (360°/s)
    pub fn set_rotation_speed(&mut self, speed_rad_per_sec: f32) {
        self.rotation_speed = speed_rad_per_sec;
        log::info!("Mesh rotation speed set to {:.3} rad/s ({:.1}°/s)", 
                   speed_rad_per_sec, speed_rad_per_sec.to_degrees());
    }

    /// Function-level comment: Get the current rotation speed in radians per second.
    pub fn get_rotation_speed(&self) -> f32 {
        self.rotation_speed
    }

    /// Function-level comment: Reset the rotation angle to zero.
    /// Useful for returning to a known orientation or synchronizing multiple objects.
    pub fn reset_rotation(&mut self) {
        self.rotation_angle = 0.0;
        self.last_frame_time = Instant::now();
        log::debug!("Mesh rotation angle reset to 0°");
    }

    /// Function-level comment: Get the current rotation angle in radians.
    pub fn get_rotation_angle(&self) -> f32 {
        self.rotation_angle
    }

    /// Function-level comment: Set rotation speed using degrees per second for convenience.
    /// This is a helper method that converts degrees to radians internally.
    pub fn set_rotation_speed_degrees(&mut self, degrees_per_sec: f32) {
        self.set_rotation_speed(degrees_per_sec.to_radians());
    }

    /// Function-level comment: Create a default camera positioned to view a unit cube
    fn create_default_camera(&self) -> Camera {
        // Use perspective projection for initial testing to match working implementation
        let mut camera = Camera::new_perspective();
        camera.eye = [0.0, 0.0, 3.0]; // Position camera closer to the cube
        camera.center = [0.0, 0.0, 0.0]; // Look at origin
        camera.up = [0.0, 1.0, 0.0]; // Y-up
        camera.fov_y_radians = std::f32::consts::PI / 4.0; // 45 degrees FOV
        camera.near = 0.1;
        camera.far = 100.0;
        
        camera
    }

    /// Function-level comment: Create default lighting setup for basic mesh illumination.
    /// Returns lighting with reasonable defaults for 3D mesh viewing.
    fn create_default_lighting(&self) -> Lighting {
        Lighting {
            direction: [0.0, -1.0, -1.0],  // Light coming from above and front
            intensity: 1.0,                 // Full intensity white light
        }
    }

    /// Function-level comment: Update GPU uniforms for basic mesh rendering with combined MVP matrix
    /// Includes Y-axis rotation if enabled, using frame-rate independent timing
    pub fn update_uniforms(&mut self, queue: &wgpu::Queue) {
        // Update rotation angle if rotation is enabled
        if self.rotation_enabled {
            let current_time = Instant::now();
            let delta_time = current_time.duration_since(self.last_frame_time).as_secs_f32();
            self.rotation_angle += self.rotation_speed * delta_time;
            
            // Keep angle in [0, 2π] range to prevent floating point precision issues
            use std::f32::consts::TAU; // TAU = 2π
            if self.rotation_angle >= TAU {
                self.rotation_angle -= TAU;
            }
            
            self.last_frame_time = current_time;
            
            #[cfg(feature = "trace-logging")]
            log::trace!("[MESH_ROTATION] Angle: {:.3} rad ({:.1}°), Speed: {:.3} rad/s, Delta: {:.3}ms", 
                       self.rotation_angle, self.rotation_angle.to_degrees(), self.rotation_speed, delta_time * 1000.0);
        }
        
        if let Some(ctx) = &self.ctx {
            let aspect_ratio = if self.dim.1 > 0 {
                self.dim.0 as f32 / self.dim.1 as f32
            } else {
                1.0
            };
            
            log::debug!("[BASIC_MESH_UNIFORMS] Viewport dimensions: {}x{}, aspect_ratio: {:.3}", 
                       self.dim.0, self.dim.1, aspect_ratio);
            
            // Simplified MVP matrix calculation for guaranteed visibility
            use std::f32::consts::PI;
            
            // Model matrix - scaling with optional Y-axis rotation
            let scale = 1.0; // Make cube much smaller
            
            let model_matrix = if self.rotation_enabled {
                // Create Y-axis rotation matrix
                let cos_y = self.rotation_angle.cos();
                let sin_y = self.rotation_angle.sin();
                
                // Combined scale and Y-axis rotation matrix
                // Rotation around Y-axis: [cos θ, 0, sin θ, 0; 0, 1, 0, 0; -sin θ, 0, cos θ, 0; 0, 0, 0, 1]
                Matrix4x4::from_array([
                    cos_y,  0.0,    sin_y,  0.0,
                    0.0,    1.0,    0.0,    0.0,
                    -sin_y, 0.0,    cos_y,  0.0,
                    0.0,    0.0,    0.0,    1.0,
                ])
            } else {
                // Identity matrix with uniform scaling only
                Matrix4x4::from_array([
                    scale, 0.0, 0.0, 0.0,
                    0.0, scale, 0.0, 0.0,
                    0.0, 0.0, scale, 0.0,
                    0.0, 0.0, 0.0, 1.0,
                ])
            };
            
            // View matrix - camera positioned for optimal viewing of smaller cube
            let view_matrix = Matrix4x4::from_array([
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, -5.0, // Closer camera for smaller cube
                0.0, 0.0, 0.0, 1.0,
            ]);
            
            // Orthogonal projection matrix for medical imaging accuracy
            let left = -2.0;
            let right = 2.0;
            let bottom = -2.0 / aspect_ratio;
            let top = 2.0 / aspect_ratio;
            let near = 0.5;
            let far = 9.;
            
            let proj_matrix = Matrix4x4::from_array([
                2.0 / (right - left), 0.0, 0.0, -(right + left) / (right - left),
                0.0, 2.0 / (top - bottom), 0.0, -(top + bottom) / (top - bottom),
                0.0, 0.0, -2.0 / (far - near), -(far + near) / (far - near),
                0.0, 0.0, 0.0, 1.0,
            ]);

            // let proj_matrix = Matrix4x4::from_array([
            //     1.0, 0.0, 0.0, 0.0,
            //     0.0, aspect_ratio, 0.0, 0.0,  // Adjust for aspect ratio
            //     0.0, 0.0, -1.0, 0.0,          // Simple depth mapping
            //     0.0, 0.0, 0.0, 1.0,
            // ]);
            // let proj_matrix = Matrix4x4::from_array([
            //     2.0 / 4., 0.0, 0.0, 0.0,
            //     0.0, 2.0 / 4., 0.0, 0.0,
            //     0.0, 0.0, -2.0 / (far - near), -(far + near) / (far - near),
            //     0.0, 0.0, 0.0, 1.0,
            // ]);
            
            
            // Calculate MVP: projection * view * model
            let view_model = view_matrix.multiply(&model_matrix);
            let mvp_matrix = proj_matrix.multiply(&view_model);
            // let mvp_matrix = proj_matrix.multiply(&view_matrix);
            
            if self.rotation_enabled {
                log::debug!("[BASIC_MESH_MATRICES] Model matrix (scale {} with Y-rotation {:.3} rad): {:?}", 
                           scale, self.rotation_angle, model_matrix.data);
            } else {
                log::debug!("[BASIC_MESH_MATRICES] Model matrix (scale {} no rotation): {:?}", 
                           scale, model_matrix.data);
            }
            log::debug!("[BASIC_MESH_MATRICES] View matrix (camera at -3): {:?}", view_matrix.data);
            log::debug!("[BASIC_MESH_MATRICES] Projection matrix (orthogonal): {:?}", proj_matrix.data);
            log::debug!("[BASIC_MESH_MATRICES] Combined MVP matrix: {:?}", mvp_matrix.data);
            
            // Update uniforms in BasicMeshContext with combined MVP matrix
            // Note: The shader expects column-major matrices, so we transpose the MVP matrix
            let mvp_matrix_transposed = mvp_matrix.transpose();
            log::trace!("[BASIC_MESH_MATRICES] Transposed MVP matrix for shader: {:?}", mvp_matrix_transposed.data);
            
            // ctx.update_uniforms(queue, &mvp_matrix_transposed.data);

            let scale = cgmath::Matrix4::from_scale(2.0);
            let rotation = cgmath::Matrix4::from_angle_y(cgmath::Rad(self.rotation_angle));
            let translate = cgmath::Matrix4::from_translation(cgmath::vec3(0.0, 0.0, -5.0));
            let aspect = self.dim.0 as f32 / self.dim.1 as f32;
            // let proj = cgmath::perspective(cgmath::Deg(45.0), aspect, 0.1, 100.0);
            let proj = cgmath::ortho(left, right, bottom, top, near, far);
            let view = cgmath::Matrix4::identity();
            let mvp = proj * view * translate * rotation* scale;
            ctx.update_uniforms(queue, &mvp.into());
        }
    }
    
    /// Function-level comment: Start frame timing for performance monitoring.
    pub fn start_frame_timing(&mut self) {
        self.quality_controller.start_frame();
    }
    
    /// Function-level comment: End frame timing and check for quality adjustments.
    pub fn end_frame_timing(&mut self) -> Option<QualityLevel> {
        self.quality_controller.end_frame()
    }
    
    /// Function-level comment: Check if the view is in a healthy state for rendering.
    pub fn is_healthy(&self) -> bool {
        self.consecutive_errors < 5 && 
        self.last_success_time.elapsed().as_secs_f64() < 30.0 &&
        !matches!(self.fallback_mode, FallbackMode::Disabled)
    }
    
    /// Function-level comment: Handle rendering errors and determine appropriate fallback strategy.
    fn handle_render_error(&mut self, error: MeshRenderError) {
        self.stats.error_count += 1;
        self.consecutive_errors += 1;
        
        log::warn!("Mesh render error (consecutive: {}): {}", self.consecutive_errors, error);
        
        // Update specific error counters
        match error {
            MeshRenderError::BufferValidationFailed(_) => {
                self.stats.buffer_validation_failures += 1;
            }
            MeshRenderError::PipelineError(_) => {
                self.stats.pipeline_errors += 1;
            }
            _ => {}
        }
        
        // Determine fallback strategy based on error count and type
        self.fallback_mode = match self.consecutive_errors {
            1..=2 => FallbackMode::Normal, // Retry normal rendering
            3..=4 => FallbackMode::Simplified, // Switch to simplified rendering
            5..=7 => FallbackMode::Wireframe, // Fall back to wireframe
            _ => FallbackMode::Disabled, // Disable rendering entirely
        };
        
        log::info!("Switched to fallback mode: {:?}", self.fallback_mode);
    }
    
    /// Function-level comment: Record successful render and update performance metrics.
    fn record_success(&mut self, render_time_ms: f32) {
        self.stats.frame_count += 1;
        self.stats.last_render_time_ms = render_time_ms;
        
        // Update rolling average
        let alpha = 0.1; // Smoothing factor
        self.stats.average_render_time_ms = 
            alpha * render_time_ms + (1.0 - alpha) * self.stats.average_render_time_ms;
        
        // Reset error state on successful render
        if self.consecutive_errors > 0 {
            self.consecutive_errors = 0;
            log::info!("Mesh rendering recovered after {} consecutive errors", self.consecutive_errors);
        }
        
        self.last_success_time = Instant::now();
        
        // Gradually return to normal mode if we've been in fallback
        if matches!(self.fallback_mode, FallbackMode::Simplified | FallbackMode::Wireframe) {
            if self.stats.frame_count % 60 == 0 { // Try to upgrade every 60 frames
                self.fallback_mode = match self.fallback_mode {
                    FallbackMode::Wireframe => FallbackMode::Simplified,
                    FallbackMode::Simplified => FallbackMode::Normal,
                    _ => self.fallback_mode,
                };
                log::info!("Upgraded to fallback mode: {:?}", self.fallback_mode);
            }
        }
    }
    
    /// Function-level comment: Attempt to render with comprehensive error handling and fallback.
    fn try_render(&mut self, render_pass: &mut wgpu::RenderPass) -> Result<(), MeshRenderError> {
        // Start frame timing for performance monitoring
        self.start_frame_timing();
        let start_time = Instant::now();
        
        // Check if rendering is disabled
        if matches!(self.fallback_mode, FallbackMode::Disabled) {
            log::trace!("BasicMeshView::try_render - Rendering disabled due to fallback mode");
            return Err(MeshRenderError::ResourceError("Rendering disabled due to repeated failures".to_string()));
        }
        
        // Ensure context is available
        let ctx = self.ctx.as_ref()
            .ok_or_else(|| {
                log::trace!("BasicMeshView::try_render - No context attached");
                MeshRenderError::ContextNotAttached
            })?;
        
        log::trace!("BasicMeshView::try_render - Context available, vertices: {}, indices: {}", ctx.num_vertices, ctx.num_indices);
        
        // Validate viewport dimensions
        if self.dim.0 == 0 || self.dim.1 == 0 {
            return Err(MeshRenderError::ViewportError("Invalid viewport dimensions".to_string()));
        }
        
        // Configure viewport
        let (x, y) = (self.pos.0 as f32, self.pos.1 as f32);
        let (width, height) = (self.dim.0 as f32, self.dim.1 as f32);
        render_pass.set_viewport(x, y, width, height, 0.0, 1.0);
        
        log::trace!("BasicMeshView::try_render - Viewport set to ({}, {}) {}x{}", x, y, width, height);
        
        // Use the simplified BasicMeshContext render method
        ctx.render(render_pass);
        
        // Record successful render
        let render_time_ms = start_time.elapsed().as_millis_f32();
        self.record_success(render_time_ms);
        log::trace!("BasicMeshView::try_render - Render completed successfully in {:.2}ms", render_time_ms);
        
        // End frame timing and check for quality adjustments
        if let Some(new_quality) = self.end_frame_timing() {
            log::info!("Quality automatically adjusted to {:?} based on performance", new_quality);
            // Quality adjustment is already applied by the controller
        }
        
        Ok(())
    }
}

impl Renderable for MeshView {
    fn update(&mut self, queue: &wgpu::Queue) {
        // Function-level comment: Update mesh view state and perform health checks
        
        // Update uniform buffers with current camera and lighting data
        self.update_uniforms(queue);
        
        // Perform periodic health checks
        if self.stats.frame_count % 300 == 0 { // Every 5 seconds at 60fps
            if !self.is_healthy() {
                log::warn!("MeshView health check failed: consecutive_errors={}, last_success={:?}s ago, mode={:?}", 
                    self.consecutive_errors, 
                    self.last_success_time.elapsed().as_secs_f64(),
                    self.fallback_mode
                );
            }
        }
        
        // Auto-recovery attempt if we've been disabled for too long
        if matches!(self.fallback_mode, FallbackMode::Disabled) && 
           self.last_success_time.elapsed().as_secs_f64() > 60.0 {
            log::info!("Attempting auto-recovery from disabled state");
            self.fallback_mode = FallbackMode::Wireframe;
            self.consecutive_errors = 5; // Start with reduced error count
        }
    }
    
    fn render(&mut self, render_pass: &mut wgpu::RenderPass) -> Result<(), wgpu::SurfaceError> {
        // Function-level comment: Render mesh with comprehensive error handling and fallback mechanisms
        
        // Attempt rendering with error handling
        match self.try_render(render_pass) {
            Ok(()) => {
                // Successful render - no action needed
                Ok(())
            }
            Err(error) => {
                // Handle the error and determine recovery strategy
                self.handle_render_error(error);
                
                // Always return Ok to prevent frame failure
                // The error handling will adjust fallback mode for next frame
                Ok(())
            }
        }
    }
}

impl Drop for MeshView {
    /// Function-level comment: Clean up MeshView resources and log the drop for debugging.
    fn drop(&mut self) {
        log::debug!("[MESH_VIEW] Dropping MeshView at position {:?} with size {:?}", self.pos, self.dim);
    }
}

impl View for MeshView {
    fn position(&self) -> (i32, i32) { self.pos }
    fn dimensions(&self) -> (u32, u32) { self.dim }
    fn move_to(&mut self, pos: (i32, i32)) { 
        log::debug!("[MESH_VIEW] Moving to position: {:?}", pos);
        self.pos = pos; 
    }
    fn resize(&mut self, dim: (u32, u32)) { 
        log::debug!("[MESH_VIEW] Resizing to dimensions: {:?}", dim);
        self.dim = dim; 
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}