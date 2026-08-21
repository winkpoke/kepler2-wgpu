#![allow(dead_code)]

use super::{
    basic_mesh_context::MultiMeshContext,
    mesh::{BasicLightingUniforms, Mesh, MeshRenderContext, VolumeUniforms},
    performance::{PerformanceStats, QualityController, QualityLevel},
};
use crate::{
    core::{timing::Instant, KeplerResult, WindowLevel},
    rendering::view::{Renderable, View, NeedleUniform, ObliquePlaneUniform},
    rendering::view::camera::Camera
};
use glam::{Mat4, Quat, Vec3};
use std::f32::consts::FRAC_PI_2;
use std::sync::{Arc, Mutex};

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
    view_id: usize,
    volume_ctx: Option<Arc<MeshRenderContext>>,
    spine_ctx: Option<Arc<Mutex<MultiMeshContext>>>,
    needle_ctx: Option<Arc<Mutex<MultiMeshContext>>>,
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
    /// Rotation speed in radians per second (default: π/2 = 90 degrees/second)
    rotation_speed: f32,
    /// Last frame time for rotation calculation
    last_frame_time: Instant,
    opacity: f32,
    /// Spine mesh lighting uniforms (controls direction, color, opacity)
    spine_lighting: BasicLightingUniforms,
    roi_min: [f32; 3],
    roi_max: [f32; 3],
    window_level: WindowLevel,
    slab_thickness: f32,
    mode: usize,
    needle_enabled: f32,
    needle_index: u32,
    plane_rotation_angle: f32,
    needles: Vec<NeedleUniform>,
    needle_unit: Option<Mesh>,
    oblique_planes: [ObliquePlaneUniform; 4],
    camera: Camera,
}

impl MeshView {
    pub fn new() -> Self {
        Self {
            view_id: 0,
            volume_ctx: None,
            spine_ctx: None,
            needle_ctx: None,
            pos: (0, 0),
            dim: (0, 0),
            stats: RenderStats::default(),
            fallback_mode: FallbackMode::default(),
            consecutive_errors: 0,
            last_success_time: Instant::now(),
            quality_controller: QualityController::default(),
            rotation_enabled: false,
            rotation_speed: FRAC_PI_2, // 90 degrees per second (only used when enabled)
            last_frame_time: Instant::now(),
            opacity: 1.0,
            spine_lighting: BasicLightingUniforms::default(),
            roi_min: [0.0, 0.0, 0.0],
            roi_max: [1.0, 1.0, 1.0],
            window_level: WindowLevel {
                window_level: 300.0,
                window_width: 300.0,
                ..Default::default()
            },
            slab_thickness: 1.25,
            mode: 1,
            needle_enabled: 0.0,
            needle_index: 0,
            plane_rotation_angle: 180.0,
            needles: Vec::new(),
            needle_unit: None,
            oblique_planes: [ObliquePlaneUniform::default(); 4],
            camera: Camera::new(),
        }
    }

    pub fn view_id(&self) -> usize {
        self.view_id
    }

    /// Function-level comment: Attaches a volume render context for GPU operations
    pub fn attach_context(&mut self, ctx: std::sync::Arc<MeshRenderContext>) {
        self.volume_ctx = Some(ctx);
        log::debug!("MeshView::attach_context - Volume context attached successfully");
    }

    /// Function-level comment: Attaches a spine render context for GPU operations
    pub fn attach_spine_context(&mut self, ctx: MultiMeshContext) {
        self.spine_ctx = Some(Arc::new(Mutex::new(ctx)));
        log::debug!("MeshView::attach_spine_context - Spine context attached successfully");
    }

    pub fn attach_needle_context(&mut self, ctx: MultiMeshContext) {
        self.needle_ctx= Some(Arc::new(Mutex::new(ctx)));
        log::debug!("MeshView::attach_needle_context - Needle context attached successfully");
    }

    // /// Re-bake every needle's transform into vertex data and upload to the needle contex
    // pub fn rebuild_needle_meshes(&self, device: &wgpu::Device) {
    //     let (Some(ctx), Some(unit)) = (&self.needle_ctx, &self.needle_unit) else {
    //         return;
    //     };
    //     let meshes: Vec<Mesh> = self.needles.iter().map(|n| {
    //         log::info!("Needle: {:?}", n);
    //         Mesh::instance_for_needle(
    //             unit,
    //             glam::Vec3::from(n.entry),
    //             glam::Vec3::from(n.tip),
    //             n.radius,
    //             [1.0, 1.0, 1.0, 1.0]
    //         )
    //     }).collect();
    //     if let Ok(mut guard) = ctx.lock() {
    //         guard.set_meshes(device, &meshes);
    //         log::info!(
    //             "MeshView::rebuild_needle_meshes - uploaded {} needle instances",
    //             meshes.len()
    //         );
    //     }
    // }

    pub fn set_meshes(&self, id: u32, kind: u32, device: &wgpu::Device, meshes: Arc<Vec<Mesh>>) {
        let ctx = match kind {
            0 | 3 => &self.spine_ctx,
            1 => &self.needle_ctx,
            _ => {
                log::warn!("MeshView::set_meshes - unknown kind {}", kind);
                return;
            }
        };

        if let Some(ctx) = ctx {
            if let Ok(mut guard) = ctx.lock() {
                guard.set_meshes(id, kind, device, &meshes);
            }
        }
    }

    /// Function-level comment: Toggle visibility of a single vertebra by label id.
    pub fn set_spine_visibility(&self, label_id: u8, visible: bool) {
        if let Some(ctx) = &self.spine_ctx {
            if let Ok(mut guard) = ctx.lock() {
                guard.set_label_visibility(label_id, visible);
            }
        }
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
        let mut used = 0;
        let mut total = 0;

        // volume
        if let Some(vol_ctx) = &self.volume_ctx {
            let (u, t, _, _) = vol_ctx.get_memory_stats();
            used += u;
            total += t;
        }

        let ratio = if total > 0 {
            used as f32 / total as f32
        } else {
            0.0
        };

        (used, total, ratio, 0.0)
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
        self.camera.set_model_rotation_quat(Quat::IDENTITY);
        self.last_frame_time = Instant::now();
        log::debug!("Mesh rotation reset to identity");
    }

    /// Apply an incremental orbit in degrees
    pub fn set_rotation_angle_degrees(&mut self, degrees_x: f32, degrees_y: f32) {
        self.camera.orbit_angles(degrees_y.to_radians(), degrees_x.to_radians());
        self.last_frame_time = Instant::now();
        log::info!(
            "Mesh rotation set to (deg_x: {}, deg_y: {})",
            degrees_x,
            degrees_y
        );
    }

    /// Set Mesh rotation angles in degrees around X, Y, Z axes.
    pub fn set_rotation_degrees(&mut self, roll_deg: f32, yaw_deg: f32, pitch_deg: f32) {
        let (roll, yaw, pitch) = (
            roll_deg.to_radians(),
            yaw_deg.to_radians(),
            pitch_deg.to_radians(),
        );
        let rot = Mat4::from_rotation_x(roll) * Mat4::from_rotation_y(yaw) * Mat4::from_rotation_z(pitch);
        self.camera.set_model_rotation_quat(Quat::from_mat4(&rot));
        self.last_frame_time = Instant::now();
    }

    pub fn set_rotation_quat(&mut self, rotation: [f32; 4]) -> KeplerResult<()> {
        self.camera.set_model_rotation_quat(Quat::from_array(rotation));
        self.last_frame_time = Instant::now();
        log::info!("Mesh rotation set to {:?}", rotation);
        Ok(())
    }

    pub fn get_rotation_quat(&self) -> Quat {
        self.camera.model_rotation_quat()
    }

    /// Shared camera accessors for external orchestration.
    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    /// Function-level comment: Set rotation speed using degrees per second for convenience.
    /// This is a helper method that converts degrees to radians internally.
    pub fn set_rotation_speed_degrees(&mut self, degrees_per_sec: f32) {
        self.set_rotation_speed(degrees_per_sec.to_radians());
    }

    /// Function-level comment: Set uniform zoom applied through the shared camera.
    /// Typical values: 0.25 (zoomed out) .. 2.0 (2x magnification). Default is 1.0.
    pub fn set_scale_factor(&mut self, scale: f32) {
        self.camera.set_zoom(scale);
        log::info!("Mesh scale factor set to {:.3}", self.camera.get_zoom());
    }

    /// Function-level comment: Get the current uniform scale factor.
    pub fn get_scale_factor(&self) -> f32 {
        self.camera.get_zoom()
    }

    /// Function-level comment: Reset the uniform scale factor to default (1.0).
    pub fn reset_scale_factor(&mut self) {
        self.camera.set_zoom(1.0);
        log::info!("Mesh scale factor reset to default (1.0)");
    }

    pub fn set_slab_thickness(&mut self, thickness: f32) {
        self.slab_thickness = thickness;
        log::info!("Mesh slab thickness set to {:.3} mm", thickness);
    }

    /// Function-level comment: Set view-plane pan (world units) through the shared camera.
    /// Positive x moves the image right, positive y moves it up.
    pub fn set_pan(&mut self, dx: f32, dy: f32) {
        self.camera.set_pan(dx, dy);
        log::info!("Mesh pan offset set to ({}, {})", dx, dy);
    }

    /// Function-level comment: Get the current pan translation offset.
    pub fn get_pan(&self) -> [f32; 3] {
        let pan = self.camera.get_pan();
        [pan.x, pan.y, 0.0]
    }

    /// Function-level comment: Reset mesh pan translation to the origin.
    pub fn reset_pan(&mut self) {
        self.camera.set_pan(0.0, 0.0);
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

    pub fn get_opacity(&self) -> f32 {
        self.opacity
    }

    pub fn set_mode(&mut self, mode: usize) {
        self.mode = mode;
        log::info!("Mesh mode set to {:?}", mode);
    }

    pub fn set_roi(&mut self, min: [f32; 3], max: [f32; 3]) {
        self.roi_min = min;
        self.roi_max = max;
    }

    pub fn reset_roi(&mut self) {
        self.roi_min = [0.0, 0.0, 0.0];
        self.roi_max = [1.0, 1.0, 1.0];
    }

    pub fn set_needle_enabled(&mut self, enabled: f32) {
        self.needle_enabled = enabled;
        log::info!("[NEEDLE] Mesh needle rendering {}", enabled);
    }

    pub fn set_new_needle(&mut self, id: u32, entry: [f32; 3], pos: [f32; 3], color: [f32; 4]) {
        let needle = self.needles.iter_mut().find(|n| n.id == id);
        if let Some(needle) = needle {
            needle.entry = entry;
            needle.tip = pos;
            needle.color = color;
        } else {
            self.needles.push(NeedleUniform {
                entry, 
                tip: pos, 
                radius: NeedleUniform::default().radius, 
                id, 
                color
            });
            log::info!("[NEEDLE] set_new_needle {} : entry={:?}, pos={:?}",id, entry, pos);
        }
    }

    pub fn set_needle_position(&mut self, id: u32, pos: [f32; 3]) {
        let needle = self.needles.iter_mut().find(|n| n.id == id);
        if let Some(needle) = needle {
            needle.tip = pos;
        }
        log::debug!("[NEEDLE]Mesh needle {} position set to {:?}", id, pos);
    }

    /// Set the visibility of the oblique cutting plane.
    ///
    /// # Arguments
    /// * `center`    - world-space center of the plane
    /// * `normal`    - world-space normal of the plane (will be normalized
    ///                 in-shader; do not assume a unit length on the CPU side)
    /// * `index`     - index of the plane to set
    /// * `visible`   - whether the plane is composited this frame
    /// * `alpha`     - 0..=1 compositing opacity shared by all 4 planes
    pub fn set_oblique_plane(&mut self, center: [f32; 3], normal: [f32; 3], index: usize, oblique_crop: f32, alpha: f32) {
        self.oblique_planes[index] = ObliquePlaneUniform {
            center,
            visible: oblique_crop,
            normal,
            plane_alpha: alpha,
        };
        log::info!(
            "[OBLIQUE→3D] View {:?} set_oblique_plane: center={:?} normal={:?} oblique_crop={} alpha={}",
            index, center, normal, oblique_crop, alpha
        );
    }

    pub fn get_oblique_planes(&self) -> [ObliquePlaneUniform; 4] {
        self.oblique_planes
    }

    /// Set the slice-clip plane (UV-space `normal · x + d = 0`) used to clip
    /// the spine mesh to a thin slab intersecting the current MPR plane.
    pub fn set_slice_clip(
        &self,
        queue: &wgpu::Queue,
        plane_normal: [f32; 3],
        plane_d: f32,
        enabled: bool,
    ) {
        if let Some(needle_ctx) = &self.needle_ctx {
            if let Ok(mut guard) = needle_ctx.lock() {
                guard.set_slice_plane(queue, plane_normal, plane_d, self.slab_thickness, enabled);
            }
        }
    }

    pub fn set_needle_radius(&mut self, id: u32, radius: f32) {
        let needle = self.needles.iter_mut().find(|n| n.id == id);
        if let Some(needle) = needle {
            needle.radius = radius.clamp(0.0004, 0.04);
        }
        log::debug!("[NEEDLE]Mesh needle radius set to {:.6}", radius);
    }

    pub fn set_needle_angle(&mut self, id: u32, angle: f32) -> Option<(Vec3, Vec3)> {
        if self.needle_enabled <= 1.5 {
            return None;
        }
        let index = self.needles.iter().position(|n| n.id == id)?;
        self.needle_index = index as u32;
        self.plane_rotation_angle = angle;
        Some(self.needles[index].plane_from_needle(angle))
    }

    pub fn set_window_level(&mut self, window: f32) -> KeplerResult<()> {
        let _ = self.window_level.set_window_level(window);
        log::info!("MIP window set to {:.3}", window);
        Ok(())
    }

    pub fn set_window_width(&mut self, window_width: f32) -> KeplerResult<()> {
        let _ = self.window_level.set_window_width(window_width);
        log::info!("MIP window width set to {:.3}", window_width);
        Ok(())
    }

    /// Mesh-only view projection that matches the DVR volume's screen mapping.
    fn mesh_view_projection(&self, aspect_ratio: f32) -> Mat4 {
        Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0)) * self.camera.view_projection_matrix(aspect_ratio)
    }

    /// Function-level comment: Update GPU uniforms for basic mesh rendering with combined MVP matrix
    /// Includes rotation if enabled, using frame-rate independent timing
    pub fn update_uniforms(&mut self, queue: &wgpu::Queue) {
        // Update rotation angle only when rotation is enabled; orientation should persist when disabled
        if self.rotation_enabled {
            let current_time = Instant::now();
            let delta_time = current_time
                .duration_since(self.last_frame_time)
                .as_secs_f32();

            // Auto-rotate the model around the view-up axis via the shared camera
            let angle_delta = self.rotation_speed * delta_time;
            self.camera.azimuth(-angle_delta);
            self.last_frame_time = current_time;
        }

        // Extract volume dimensions — shared by both volume and needle transforms
        let vol_extent = self.volume_ctx
            .as_ref()
            .map(|ctx| ctx.render_content.texture.size())
            .unwrap_or_default();
        let w = vol_extent.width.max(1) as f32;
        let h = vol_extent.height.max(1) as f32;
        let d = vol_extent.depth_or_array_layers.max(1) as f32;
        let phys = Vec3::new(w, h, d * self.slab_thickness);
        let max_extent = phys.max_element().max(1e-6);
        let volume_scale = phys / max_extent;

        let aspect_ratio = if self.dim.1 > 0 && self.dim.0 > 0 {
            self.dim.0 as f32 / self.dim.1 as f32
        } else {
            1.0
        };

        let view_projection = self.camera.view_projection_matrix(aspect_ratio);
        let inv_view_projection = self.camera.inverse_view_projection_matrix(aspect_ratio);
        let camera_position = self.camera.effective_position();

        // Volume uniforms
        if let Some(vol_ctx) = &self.volume_ctx {
            // Extract format and bias from RenderContent decode parameters
            let decode_params = vol_ctx.render_content.decode_parameters();
            let is_packed_rg8 = if decode_params.is_packed_flag == 1 { 1.0 } else { 0.0 };

            let mut gpu_needles = [NeedleUniform::default(); 32];
            for (i, needle) in self.needles.iter().take(32).enumerate() {
                gpu_needles[i] = NeedleUniform {
                    entry: needle.entry,
                    radius: needle.radius,
                    tip: needle.tip,
                    id: needle.id,
                    color: needle.color
                };
            }

            let vol_uniforms = VolumeUniforms {
                ray_step_size: 0.004,
                max_steps: 1500.0,
                is_packed_rg8: is_packed_rg8,
                bias: decode_params.bias,
                window: self.window_level.window_width(),
                level: self.window_level.window_level(),
                roi_min: self.roi_min,
                roi_max: self.roi_max,
                opacity: self.opacity,
                light_dir: [0.5, 0.5, -1.0],
                aspect_ratio,
                vol_dims: [w, h, d],
                view_proj: view_projection.to_cols_array(),
                inv_view_proj: inv_view_projection.to_cols_array(),
                camera_position: camera_position.to_array(),
                volume_scale: volume_scale.to_array(),
                needle_count: self.needles.len().min(32) as u32,
                needle_enabled: self.needle_enabled,
                needle_index: self.needle_index,
                plane_rotation_angle: self.plane_rotation_angle,
                oblique_planes: self.oblique_planes,
                needles: gpu_needles,
                ..Default::default()
            };

            // update
            vol_ctx.update_uniforms(queue, &vol_uniforms);
        }

        // Spine mesh uniforms and lighting. The mesh MVP carries the clip-space
        // Y flip so meshes track the DVR volume's mirrored screen mapping.
        let mesh_vp = self.mesh_view_projection(aspect_ratio);
        if let Some(spine_ctx) = &self.spine_ctx {
            if let Ok(mut guard) = spine_ctx.lock() {
                guard.update_uniforms(queue, &mesh_vp.to_cols_array_2d());
                self.spine_lighting.opacity = 0.5;
                guard.update_lighting(queue, self.spine_lighting);
            }
        }

        // Needle mesh uniforms and lighting (same flipped MVP as the spine)
        if let Some(needle_ctx) = &self.needle_ctx {
            if let Ok(mut guard) = needle_ctx.lock() {
                guard.update_uniforms(queue, &mesh_vp.to_cols_array_2d());
                self.spine_lighting.opacity = 1.0;
                guard.update_lighting(queue, self.spine_lighting);
            }
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

        // Render the translucent DVR volume
        if let Some(vol_ctx) = &self.volume_ctx {
            vol_ctx.render(render_pass);
        } else {
            log::warn!("BasicMeshView::try_render - Volume rendering requested but no volume context attached");
        }

        // Render the polygonal spine mesh SECOND, as a translucent overlay.
        if let Some(spine_ctx) = &self.spine_ctx {
            if let Ok(guard) = spine_ctx.lock() {
                guard.render(render_pass);
            }
        }

        // Render OBJ-needle meshes THIRD, only when needles are enabled
        if self.needle_enabled > 0.5 {
            if let Some(needle_ctx) = &self.needle_ctx {
                if let Ok(guard) = needle_ctx.lock() {
                    guard.render(render_pass);
                }
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
    use glam::Mat4;
    use std::f32::consts::{FRAC_PI_2, PI};

    /// Function-level comment: Verify default rotation state and speed
    #[test]
    fn test_rotation_default_identity() {
        let mesh_view = MeshView::new();
        assert!(mesh_view.camera().model_rotation_quat().abs_diff_eq(Quat::IDENTITY, 1e-6));

        // Check Matrix columns
        let mat = Mat4::from_quat(mesh_view.camera().model_rotation_quat());
        assert!(mat.abs_diff_eq(Mat4::IDENTITY, 1e-6));

        // Verify default speed (90 degrees/s)
        assert!((mesh_view.rotation_speed - FRAC_PI_2).abs() < 1e-6);
    }

    /// Function-level comment: Ensure enabling/disabling rotation does not panic and preserves orientation
    #[test]
    fn test_rotation_enable_disable() {
        let mut mesh_view = MeshView::new();

        // Set some rotation first to ensure we aren't just testing identity or default
        // Add 45 degrees around Y to the existing default
        mesh_view.set_rotation_angle_degrees(0.0, 45.0);

        let before = mesh_view.camera().model_rotation_quat();

        mesh_view.set_rotation_enabled(false);
        let after_disable = mesh_view.camera().model_rotation_quat();

        mesh_view.set_rotation_enabled(true);
        let after_enable = mesh_view.camera().model_rotation_quat();

        // Rotation should be preserved through enable/disable cycles
        assert_eq!(before, after_disable);
        assert_eq!(before, after_enable);
    }

    /// Function-level comment: Verify rotation speed setters
    #[test]
    fn test_rotation_speed_control() {
        let mut mesh_view = MeshView::new();
        let test_speed = PI / 4.0; // 45°/s

        mesh_view.set_rotation_speed(test_speed);
        assert!((mesh_view.get_rotation_speed() - test_speed).abs() < 1e-6);

        mesh_view.set_rotation_speed_degrees(180.0); // π rad/s
        assert!((mesh_view.get_rotation_speed() - PI).abs() < 1e-6);
    }

    #[test]
    fn test_set_rotation_degrees() {
        let mut mesh_view = MeshView::new();

        mesh_view.set_rotation_degrees(90.0, 0.0, 0.0);

        let expected = Quat::from_rotation_x(90.0_f32.to_radians());
        let q = mesh_view.camera().model_rotation_quat();
        // A rotation quaternion is double-covered: q and -q are the same
        // orientation, and the shared camera may return either sign.
        assert!(q.dot(expected).abs() > 0.9999, "expected {:?}, got {:?}", expected, q);
    }

    #[test]
    fn test_rotation_accumulation() {
        let mut mesh_view = MeshView::new();

        mesh_view.set_rotation_angle_degrees(0.0, 90.0);

        let q1 = mesh_view.camera().model_rotation_quat();

        mesh_view.set_rotation_angle_degrees(0.0, 90.0);

        let q2 = mesh_view.camera().model_rotation_quat();

        // Two distinct rotations must not represent the same orientation.
        assert!(q1.dot(q2).abs() < 0.9999);

        // Should approach 180° about Y (sign-agnostic).
        let expected = Quat::from_rotation_y(180.0_f32.to_radians());
        assert!(q2.dot(expected).abs() > 0.9999, "expected {:?}, got {:?}", expected, q2);
    }

    /// Function-level comment: Reset rotation and verify default orientation (Identity)
    #[test]
    fn test_rotation_angle_reset() {
        let mut mesh_view = MeshView::new();

        // Apply some rotation
        mesh_view.set_rotation_angle_degrees(90.0, 45.0);

        // Reset
        mesh_view.reset_rotation();
        assert!(mesh_view.camera().model_rotation_quat().abs_diff_eq(Quat::IDENTITY, 1e-6));
    }

    /// Regression guard: the polygonal mesh `model_view_proj` must place the
    /// `[0, 1]^3` texture box into the same world AABB the DVR uses (the
    /// identity, since both live in the same UV/world space). If this
    /// drifts, the mesh and DVR will display with different centers/sizes
    /// and the shared depth buffer can no longer be used for occlusion.
    #[test]
    fn test_spine_model_matches_dvr_aabb() {
        // The model matrix is the identity in the shared `[0, 1]^3` UV
        // space — both meshes and the DVR volume live in this space, so
        // the model transform contributes nothing.
        let model = Mat4::IDENTITY;

        // Texture origin (0,0,0) → world minimum corner (still (0,0,0)).
        let origin = (model * Vec3::ZERO.extend(1.0)).truncate();
        assert!(
            origin.abs_diff_eq(Vec3::ZERO, 1e-6),
            "origin should map to (0,0,0); got {:?}",
            origin,
        );

        // Texture corner (1,1,1) → world maximum corner.
        let corner = (model * Vec3::ONE.extend(1.0)).truncate();
        assert!(
            corner.abs_diff_eq(Vec3::ONE, 1e-6),
            "corner expected (1,1,1), got {:?}",
            corner,
        );

        // Texture center (0.5,0.5,0.5) → world volume center.
        let center = (model * Vec3::splat(0.5).extend(1.0)).truncate();
        assert!(
            center.abs_diff_eq(Vec3::splat(0.5), 1e-6),
            "center should map to (0.5, 0.5, 0.5); got {:?}",
            center,
        );

        // The mesh's local coordinates are the texture coordinates directly
        // (the model matrix is the identity), so any (tx, ty, tz) in
        // [0, 1]^3 should round-trip through the model matrix unchanged.
        let samples = [
            (0.0, 0.0, 0.0),
            (1.0, 0.0, 0.0),
            (0.0, 1.0, 0.0),
            (0.0, 0.0, 1.0),
            (0.5, 0.5, 0.5),
            (1.0, 1.0, 1.0),
        ];
        for (tx, ty, tz) in samples {
            let v = Vec3::new(tx, ty, tz);
            let world_from_mesh = (model * v.extend(1.0)).truncate();
            assert!(
                world_from_mesh.abs_diff_eq(v, 1e-5),
                "tex=({tx},{ty},{tz}): mesh->{world_from_mesh:?} expected {v:?}",
            );
        }
    }

    /// Regression guard: the mesh MVP must share the DVR volume's screen
    /// mapping. `volume.wgsl` samples each pixel through the vertically
    /// mirrored NDC position (`ndc_y = 1 - 2 * uv.y`), so a world point is
    /// displayed by the volume at `-ndc.y`. The mesh MVP therefore has to
    /// produce exactly that NDC for the same point — otherwise mesh and
    /// volume pan along Y in opposite directions (the bug this guards).
    #[test]
    fn test_mesh_mvp_matches_volume_screen_mapping() {
        let mut mesh_view = MeshView::new();
        let aspect = 1.0;
        let q = Vec3::new(0.5, 0.75, 0.5); // a feature inside the volume box

        let pans = [(0.0, 0.0), (0.0, 0.1), (0.0, -0.2), (0.3, 0.15)];
        let mut prev_mesh_y = None;
        let mut prev_vol_y = None;
        for (dx, dy) in pans {
            mesh_view.set_pan(dx, dy);

            let vp = mesh_view.camera().view_projection_matrix(aspect);
            let flipped = Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0)) * vp;

            let vol = vp * q.extend(1.0);
            let mesh = flipped * q.extend(1.0);

            // Same pixel: mesh NDC == volume's mirrored NDC.
            assert!(
                (mesh.y / mesh.w - -(vol.y / vol.w)).abs() < 1e-6,
                "pan=({dx},{dy}): mesh ndc.y {} != volume ndc.y {}",
                mesh.y / mesh.w,
                -(vol.y / vol.w),
            );
            assert!((mesh.x / mesh.w - vol.x / vol.w).abs() < 1e-6);

            // Depth channel untouched: occlusion against the DVR first-hit
            // depth stays valid.
            assert!((mesh.z / mesh.w - vol.z / vol.w).abs() < 1e-6);

            // Pan response moves mesh and volume the same way (Y increases
            // together as dy decreases from +0.1 to -0.2).
            if let (Some(pv_m), Some(pv_v)) = (prev_mesh_y, prev_vol_y) {
                let d_mesh = mesh.y / mesh.w - pv_m;
                let d_vol = -(vol.y / vol.w) - pv_v;
                assert!(
                    d_mesh * d_vol > 0.0,
                    "pan dy {dy}: mesh moved {d_mesh}, volume moved {d_vol} — opposite directions",
                );
            }
            prev_mesh_y = Some(mesh.y / mesh.w);
            prev_vol_y = Some(-(vol.y / vol.w));
        }
    }
}
