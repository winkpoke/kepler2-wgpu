#![allow(dead_code)]

use super::{
    basic_mesh_context::BasicMeshContext,
    camera::Camera,
    mesh::{Lighting, Mesh},
    performance::{PerformanceStats, QualityController, QualityLevel},
};
use crate::{
    core::timing::Instant,
    rendering::view::{Renderable, View},
};
use glam::{Mat4, Quat, Vec2, Vec3};

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
            MeshRenderError::BufferValidationFailed(msg) => {
                write!(f, "Buffer validation failed: {}", msg)
            }
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
    // pub material: Option<Material>,
    pub camera: Option<Camera>,
    pub lighting: Option<Lighting>,
    ctx: Option<std::sync::Arc<BasicMeshContext>>,
    /// Context for the orientation cube (bottom-left gizmo)
    orientation_cube_ctx: Option<std::sync::Arc<BasicMeshContext>>,
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
    /// rotation state
    rotation_enabled: bool,
    /// Current rotation state as a quaternion (supports free 3D rotation)
    rotation_quat: Quat,
    /// Rotation speed in radians per second (default: π/2 = 90 degrees/second)
    rotation_speed: f32,
    /// Last frame time for rotation calculation
    last_frame_time: Instant,
    /// Uniform scale factor
    scale_factor: f32,
    /// Pan translation in world units (X, Y, Z)
    pan: [f32; 3],
    /// Mesh opacity (0.0 transparent .. 1.0 opaque)
    opacity: f32,
}

impl Default for MeshView {
    fn default() -> Self {
        use std::f32::consts::FRAC_PI_2;
        Self {
            mesh: None,
            // material: None,
            camera: None,
            lighting: None,
            ctx: None,
            orientation_cube_ctx: None,
            pos: (0, 0),
            dim: (0, 0),
            stats: RenderStats::default(),
            fallback_mode: FallbackMode::default(),
            consecutive_errors: 0,
            last_success_time: Instant::now(),
            quality_controller: QualityController::default(),
            rotation_enabled: true,
            // Angles are stored in radians; 90°=FRAC_PI_2, 180°=PI
            rotation_quat: Quat::IDENTITY,
            rotation_speed: FRAC_PI_2, // 90 degrees per second - reasonable default speed
            last_frame_time: Instant::now(),
            scale_factor: 1.0,
            pan: [0.0, 0.0, 0.0],
            opacity: 1.0,
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

    /// Function-level comment: Attaches a basic mesh render context for the orientation cube
    pub fn attach_orientation_cube_context(&mut self, ctx: std::sync::Arc<BasicMeshContext>) {
        self.orientation_cube_ctx = Some(ctx);
        log::debug!(
            "MeshView::attach_orientation_cube_context - Orientation cube context attached"
        );
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

    /// Function-level comment: Enable or disable rotation animation.
    pub fn set_rotation_enabled(&mut self, enabled: bool) {
        self.rotation_enabled = enabled;
        if enabled {
            // Reset timing when enabling rotation to prevent jumps
            self.last_frame_time = Instant::now();
            log::info!(
                "Mesh rotation enabled at {:.1}°/s",
                self.rotation_speed.to_degrees()
            );
        } else {
            log::info!("Mesh rotation disabled");
        }
    }

    /// Function-level comment: Set the rotation speed in radians per second.
    /// Positive values rotate counter-clockwise when viewed from above (standard Y-up convention).
    /// Common values: π/4 (45°/s), π/2 (90°/s), π (180°/s), 2π (360°/s)
    pub fn set_rotation_speed(&mut self, speed_rad_per_sec: f32) {
        self.rotation_speed = speed_rad_per_sec;
        log::info!(
            "Mesh rotation speed set to {:.3} rad/s ({:.1}°/s)",
            speed_rad_per_sec,
            speed_rad_per_sec.to_degrees()
        );
    }

    /// Function-level comment: Get the current rotation speed in radians per second.
    pub fn get_rotation_speed(&self) -> f32 {
        self.rotation_speed
    }

    /// Function-level comment: Reset the rotation angle to zero.
    /// Useful for returning to a known orientation or synchronizing multiple objects.
    pub fn reset_rotation(&mut self) {
        self.rotation_quat = Quat::IDENTITY;
        self.last_frame_time = Instant::now();
        log::debug!("Mesh rotation reset to identity");
    }

    /// Function-level comment: Get the current rotation angle in degrees (Pitch, Yaw, Roll).
    pub fn get_rotation_quat(&self) -> [f32; 4] {
        let (roll, pitch, yaw) = self.rotation_quat.to_euler(glam::EulerRot::XYZ);
        [roll.to_degrees(), pitch.to_degrees(), yaw.to_degrees(), 0.0]
    }

    /// Function-level comment: Set the current rotation angle using degrees for convenience.
    /// This directly sets the orientation without affecting rotation speed.
    pub fn set_rotation_angle_degrees(&mut self, degrees: [f32; 3]) {
        let rad = degrees.map(|d| d.to_radians());
        // Map [x, y, z] to Pitch, Yaw, Roll.
        // Using XYZ order: Pitch(X), Yaw(Y), Roll(Z)
        self.rotation_quat = Quat::from_euler(glam::EulerRot::XYZ, rad[0], rad[1], rad[2]);
        self.last_frame_time = Instant::now();
        log::info!("Mesh rotation set to {:?}°", degrees);
    }

    /// Function-level comment: Rotate the mesh based on mouse movement (pixels).
    /// Allows 3-axis rotation using only 2D input by accumulating rotations.
    /// dx: horizontal mouse movement (rotates around Global Y)
    /// dy: vertical mouse movement (rotates around Local X)
    pub fn rotate_by_mouse(&mut self, dx: f32, dy: f32) {
        let sensitivity = 0.03;
        let drag = Vec2::new(dx, dy);
        let angle = drag.length() * sensitivity;

        if angle.abs() < 1e-6 {
            return;
        }

        // screen space direction to rotation axis (around Z vertical screen)
        let axis = Vec3::new(-drag.x, -drag.y, 0.0).normalize();
        let rot = Quat::from_axis_angle(axis, angle);

        self.rotation_quat = rot * self.rotation_quat;
        self.rotation_quat = self.rotation_quat.normalize();
    }

    /// Function-level comment: Set rotation speed using degrees per second for convenience.
    /// This is a helper method that converts degrees to radians internally.
    pub fn set_rotation_speed_degrees(&mut self, degrees_per_sec: f32) {
        self.set_rotation_speed(degrees_per_sec.to_radians());
    }

    /// Function-level comment: Set uniform scale factor applied to the mesh model.
    /// Typical values: 0.25 (very small) .. 2.0 (double size). Default is 1.0.
    pub fn set_scale_factor(&mut self, scale: f32) {
        // Clamp to a reasonable range to avoid clipping or degenerate matrices
        let clamped = scale.clamp(0.001, 100.0);
        self.scale_factor = clamped;
        log::info!("Mesh scale factor set to {:.3}", clamped);
    }

    /// Function-level comment: Get the current uniform scale factor.
    pub fn get_scale_factor(&self) -> f32 {
        self.scale_factor
    }

    /// Function-level comment: Reset the uniform scale factor to default (0.5).
    pub fn reset_scale_factor(&mut self) {
        self.scale_factor = 1.0;
        log::info!("Mesh scale factor reset to default (1.0)");
    }

    /// Function-level comment: Set mesh pan translation (world units) for X and Y axes.
    /// Pan values are uploaded to the vertex shader as a uniform offset.
    pub fn set_pan(&mut self, dx: f32, dy: f32) {
        self.pan[0] = dx;
        self.pan[1] = dy;
        log::info!("Mesh pan offset set to ({}, {})", dx, dy);
    }

    /// Function-level comment: Get the current pan translation offset.
    pub fn get_pan(&self) -> [f32; 3] {
        self.pan
    }

    /// Function-level comment: Reset mesh pan translation to the origin.
    pub fn reset_pan(&mut self) {
        self.pan = [0.0, 0.0, 0.0];
        log::info!("Mesh pan reset to (0, 0, 0)");
    }

    /// Function-level comment: Set mesh opacity (0.0 transparent .. 1.0 opaque).
    pub fn set_opacity(&mut self, alpha: f32) {
        self.opacity = alpha.clamp(0.0, 1.0);
        log::info!("Mesh opacity set to {:.3}", self.opacity);
    }

    pub fn reset_opacity(&mut self) {
        self.opacity = 1.0;
        log::info!("Mesh opacity reset to default (1.0)");
    }

    /// Function-level comment: Create a default camera compatible with orthogonal projection
    /// Ensures the entire unit cube is visible without clipping when using orthographic mode.
    fn create_default_camera(&self) -> Camera {
        // Start with orthogonal projection camera
        let mut camera = Camera::new(); // uses Orthogonal by default

        // Set standard viewing parameters
        camera.eye = [0.0, 0.0, 3.0]; // Camera in front of the scene
        camera.center = [0.0, 0.0, 0.0]; // Look at origin
        camera.up = [0.0, 1.0, 0.0]; // Y-up coordinate system

        // Fix clipping issues: near and far planes must include the scene
        // For orthographic projection, near MUST be < far; negative near is allowed.
        camera.near = -5.0; // Allow objects between camera and center
        camera.far = 5.0; // Small range improves depth precision

        // Setup orthogonal bounds to ensure a unit cube (-1..1) fits inside view
        camera.ortho_left = -2.0;
        camera.ortho_right = 2.0;
        camera.ortho_bottom = -2.0;
        camera.ortho_top = 2.0;

        camera
    }

    /// Function-level comment: Create default lighting setup for basic mesh illumination.
    /// Returns lighting with reasonable defaults for 3D mesh viewing.
    fn create_default_lighting(&self) -> Lighting {
        Lighting {
            direction: [0.6, -0.7, 0.3],
            light_color: [1.0, 1.0, 1.0],
            light_intensity: 1.0,
            ambient_color: [0.4, 0.4, 0.5],
            ambient_intensity: 0.4,
        }
    }

    /// Function-level comment: Update GPU uniforms for basic mesh rendering with combined MVP matrix
    /// Includes rotation if enabled, using frame-rate independent timing
    pub fn update_uniforms(&mut self, queue: &wgpu::Queue) {
        // Keep angle in [0, 2π] range to prevent floating point precision issues
        // use std::f32::consts::TAU; // TAU = 2π

        // Update rotation angle only when rotation is enabled; orientation should persist when disabled
        if self.rotation_enabled {
            let current_time = Instant::now();
            let delta_time = current_time
                .duration_since(self.last_frame_time)
                .as_secs_f32();

            // Auto-rotate around Global Y
            let angle_delta = self.rotation_speed * delta_time;
            let rot_delta = Quat::from_rotation_y(angle_delta);
            self.rotation_quat = rot_delta * self.rotation_quat;
            self.rotation_quat = self.rotation_quat.normalize();

            #[cfg(feature = "trace-logging")]
            log::trace!(
                "[MESH_ROTATION] Auto-rotation delta: {:.3} rad",
                angle_delta
            );

            self.last_frame_time = current_time;
        }

        if let Some(ctx) = &self.ctx {
            let aspect_ratio = if self.dim.0 > 0 && self.dim.1 > 0 {
                self.dim.0 as f32 / self.dim.1 as f32
            } else {
                1.0
            };

            log::debug!(
                "[BASIC_MESH_UNIFORMS] Viewport dimensions: {}x{}, aspect_ratio: {:.3}",
                self.dim.0,
                self.dim.1,
                aspect_ratio
            );

            // Model matrix - apply persistent rotation (if any) and uniform scale
            let rotation = Mat4::from_quat(self.rotation_quat);

            // Compose Model: Translation * Rotation * Scale
            let model_matrix = Mat4::from_translation(Vec3::from(self.pan))
                * rotation
                * Mat4::from_scale(Vec3::splat(self.scale_factor));

            // View matrix - camera positioned for optimal viewing of smaller cube
            // Camera at (0, 0, 3.0) looking at origin, but original matrix
            // had specific translation (0, 0, -2.0) hardcoded in the array.
            let view_matrix = Mat4::from_translation(Vec3::new(0.0, 0.0, -2.0));

            // Dynamic orthographic range adjustment
            let base_extent = 2.0;
            let expand_factor = 1.8;
            let view_extent = base_extent * expand_factor;
            let left = -view_extent;
            let right = view_extent;
            let bottom = -view_extent / aspect_ratio;
            let top = view_extent / aspect_ratio;
            let near = -view_extent * 2.0;
            let far = view_extent * 2.0;

            // Matches original OpenGL-style orthographic projection (-1 to 1 Z range)
            let proj_matrix = Mat4::orthographic_rh(left, right, bottom, top, near, far);

            // Calculate MVP: projection * view * model
            let mvp_matrix = proj_matrix * view_matrix * model_matrix;

            #[cfg(feature = "trace-logging")]
            log::trace!("[BASIC_MESH_MATRICES] Model matrix (scale {} with rotation {:?} rad, enabled={}): {:?}", 
                        self.scale_factor, angles, self.rotation_enabled, model_matrix.to_cols_array());
            log::trace!(
                "[BASIC_MESH_MATRICES] View matrix (camera at -3): {:?}",
                view_matrix.to_cols_array()
            );
            log::trace!(
                "[BASIC_MESH_MATRICES] Projection matrix (orthogonal): {:?}",
                proj_matrix.to_cols_array()
            );
            log::trace!(
                "[BASIC_MESH_MATRICES] Combined MVP matrix: {:?}",
                mvp_matrix.to_cols_array()
            );

            // Update uniforms in BasicMeshContext with combined MVP matrix
            // Note: The shader expects column-major matrices, and glam is column-major by default.

            ctx.update_uniforms(queue, &mvp_matrix.to_cols_array_2d());

            // Update lighting uniforms to ensure lighting effects are applied
            let mut lighting_uniforms = if let Some(ref lighting) = self.lighting {
                // Use the configured lighting and convert to BasicLightingUniforms
                lighting.to_basic_uniforms()
            } else {
                // Create default lighting if none is configured
                let default_lighting = self.create_default_lighting();
                default_lighting.to_basic_uniforms()
            };
            lighting_uniforms.opacity = self.opacity;
            ctx.update_lighting(queue, lighting_uniforms);
        }

        // Update Orientation Cube Uniforms
        if let Some(cube_ctx) = &self.orientation_cube_ctx {
            // Model: Only Rotation (no pan, no scale from main mesh)
            let rotation = Mat4::from_quat(self.rotation_quat);
            let model_matrix = rotation; // Scale 1.0, Trans 0.0

            // View: Standard fixed camera
            // Place cube closer to camera (Z=5.0) than main mesh (Z=-2.0) to ensure it renders on top
            // Orthographic range is -10 to 10, so 5.0 is well within range.
            let view_matrix = Mat4::from_translation(Vec3::new(0.0, 0.0, 5.0));

            // Proj: Fixed Ortho to fit unit cube (-1..1) with padding
            // Unit cube diagonal is 1.73. 1.5 might clip corners if rotating.
            let extent = 2.0;
            let proj_matrix = Mat4::orthographic_rh(-extent, extent, -extent, extent, -10.0, 10.0);

            let mvp_matrix = proj_matrix * view_matrix * model_matrix;

            cube_ctx.update_uniforms(queue, &mvp_matrix.to_cols_array_2d());

            // Also update lighting for cube (use same lighting)
            let lighting_uniforms = if let Some(ref lighting) = self.lighting {
                lighting.to_basic_uniforms()
            } else {
                let default_lighting = self.create_default_lighting();
                default_lighting.to_basic_uniforms()
            };
            // Cube is always opaque
            let mut cube_lighting = lighting_uniforms;
            cube_lighting.opacity = 1.0;
            cube_ctx.update_lighting(queue, cube_lighting);
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
        self.consecutive_errors < 5
            && self.last_success_time.elapsed().as_secs_f64() < 30.0
            && !matches!(self.fallback_mode, FallbackMode::Disabled)
    }

    /// Function-level comment: Handle rendering errors and determine appropriate fallback strategy.
    fn handle_render_error(&mut self, error: MeshRenderError) {
        self.stats.error_count += 1;
        self.consecutive_errors += 1;

        log::warn!(
            "Mesh render error (consecutive: {}): {}",
            self.consecutive_errors,
            error
        );

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
            1..=2 => FallbackMode::Normal,     // Retry normal rendering
            3..=4 => FallbackMode::Simplified, // Switch to simplified rendering
            5..=7 => FallbackMode::Wireframe,  // Fall back to wireframe
            _ => FallbackMode::Disabled,       // Disable rendering entirely
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
            log::info!(
                "Mesh rendering recovered after {} consecutive errors",
                self.consecutive_errors
            );
        }

        self.last_success_time = Instant::now();

        // Gradually return to normal mode if we've been in fallback
        if matches!(
            self.fallback_mode,
            FallbackMode::Simplified | FallbackMode::Wireframe
        ) {
            if self.stats.frame_count % 60 == 0 {
                // Try to upgrade every 60 frames
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
            return Err(MeshRenderError::ResourceError(
                "Rendering disabled due to repeated failures".to_string(),
            ));
        }

        // Ensure context is available
        let ctx = self.ctx.as_ref().ok_or_else(|| {
            log::trace!("BasicMeshView::try_render - No context attached");
            MeshRenderError::ContextNotAttached
        })?;

        log::trace!(
            "BasicMeshView::try_render - Context available, vertices: {}, indices: {}",
            ctx.num_vertices,
            ctx.num_indices
        );

        // Validate viewport dimensions
        if self.dim.0 == 0 || self.dim.1 == 0 {
            return Err(MeshRenderError::ViewportError(
                "Invalid viewport dimensions".to_string(),
            ));
        }

        // Configure viewport
        let (x, y) = (self.pos.0 as f32, self.pos.1 as f32);
        let (width, height) = (self.dim.0 as f32, self.dim.1 as f32);
        render_pass.set_viewport(x, y, width, height, 0.0, 1.0);

        log::trace!(
            "BasicMeshView::try_render - Viewport set to ({}, {}) {}x{}",
            x,
            y,
            width,
            height
        );

        // Use the simplified BasicMeshContext render method
        ctx.render(render_pass);

        // Render Orientation Cube (if available)
        if let Some(cube_ctx) = &self.orientation_cube_ctx {
            let cube_size = 120.0;
            let padding = 10.0;

            // Calculate bottom-left position within the view
            // Assuming (x, y) is top-left of the view
            let view_x = self.pos.0 as f32;
            let view_y = self.pos.1 as f32;
            let view_h = self.dim.1 as f32;

            // Bottom-left relative to view
            let cube_x = view_x + padding;
            let cube_y = view_y + view_h - cube_size - padding;

            // Ensure we don't draw outside the view if view is too small
            if self.dim.0 > (cube_size as u32 + 20) && self.dim.1 > (cube_size as u32 + 20) {
                render_pass.set_viewport(cube_x, cube_y, cube_size, cube_size, 0.0, 1.0);
                cube_ctx.render(render_pass);
            }
        }

        // Record successful render
        let render_time_ms = start_time.elapsed().as_millis_f32();
        self.record_success(render_time_ms);
        log::trace!(
            "BasicMeshView::try_render - Render completed successfully in {:.2}ms",
            render_time_ms
        );

        // End frame timing and check for quality adjustments
        if let Some(new_quality) = self.end_frame_timing() {
            log::info!(
                "Quality automatically adjusted to {:?} based on performance",
                new_quality
            );
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
        if self.stats.frame_count % 300 == 0 {
            // Every 5 seconds at 60fps
            if !self.is_healthy() {
                log::warn!("MeshView health check failed: consecutive_errors={}, last_success={:?}s ago, mode={:?}", 
                    self.consecutive_errors, 
                    self.last_success_time.elapsed().as_secs_f64(),
                    self.fallback_mode
                );
            }
        }

        // Auto-recovery attempt if we've been disabled for too long
        if matches!(self.fallback_mode, FallbackMode::Disabled)
            && self.last_success_time.elapsed().as_secs_f64() > 60.0
        {
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
        log::debug!(
            "[MESH_VIEW] Dropping MeshView at position {:?} with size {:?}",
            self.pos,
            self.dim
        );
    }
}

impl View for MeshView {
    fn position(&self) -> (i32, i32) {
        self.pos
    }
    fn dimensions(&self) -> (u32, u32) {
        self.dim
    }
    fn move_to(&mut self, pos: (i32, i32)) {
        log::debug!("[MESH_VIEW] Moving to position: {:?}", pos);
        self.pos = pos;
    }
    fn resize(&mut self, dim: (u32, u32)) {
        log::debug!("[MESH_VIEW] Resizing to dimensions: {:?}", dim);
        self.dim = dim;
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, PI, TAU};

    /// Function-level comment: Verify default rotation state and speed
    #[test]
    fn test_rotation_api_basic_functionality() {
        let mesh_view = MeshView::default();
        let quat = mesh_view.get_rotation_quat();
        assert!(quat[0].abs() < 1e-6);
        assert!(quat[1].abs() < 1e-6);
        assert!(quat[2].abs() < 1e-6);
        assert!(quat[3].abs() < 1e-6);
        assert!((mesh_view.get_rotation_speed() - FRAC_PI_2).abs() < 1e-6);
    }

    /// Function-level comment: Ensure enabling/disabling rotation does not panic and preserves orientation
    #[test]
    fn test_rotation_enable_disable() {
        let mut mesh_view = MeshView::default();
        let before = mesh_view.get_rotation_quat();
        mesh_view.set_rotation_enabled(false);
        let after_disable = mesh_view.get_rotation_quat();
        mesh_view.set_rotation_enabled(true);
        let after_enable = mesh_view.get_rotation_quat();

        // Quaternions should be identical
        assert!((before[0] - after_disable[0]).abs() < 1e-6);
        assert!((before[1] - after_disable[1]).abs() < 1e-6);
        assert!((before[2] - after_disable[2]).abs() < 1e-6);
        assert!((before[3] - after_disable[3]).abs() < 1e-6);

        assert!((before[0] - after_enable[0]).abs() < 1e-6);
        assert!((before[1] - after_enable[1]).abs() < 1e-6);
        assert!((before[2] - after_enable[2]).abs() < 1e-6);
        assert!((before[3] - after_enable[3]).abs() < 1e-6);
    }

    /// Function-level comment: Verify rotation speed setters
    #[test]
    fn test_rotation_speed_control() {
        let mut mesh_view = MeshView::default();
        let test_speed = PI / 4.0; // 45°/s
        mesh_view.set_rotation_speed(test_speed);
        assert!((mesh_view.get_rotation_speed() - test_speed).abs() < 1e-6);
        mesh_view.set_rotation_speed_degrees(180.0); // π rad/s
        assert!((mesh_view.get_rotation_speed() - PI).abs() < 1e-6);
    }

    /// Function-level comment: Reset rotation and verify default orientation (Identity)
    #[test]
    fn test_rotation_angle_reset() {
        let mut mesh_view = MeshView::default();
        mesh_view.reset_rotation();
        let quat = mesh_view.get_rotation_quat();
        assert!(quat[0].abs() < 1e-6);
        assert!(quat[1].abs() < 1e-6);
        assert!(quat[2].abs() < 1e-6);
        assert!(quat[3].abs() < 1e-6);
    }
}
