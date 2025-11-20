#![allow(dead_code)]

use std::{any::Any, sync::Arc};
use wgpu::{Device, Queue, RenderPipeline, BindGroupLayout, BindGroup, Buffer, BufferUsages};
use crate::rendering::view::render_content::RenderContent;
use crate::rendering::view::layout::compute_aspect_fit;
use crate::rendering::view::View;

/// Function-level comment: Configuration for Maximum Intensity Projection (MIP) rendering.
/// Provides fixed quality settings for the MVP implementation to minimize complexity
/// while delivering core MIP functionality.
#[derive(Debug, Clone)]
pub struct MipConfig {
    /// Ray step size for volume traversal (fixed for MVP)
    pub ray_step_size: f32,
    /// Maximum number of ray marching steps (fixed for MVP)
    pub max_steps: u32,
}

impl Default for MipConfig {
    /// Function-level comment: Create default MIP configuration with medium quality settings.
    /// These values provide a good balance between quality and performance for medical imaging.
    fn default() -> Self {
        Self {
            ray_step_size: 0.01,  // Fixed: 0.01 for medium quality
            max_steps: 512,       // Fixed: 512 steps for reasonable quality
        }
    }
}

/// MIP uniform data structure matching the shader
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MipUniforms {
    // Ray marching parameters
    pub ray_step_size: f32,
    pub max_steps: f32,
    
    // Texture format parameters (reused from existing logic)
    pub is_packed_rg8: f32,
    
    // Window/Level for medical imaging
    pub window: f32,
    pub level: f32,

    pub pan_x: f32,
    pub pan_y: f32,
    pub scale: f32,
}

impl Default for MipUniforms {
    fn default() -> Self {
        Self {
            ray_step_size: 0.01,
            max_steps: 512.0,
            is_packed_rg8: 1.0,  // Default to packed format
            window: 1000.0,
            level: 500.0,
            pan_x: 0.0,
            pan_y: 0.0,
            scale: 1.0,
        }
    }
}

impl MipConfig {
    /// Function-level comment: Create a new MIP configuration with default values.
    pub fn new() -> Self {
        Self::default()
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
    /// Function-level comment: Create a new MIP render context with initialized GPU resources.
    /// Sets up the render pipeline, bind group layouts, and uniform buffer for MIP rendering.
    pub fn new(device: &Device, surface_format: wgpu::TextureFormat) -> Self {
        // Create bind group layout for texture resources (group 0)
        // This layout is compatible with RenderContent texture binding
        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("MIP Uniform Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
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
        let pipeline = Arc::new(device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
        }));

        Self {
            texture_bind_group_layout,
            uniform_bind_group_layout,
            pipeline,
        }
    }

    /// Function-level comment: Create a bind group for the given RenderContent.
    /// Binds the texture and sampler from RenderContent to the MIP pipeline.
    pub fn create_texture_bind_group(&self, device: &Device, render_content: &RenderContent) -> BindGroup {
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

    /// Function-level comment: Create a bind group for MIP uniforms.
    /// Binds the provided uniform buffer to the MIP pipeline.
    pub fn create_uniform_bind_group(&self, device: &Device, uniform_buffer: &Buffer) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MIP Uniform Bind Group"),
            layout: &self.uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
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
    /// Function-level comment: Create a new MIP view using existing RenderContent.
    /// Accepts Arc<RenderContent> from MPR views to enable zero-copy texture sharing.
    pub fn new(render_content: Arc<RenderContent>, device: &Device, surface_format: wgpu::TextureFormat) -> Self {
        log::info!("[MIP_NEW] Creating MipView with surface format: {:?}", surface_format);
        log::info!("[MIP_NEW] RenderContent texture format: {:?}", render_content.texture_format);
        log::info!("[MIP_NEW] RenderContent texture size: {:?}", render_content.texture.size());
        
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
    
    /// Function-level comment: Get reference to the shared render content.
    pub fn render_content(&self) -> &Arc<RenderContent> {
        &self.render_content
    }

    /// Function-level comment: Get reference to the render context.
    pub fn render_context(&self) -> &MipRenderContext {
        &self.render_context
    }

    /// Function-level comment: Get mutable reference to the render context.
    pub fn render_context_mut(&mut self) -> &mut MipRenderContext {
        &mut self.render_context
    }

    /// Function-level comment: Get references to the pre-created bind groups.
    pub fn bind_groups(&self) -> (&BindGroup, &BindGroup) {
        (&self.texture_bind_group, &self.uniform_bind_group)
    }

    /// Function-level comment: Update the MIP uniforms for rendering.
    pub fn update_uniforms(&self, queue: &Queue, uniforms: &MipUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }
}



/// Function-level comment: MIP view that integrates with the existing RenderContent architecture.
/// Provides Maximum Intensity Projection rendering while reusing texture data from MPR views
/// for zero memory overhead and fast mode switching.
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
}

impl MipView {
    /// Function-level comment: Create a new MIP view with the given WGPU implementation.
    /// Initializes the view with default configuration and positioning.
    /// The WGPU implementation is wrapped in Arc for potential sharing between views.
    pub fn new(wgpu_impl: Arc<MipViewWgpuImpl>) -> Self {
        Self {
            wgpu_impl,
            config: MipConfig::default(),
            position: (0, 0),
            dimensions: (800, 600),
            scale: 1.0,
            pan: [0.0, 0.0, 0.0],
        }
    }

    /// Function-level comment: Get reference to the MIP configuration.
    pub fn config(&self) -> &MipConfig {
        &self.config
    }

    /// Function-level comment: Get mutable reference to the MIP configuration.
    pub fn config_mut(&mut self) -> &mut MipConfig {
        &mut self.config
    }

    /// Function-level comment: Update MIP view state and prepare for rendering.
    /// Currently minimal implementation for MVP.
    pub fn update(&mut self, queue: &wgpu::Queue) {
        log::trace!("[MIP_UPDATE] Starting MIP update");
        
        // Derive texture format flag for shader decoding
        let is_packed_rg8 = match self.wgpu_impl.render_content().texture_format {
            wgpu::TextureFormat::Rg8Unorm => 1.0,
            _ => 0.0,
        };

        // Fine-tune window/level for optimal contrast in medical data
        let (window, level) = if is_packed_rg8 > 0.5 {
            (4096.0, 2048.0)
        } else {
            // Narrower window for better contrast in R16Float medical data
            // (300.0, 150.0)
            (2000.0, 100.0)
        };

        // Create uniforms with only the fields used in the shader
        let uniforms = MipUniforms {
            ray_step_size: 0.005,  // Smaller step size for better quality
            max_steps: 1000.0,     // More steps to ensure we traverse the volume
            is_packed_rg8,
            window,
            level,
            pan_x: self.pan[0],
            pan_y: self.pan[1],
            scale: self.scale,
        };

        // Upload uniforms to GPU buffer
        self.wgpu_impl.update_uniforms(queue, &uniforms);

        log::trace!(
            "[MIP_UPDATE] Uniforms set: is_packed_rg8={}, window={}, level={}, step={}, max_steps={}",
            is_packed_rg8, window, level, uniforms.ray_step_size, uniforms.max_steps
        );
    }

    /// Function-level comment: Render MIP view using ray casting with the configured pipeline.
    /// Sets viewport, binds pipeline and resources, then draws a fullscreen quad for ray casting.
    pub fn render(
        &mut self,
        render_pass: &mut wgpu::RenderPass,
    ) -> Result<(), wgpu::SurfaceError> {
        log::trace!("[MIP_RENDER] Starting MIP render at ({}, {}) with size {}x{}",
                   self.position.0, self.position.1, self.dimensions.0, self.dimensions.1);
        
        // Set the MIP render pipeline
        render_pass.set_pipeline(&self.wgpu_impl.render_context().pipeline);

        // Set viewport for this view (aspect-preserving fit)
        let (x, y) = (self.position.0 as f32, self.position.1 as f32);
        let (w, h) = (self.dimensions.0, self.dimensions.1);
        let extent = self.wgpu_impl.render_content().texture.size();
        let cw = extent.width.max(1) as f32;
        let ch = extent.height.max(1) as f32;
        if let Some(fit) = compute_aspect_fit(w, h, cw, ch, 0) {
            render_pass.set_viewport(x + fit.x, y + fit.y, fit.w, fit.h, 0.0, 1.0);
        } else {
            render_pass.set_viewport(x, y, 1.0, 1.0, 0.0, 1.0);
        }

        // Bind pre-created texture bind group (volume texture and sampler)
        render_pass.set_bind_group(0, &*self.wgpu_impl.bind_groups().0, &[]);

        // Bind pre-created uniform bind group (camera and volume parameters)
        render_pass.set_bind_group(1, &*self.wgpu_impl.bind_groups().1, &[]);

        // Draw fullscreen quad using triangle strip (4 vertices, no vertex buffer needed)
        // The vertex shader generates positions using vertex_index
        render_pass.draw(0..4, 0..1);

        log::trace!("[MIP_RENDER] MIP render completed successfully");

        Ok(())
    }

    /// Function-level comment: Set scale factor.
    pub fn set_scale(&mut self, scale: f32) {
        // Clamp to a reasonable range to avoid clipping or degenerate matrices
        let clamped = scale.clamp(0.001, 100.0);
        self.scale = clamped;
        log::info!("MIP scale factor set to {:.3}", clamped);
    }

    /// Function-level comment: Set MIP pan translation (world units) for X and Y axes.
    /// Pan values are uploaded to the vertex shader as a uniform offset.
    pub fn set_pan(&mut self, dx: f32, dy: f32) {
        const MAX_PAN_DISTANCE: f32 = 10000.0; // Maximum pan distance in mm
        let clamped_x = dx.clamp(-MAX_PAN_DISTANCE, MAX_PAN_DISTANCE);
        let clamped_y = dy.clamp(-MAX_PAN_DISTANCE, MAX_PAN_DISTANCE);
        self.pan[0] = clamped_x;
        self.pan[1] = clamped_y;
        log::info!("MIP pan offset set to ({}, {})", clamped_x, clamped_y);
    }
}

impl crate::rendering::view::Renderable for MipView {
    /// Function-level comment: Bridge trait implementation to call inherent update method.
    fn update(&mut self, queue: &wgpu::Queue) {
        MipView::update(self, queue);
    }

    /// Function-level comment: Bridge trait implementation to call inherent render method.
    fn render(
        &mut self,
        render_pass: &mut wgpu::RenderPass,
    ) -> Result<(), wgpu::SurfaceError> {
        MipView::render(self, render_pass)
    }
}

impl View for MipView {
    /// Function-level comment: Get current view position on screen.
    fn position(&self) -> (i32, i32) {
        self.position
    }

    /// Function-level comment: Get current view dimensions.
    fn dimensions(&self) -> (u32, u32) {
        self.dimensions
    }

    /// Function-level comment: Move view to new screen position.
    fn move_to(&mut self, pos: (i32, i32)) {
        self.position = pos;
    }

    /// Function-level comment: Resize view to new dimensions.
    fn resize(&mut self, dim: (u32, u32)) {
        self.dimensions = dim;
    }

    /// Function-level comment: Get reference to this view as Any for type casting.
    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Function-level comment: Get mutable reference to this view as Any for type casting.
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    // Function-level comment: MipView is not an MPR view, so return None.
    // fn as_mpr(&mut self) -> Option<&mut dyn crate::rendering::view::MPRView> {
    //     None
    // }
}
