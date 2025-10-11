#![allow(dead_code)]

use std::sync::Arc;
use wgpu::{Device, Queue, RenderPipeline, BindGroupLayout, BindGroup, Buffer, BufferUsages};
use crate::rendering::content::render_content::RenderContent;

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
    // Camera parameters
    pub camera_pos: [f32; 3],
    pub _padding1: f32,
    pub camera_front: [f32; 3],
    pub _padding2: f32,
    pub camera_up: [f32; 3],
    pub _padding3: f32,
    pub camera_right: [f32; 3],
    pub _padding4: f32,
    
    // Volume parameters
    pub volume_size: [f32; 3],
    pub _padding5: f32,
    
    // Ray marching parameters
    pub ray_step_size: f32,
    pub max_steps: f32,
    
    // Texture format parameters
    pub is_packed_rg8: f32,
    pub _padding6: f32,
    
    // Window/Level for medical imaging
    pub window: f32,
    pub level: f32,
    
    // View matrix for coordinate transformation
    pub view_matrix: [[f32; 4]; 4],
    
    // Padding to ensure proper alignment (192 bytes total)
    pub _padding_end: [f32; 6],
}

impl Default for MipUniforms {
    fn default() -> Self {
        Self {
            camera_pos: [0.0, 0.0, -2.0],
            _padding1: 0.0,
            camera_front: [0.0, 0.0, 1.0],
            _padding2: 0.0,
            camera_up: [0.0, 1.0, 0.0],
            _padding3: 0.0,
            camera_right: [1.0, 0.0, 0.0],
            _padding4: 0.0,
            volume_size: [1.0, 1.0, 1.0],
            _padding5: 0.0,
            ray_step_size: 0.01,
            max_steps: 512.0,
            is_packed_rg8: 1.0,  // Default to packed format
            _padding6: 0.0,
            window: 1000.0,
            level: 500.0,
            view_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            _padding_end: [0.0; 6],
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
    pub pipeline: RenderPipeline,
    /// Uniform buffer for MIP parameters
    pub uniform_buffer: Buffer,
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

        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MIP Uniform Buffer"),
            size: std::mem::size_of::<MipUniforms>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Load MIP shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("MIP Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/mip.wgsl").into()),
        });

        // Create render pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("MIP Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout, &uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
        });

        Self {
            texture_bind_group_layout,
            uniform_bind_group_layout,
            pipeline,
            uniform_buffer,
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
    /// Binds the uniform buffer to the MIP pipeline.
    pub fn create_uniform_bind_group(&self, device: &Device) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MIP Uniform Bind Group"),
            layout: &self.uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// Function-level comment: Update the uniform buffer with new MIP parameters.
    /// Uploads the MipUniforms data to the GPU buffer.
    pub fn update_uniforms(&self, queue: &Queue, uniforms: &MipUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }
}



/// Function-level comment: MIP view that integrates with the existing RenderContent architecture.
/// Provides Maximum Intensity Projection rendering while reusing texture data from MPR views
/// for zero memory overhead and fast mode switching.
pub struct MipView {
    /// Shared render content from existing MPR views
    render_content: Arc<RenderContent>,
    /// MIP configuration settings
    config: MipConfig,
    /// Render context for GPU resources
    render_context: MipRenderContext,
    /// Pre-created texture bind group for rendering
    texture_bind_group: BindGroup,
    /// Pre-created uniform bind group for rendering
    uniform_bind_group: BindGroup,
    /// View position on screen
    position: (i32, i32),
    /// View dimensions
    dimensions: (u32, u32),
}

impl MipView {
    /// Function-level comment: Create a new MIP view using existing RenderContent.
    /// Accepts Arc<RenderContent> from MPR views to enable zero-copy texture sharing.
    pub fn new(render_content: Arc<RenderContent>, device: &Device, surface_format: wgpu::TextureFormat) -> Self {
        let render_context = MipRenderContext::new(device, surface_format);
        let texture_bind_group = render_context.create_texture_bind_group(device, &render_content);
        let uniform_bind_group = render_context.create_uniform_bind_group(device);
        
        Self {
            render_content,
            config: MipConfig::new(),
            render_context,
            texture_bind_group,
            uniform_bind_group,
            position: (0, 0),
            dimensions: (512, 512),
        }
    }

    /// Function-level comment: Get reference to the shared render content.
    pub fn render_content(&self) -> &Arc<RenderContent> {
        &self.render_content
    }

    /// Function-level comment: Get reference to the MIP configuration.
    pub fn config(&self) -> &MipConfig {
        &self.config
    }

    /// Function-level comment: Get mutable reference to the MIP configuration.
    pub fn config_mut(&mut self) -> &mut MipConfig {
        &mut self.config
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
        self.render_context.update_uniforms(queue, uniforms);
    }

    /// Function-level comment: Update MIP uniforms with camera and volume parameters for ray casting.
    /// Calculates camera vectors and volume parameters needed for MIP ray marching.
    pub fn update_camera_and_volume_uniforms(
        &self, 
        queue: &Queue, 
        camera: &crate::rendering::mesh::camera::Camera,
        volume_size: [f32; 3],
        window_level: f32,
        window_width: f32,
    ) {
        // Calculate camera vectors from view matrix
        let view_matrix = camera.view_matrix();
        
        // Extract camera vectors from view matrix
        // View matrix transforms world to camera space, so we need the inverse directions
        let camera_right = [view_matrix.data[0][0], view_matrix.data[1][0], view_matrix.data[2][0]];
        let camera_up = [view_matrix.data[0][1], view_matrix.data[1][1], view_matrix.data[2][1]];
        let camera_front = [-view_matrix.data[0][2], -view_matrix.data[1][2], -view_matrix.data[2][2]];
        
        // Create MIP uniforms with camera and volume parameters
        let uniforms = MipUniforms {
            camera_pos: camera.eye,
            _padding1: 0.0,
            camera_front,
            _padding2: 0.0,
            camera_up,
            _padding3: 0.0,
            camera_right,
            _padding4: 0.0,
            volume_size,
            _padding5: 0.0,
            ray_step_size: self.config.ray_step_size,
            max_steps: self.config.max_steps as f32,
            is_packed_rg8: 0.0, // Assume unpacked format for now
            _padding6: 0.0,
            window: window_width,
            level: window_level,
            view_matrix: view_matrix.data,
            _padding_end: [0.0;6],
        };
        
        self.update_uniforms(queue, &uniforms);
    }

    /// Function-level comment: Set the view position on screen.
    pub fn set_position(&mut self, position: (i32, i32)) {
        self.position = position;
    }

    /// Function-level comment: Get the current view position.
    pub fn position(&self) -> (i32, i32) {
        self.position
    }

    /// Function-level comment: Set the view dimensions.
    pub fn set_dimensions(&mut self, dimensions: (u32, u32)) {
        self.dimensions = dimensions;
    }

    /// Function-level comment: Get the current view dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        self.dimensions
    }
}

use std::any::Any;
use crate::rendering::view::{View, Renderable};

impl Renderable for MipView {
    /// Function-level comment: Update MIP view state and prepare for rendering.
    /// Currently minimal implementation for MVP.
    fn update(&mut self, _queue: &wgpu::Queue) {
        // MVP: Minimal update implementation
        // Future: Update uniforms, camera matrices, etc.
    }

    /// Function-level comment: Render MIP view using ray casting with the configured pipeline.
    /// Sets viewport, binds pipeline and resources, then draws a fullscreen quad for ray casting.
    fn render(
        &mut self,
        render_pass: &mut wgpu::RenderPass,
    ) -> Result<(), wgpu::SurfaceError> {
        // Set the MIP render pipeline
        render_pass.set_pipeline(&self.render_context.pipeline);

        // Set viewport for this view
        let (x, y) = (self.position.0 as f32, self.position.1 as f32);
        let (width, height) = (self.dimensions.0 as f32, self.dimensions.1 as f32);
        render_pass.set_viewport(x, y, width, height, 0.0, 1.0);

        // Bind pre-created texture bind group (volume texture and sampler)
        render_pass.set_bind_group(0, &self.texture_bind_group, &[]);

        // Bind pre-created uniform bind group (camera and volume parameters)
        render_pass.set_bind_group(1, &self.uniform_bind_group, &[]);

        // Draw fullscreen quad using triangle strip (4 vertices, no vertex buffer needed)
        // The vertex shader generates positions using vertex_index
        render_pass.draw(0..4, 0..1);

        log::debug!("[MIP_RENDER] Rendered MIP view at ({}, {}) with size {}x{}", 
                   self.position.0, self.position.1, self.dimensions.0, self.dimensions.1);

        Ok(())
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

    /// Function-level comment: MipView is not an MPR view, so return None.
    fn as_mpr(&mut self) -> Option<&mut dyn crate::rendering::view::MPRView> {
        None
    }
}