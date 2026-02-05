#![allow(dead_code)]

use crate::data::volume_encoding::VolumeEncoding;
use crate::rendering::view::layout::compute_aspect_fit;
use crate::rendering::view::render_content::RenderContent;
use crate::rendering::view::{Renderable, View};
use glam::{Mat4, Vec3};
use std::{any::Any, sync::Arc};
use wgpu::{BindGroup, BindGroupLayout, Buffer, BufferUsages, Device, Queue, RenderPipeline};

/// Configuration for Maximum Intensity Projection (MIP) rendering.
/// Provides fixed quality settings for the MVP implementation to minimize complexity
/// while delivering core MIP functionality.
#[derive(Debug, Clone)]
pub struct MipConfig {
    /// Slab thickness for MIP rendering in mm
    pub slab_thickness: f32,
    /// MIP rendering mode (0: MIP, 1: MinIP, 2: AvgIP)
    pub mip_mode: u32,
}

impl Default for MipConfig {
    /// Function-level comment: Create default MIP configuration with medium quality settings.
    /// These values provide a good balance between quality and performance for medical imaging.
    fn default() -> Self {
        Self {
            slab_thickness: 10.0, // Default 10mm slab
            mip_mode: 0,              // Default to Maximum Intensity Projection
        }
    }
}

impl MipConfig {
    /// Create a new MIP configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }
}

/// MIP uniform data structure matching the shader
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MipUniforms {
    // Ray marching parameters
    pub ray_step_size: f32,
    pub max_steps: f32,

    // Texture format parameters
    pub is_packed_rg8: f32,
    /// Bias used to decode packed RG8 back to raw HU
    pub bias: f32,

    // Window/Level for medical imaging
    pub window: f32,
    pub level: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub scale: f32,
    pub mip_mode: f32,
    pub lower_threshold: f32,
    pub upper_threshold: f32,
    pub rotation: [f32; 16],
}

impl Default for MipUniforms {
    fn default() -> Self {
        Self {
            ray_step_size: 0.01,
            max_steps: 512.0,
            is_packed_rg8: 1.0, // Default to packed format
            bias: VolumeEncoding::DEFAULT_HU_OFFSET,
            window: 1500.0,
            level: 400.0,
            pan_x: 0.0,
            pan_y: 0.0,
            scale: 1.0,
            mip_mode: 0.0,
            lower_threshold: -1024.0,
            upper_threshold: 3071.0,
            rotation: Mat4::IDENTITY.to_cols_array(),
        }
    }
}

/// GPU resources and state for MIP rendering
pub struct MipRenderContext {
    /// Bind group layout for texture resources (group 0)
    pub texture_bind_group_layout: BindGroupLayout,
    /// Bind group layout for uniforms (group 1)
    pub uniform_bind_group_layout: BindGroupLayout,
    /// Render pipeline for MIP rendering
    pub pipeline: Arc<RenderPipeline>,
}

impl MipRenderContext {
    /// Create a new MIP render context with initialized GPU resources.
    pub fn new(device: &Device, surface_format: wgpu::TextureFormat) -> Self {
        // Create bind group layout for texture resources (group 0)
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MIP Texture Bind Group Layout"),
                entries: &[
                    // Texture binding
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D3,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    // Sampler binding
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // Create bind group layout for uniforms (group 1)
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MIP Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<MipUniforms>() as u64,
                        ),
                    },
                    count: None,
                }],
            });

        // Load MIP shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("MIP Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/mip.wgsl").into()),
        });

        // Create render pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("MIP Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout, &uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = Arc::new(
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("MIP Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            }),
        );

        Self {
            texture_bind_group_layout,
            uniform_bind_group_layout,
            pipeline,
        }
    }

    /// Create a bind group for the given RenderContent.
    pub fn create_texture_bind_group(
        &self,
        device: &Device,
        render_content: &RenderContent,
    ) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MIP Texture Bind Group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&render_content.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&render_content.sampler),
                },
            ],
        })
    }

    /// Create a bind group for MIP uniforms.
    pub fn create_uniform_bind_group(&self, device: &Device, uniform_buffer: &Buffer) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MIP Uniform Bind Group"),
            layout: &self.uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        })
    }
}

pub struct MipViewWgpuImpl {
    /// Shared render content from existing MPR views
    render_content: Arc<RenderContent>,
    /// Render context for GPU resources
    render_context: MipRenderContext,
    /// Pre-created texture bind group for rendering
    texture_bind_group: BindGroup,
    /// Pre-created uniform bind group for rendering
    uniform_bind_group: BindGroup,
    /// Uniform buffer for MIP parameters
    uniform_buffer: Buffer,
}

impl MipViewWgpuImpl {
    /// Create a new MIP view using existing RenderContent.
    pub fn new(
        render_content: Arc<RenderContent>,
        device: &Device,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let render_context = MipRenderContext::new(device, surface_format);
        let texture_bind_group = render_context.create_texture_bind_group(device, &render_content);

        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MIP Uniform Buffer"),
            size: std::mem::size_of::<MipUniforms>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_bind_group = render_context.create_uniform_bind_group(device, &uniform_buffer);
        Self {
            render_content,
            render_context,
            texture_bind_group,
            uniform_bind_group,
            uniform_buffer,
        }
    }

    pub fn render_content(&self) -> &Arc<RenderContent> {
        &self.render_content
    }

    pub fn render_context(&self) -> &MipRenderContext {
        &self.render_context
    }

    pub fn bind_groups(&self) -> (&BindGroup, &BindGroup) {
        (&self.texture_bind_group, &self.uniform_bind_group)
    }

    pub fn update_uniforms(&self, queue: &Queue, uniforms: &MipUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }
}

/// MIP view that integrates with the existing RenderContent architecture.
pub struct MipView {
    /// WGPU implementation details
    wgpu_impl: Arc<MipViewWgpuImpl>,
    /// MIP configuration settings
    config: MipConfig,
    /// View position on screen
    position: (i32, i32),
    /// View dimensions
    dimensions: (u32, u32),
    /// scale factor
    scale: f32,
    /// Pan translation in screen coordinates
    pan: [f32; 3],
    /// Rotation angles in radians around X, Y, Z axes
    rotation_radians: [f32; 3],
    /// Content dimensions in world space
    content_dimensions: (f32, f32),
}

impl MipView {
    /// Create a new MIP view with the given WGPU implementation.
    pub fn new(wgpu_impl: Arc<MipViewWgpuImpl>) -> Self {
        Self {
            wgpu_impl,
            config: MipConfig::default(),
            position: (0, 0),
            dimensions: (800, 600),
            scale: 1.0,
            pan: [0.0, 0.0, 0.0],
            rotation_radians: [0.0, 0.0, 0.0],
            content_dimensions: (1.0, 1.0),
        }
    }

    pub fn config(&self) -> &MipConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut MipConfig {
        &mut self.config
    }

    pub fn build_rotation_matrix(roll: f32, yaw: f32, pitch: f32) -> Mat4 {
        Mat4::from_rotation_z(roll) * Mat4::from_rotation_y(yaw) * Mat4::from_rotation_x(pitch)
    }

    /// Helper to get render parameters (W/L, Thresholds) based on mode.
    /// Returns (window, level, lower_threshold, upper_threshold).
    fn get_render_params(mip_mode: u32) -> (f32, f32, f32, f32) {
        match mip_mode {
            1 => {
                // MinIP: Lung Window, full range to include air
                let (w, l) = crate::core::window_level::WindowLevel::DEFAULT_LUNG;
                (w, l, -1024.0, 300.0)
            }
            2 => {
                // AvgIP: Soft Tissue Window, full range
                let (w, l) = crate::core::window_level::WindowLevel::DEFAULT_SOFT_TISSUE;
                (w, l, -200.0, 300.0)
            }
            _ => {
                // MIP: Bone Window, full range
                let (w, l) = crate::core::window_level::WindowLevel::DEFAULT_BONE;
                (w, l, -1024.0, 3071.0)
            }
        }
    }

    /// Helper to get quality parameters (step size, max steps) based on mode.
    /// Returns (ray_step_size, max_steps).
    fn get_quality_params(mip_mode: u32) -> (f32, f32) {
        match mip_mode {
            2 => (0.003, 2000.0), // AvgIP 高质量采样
            1 => (0.004, 1500.0), // MinIP 中等采样
            _ => (0.005, 1000.0), // MIP 默认
        }
    }

    /// Set scale factor.
    pub fn set_scale(&mut self, scale: f32) {
        let clamped = scale.clamp(0.001, 100.0);
        self.scale = clamped;
        log::info!("MIP scale factor set to {:.3}", clamped);
    }

    /// Set MIP pan translation (world units) for X and Y axes.
    pub fn set_pan(&mut self, dx: f32, dy: f32) {
        const MAX_PAN_DISTANCE: f32 = 10000.0;
        let clamped_x = dx.clamp(-MAX_PAN_DISTANCE, MAX_PAN_DISTANCE);
        let clamped_y = dy.clamp(-MAX_PAN_DISTANCE, MAX_PAN_DISTANCE);
        self.pan = [clamped_x, clamped_y, 0.0];
    }

    /// Set MIP rendering mode.
    pub fn set_mip_mode(&mut self, mip_mode: u32) {
        self.config.mip_mode = mip_mode;
        log::info!("MIP mode set to {}", mip_mode);
    }

    /// Set MIP slab thickness in mm.
    pub fn set_slab_thickness(&mut self, thickness: f32) {
        self.config.slab_thickness = thickness;
        log::info!("MIP slab thickness set to {:.3} mm", thickness);
    }

    /// Set MIP rotation angles in degrees around X, Y, Z axes.
    pub fn set_rotation_degrees(&mut self, roll_deg: f32, yaw_deg: f32, pitch_deg: f32) {
        let (roll, yaw, pitch) = (
            roll_deg.to_radians(),
            yaw_deg.to_radians(),
            pitch_deg.to_radians(),
        );
        self.rotation_radians = [roll, yaw, pitch];
    }

    /// Set MIP rotation angles in radians around X, Y, Z axes.
    pub fn set_rotation_radians(&mut self, roll: f32, yaw: f32, pitch: f32) {
        self.rotation_radians = [roll, yaw, pitch];
    }

    pub fn get_scale(&self) -> f32 {
        self.scale
    }

    pub fn get_pan(&self) -> [f32; 3] {
        self.pan
    }

    pub fn get_rotation_radians(&self) -> [f32; 3] {
        self.rotation_radians
    }
}

impl Renderable for MipView {
    fn update(&mut self, queue: &Queue) {
        log::trace!("[MIP_UPDATE] Starting MIP update");

        // Derive texture format flag for shader decoding
        let decode_params = self.wgpu_impl.render_content().decode_parameters();

        // Automatically determine Window/Level and Thresholds based on mode
        let (window, level, lower_threshold, upper_threshold) = Self::get_render_params(self.config.mip_mode);
        
        // Determine quality settings
        let (ray_step_size, max_steps) = Self::get_quality_params(self.config.mip_mode);

        // Create uniforms
        let rotation = Self::build_rotation_matrix(
            self.rotation_radians[0],
            self.rotation_radians[1],
            self.rotation_radians[2],
        );

        let extent = self.wgpu_impl.render_content().texture.size();
        let w_vol = extent.width.max(1) as f32;
        let h_vol = extent.height.max(1) as f32;
        let d_vol = extent.depth_or_array_layers.max(1) as f32;

        // Calculate Projected Bounding Box
        let sz = self.config.slab_thickness;
        let w_mm = w_vol * 1.0;
        let h_mm = h_vol * 1.0;
        let d_mm = d_vol * sz;
        let diag = (w_mm * w_mm + h_mm * h_mm + d_mm * d_mm).sqrt();
        let cw = diag;
        let ch = diag;
        self.content_dimensions = (cw, ch);

        // Construct Composite Matrix for Shader
        let scale_viewport = Mat4::from_scale(Vec3::new(cw, ch, cw));
        let scale_texture = Mat4::from_scale(Vec3::new(1.0 / w_mm, 1.0 / h_mm, 1.0 / d_mm));
        let final_matrix = scale_texture * rotation * scale_viewport;

        let uniforms = MipUniforms {
            ray_step_size,
            max_steps,
            is_packed_rg8: decode_params.is_packed_flag as f32,
            bias: decode_params.bias,
            window,
            level,
            pan_x: self.pan[0],
            pan_y: self.pan[1],
            scale: self.scale,
            mip_mode: self.config.mip_mode as f32,
            lower_threshold,
            upper_threshold,
            rotation: final_matrix.to_cols_array(),
        };

        // Upload uniforms to GPU buffer
        self.wgpu_impl.update_uniforms(queue, &uniforms);

        log::trace!(
            "[MIP_UPDATE] Uniforms set: mip_mode={}, window={}, level={}, lower={}, upper={}",
            uniforms.mip_mode, window, level, lower_threshold, upper_threshold
        );
    }

    fn render(&mut self, render_pass: &mut wgpu::RenderPass) -> Result<(), wgpu::SurfaceError> {
        log::trace!("[MIP_RENDER] Starting MIP render");

        // Set the MIP render pipeline
        render_pass.set_pipeline(&self.wgpu_impl.render_context().pipeline);

        // Set viewport for this view
        let (x, y) = (self.position.0 as f32, self.position.1 as f32);
        let (w, h) = (self.dimensions.0, self.dimensions.1);

        // Calculate projected volume dimensions
        let (cw, ch) = self.content_dimensions;

        // Compute aspect fit
        if let Some(fit) = compute_aspect_fit(w, h, cw, ch, 0) {
            render_pass.set_viewport(x + fit.x, y + fit.y, fit.w, fit.h, 0.0, 1.0);
        } else {
            render_pass.set_viewport(x, y, 1.0, 1.0, 0.0, 1.0);
        }

        // Bind resources
        render_pass.set_bind_group(0, &*self.wgpu_impl.bind_groups().0, &[]);
        render_pass.set_bind_group(1, &*self.wgpu_impl.bind_groups().1, &[]);

        // Draw fullscreen quad
        render_pass.draw(0..4, 0..1);

        log::trace!("[MIP_RENDER] MIP render completed");
        Ok(())
    }
}

impl View for MipView {
    fn position(&self) -> (i32, i32) {
        self.position
    }

    fn dimensions(&self) -> (u32, u32) {
        self.dimensions
    }

    fn move_to(&mut self, pos: (i32, i32)) {
        self.position = pos;
    }

    fn resize(&mut self, dim: (u32, u32)) {
        self.dimensions = dim;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    #[test]
    fn test_mip_uniforms_size() {
        let size = std::mem::size_of::<MipUniforms>();
        assert_eq!(size % 16, 0);
        assert_eq!(size, 112);
    }

    #[test]
    fn test_build_rotation_matrix_identity() {
        let m = MipView::build_rotation_matrix(0.0, 0.0, 0.0);
        assert_eq!(m, Mat4::IDENTITY);
    }

    #[test]
    fn test_build_rotation_matrix_roll_90() {
        let m = MipView::build_rotation_matrix(FRAC_PI_2, 0.0, 0.0);
        let v = (m * glam::Vec4::new(1.0, 0.0, 0.0, 0.0)).truncate();
        assert!((v.x - 0.0).abs() < 1e-5);
        assert!((v.y - 1.0).abs() < 1e-5);
        assert!(v.z.abs() < 1e-5);
    }

    #[test]
    fn test_mip_config_creation() {
        let config = MipConfig::new();
        // Just verify it can be created and has expected default mode
        assert_eq!(config.mip_mode, 0);
        assert!(config.slab_thickness > 0.0);
    }
}
