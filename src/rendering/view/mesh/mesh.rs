#![allow(dead_code)]

use crate::data::{volume_encoding::VolumeEncoding, CTVolume};
use crate::rendering::view::render_content::RenderContent;
use std::sync::Arc;
use wgpu::{BindGroup, BindGroupLayout, Buffer, BufferUsages, Device, RenderPipeline};
use glam::Mat4;

/// Volume rendering parameters (sent to fragment shader)
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshUniforms {
    pub ray_step_size: f32,
    pub max_steps: f32,
    pub is_packed_rg8: f32,
    pub bias: f32,
    pub window: f32,
    pub level: f32,
    pub _pad0: [f32; 2],
    pub pan_x: f32,
    pub pan_y: f32,
    pub scale: f32,
    pub opacity_multiplier: f32,
    pub light_dir: [f32; 3],
    pub shading_strength: f32,
    pub rotation: [f32; 16],
}

impl Default for MeshUniforms {
    fn default() -> Self {
        Self {
            ray_step_size: 0.01,
            max_steps: 512.0,
            is_packed_rg8: 1.0,
            bias: VolumeEncoding::DEFAULT_HU_OFFSET,
            window: 1500.0,
            level: 400.0,
            _pad0:[0.0, 0.0],
            pan_x: 0.0,
            pan_y: 0.0,
            scale: 1.0,
            opacity_multiplier: 0.05,
            light_dir: [0.5, 0.5, -1.0],
            shading_strength: 0.0,
            rotation: Mat4::IDENTITY.to_cols_array(),
        }
    }
}

/// GPU resources and state for Mesh rendering
pub struct MeshRenderContext {
    pub texture_bind_group_layout: BindGroupLayout,
    pub uniform_bind_group_layout: BindGroupLayout,
    pub pipeline: Arc<RenderPipeline>,
    pub uniform_buffer: Buffer,
    pub uniform_bind_group: BindGroup,
    pub texture_bind_group: BindGroup,
    pub render_content: Arc<RenderContent>,
}

impl MeshRenderContext {
    pub fn new(
        device: &Device,
        surface_format: wgpu::TextureFormat,
        render_content: Arc<RenderContent>,
    ) -> Self {
        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Mesh Volume Texture Bind Group Layout"),
            entries: &[
                // 3D Texture
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

        // Uniform buffer
        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Mesh Volume Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<
                        MeshUniforms,
                    >()
                        as u64),
                },
                count: None,
            }],
        });

        // Load Mesh shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Mesh Volume Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../../rendering/shaders/mesh_volume.wgsl").into(),
            ),
        });

        // Create render pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mesh Volume Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout, &uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = Arc::new(
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Mesh Volume Render Pipeline"),
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
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING), // Important for overlaying
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
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24Plus,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Always,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            }),
        );

        // GPU Buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Mesh Volume Uniform Buffer"),
            size: std::mem::size_of::<MeshUniforms>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Mesh Volume Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Texture Bind Group
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Mesh Volume Texture Bind Group"),
            layout: &texture_bind_group_layout,
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
        });

        Self {
            texture_bind_group_layout,
            uniform_bind_group_layout,
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            texture_bind_group,
            render_content,
        }
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
        render_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
        render_pass.draw(0..4, 0..1); // fullscreen quad
    }

    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &MeshUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }

    pub fn get_memory_stats(&self) -> (u64, u64, f32, f32) {
        self.render_content.get_memory_stats()
    }
}

/// Function-level comment: Minimal lighting uniform structure for basic mesh lighting MVP.
/// Supports a single directional light with ambient lighting for simple 3D illumination.
/// Uses proper 16-byte alignment for WGSL uniform buffers.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BasicLightingUniforms {
    pub light_direction: [f32; 3],
    pub _padding1: f32,
    pub light_color: [f32; 3],
    pub light_intensity: f32,
    pub ambient_color: [f32; 3],
    pub ambient_intensity: f32,
    pub padding2: [f32; 3],
    pub opacity: f32,
}

impl Default for BasicLightingUniforms {
    /// Function-level comment: Creates default lighting configuration for basic mesh rendering.
    /// Provides reasonable defaults for directional lighting from top-left-front direction.
    fn default() -> Self {
        Self {
            light_direction: [0.6, -0.7, 0.3], // Top-left-front direction
            _padding1: 0.0,
            light_color: [1.0, 1.0, 1.0], // White light
            light_intensity: 1.0,
            ambient_color: [0.4, 0.4, 0.4],
            ambient_intensity: 0.5,
            padding2: [0.0, 0.0, 0.0],
            opacity: 1.0,
        }
    }
}

/// Function-level comment: High-level lighting configuration for mesh rendering.
/// Provides a simple interface for setting light direction and intensity.
#[derive(Debug, Clone)]
pub struct Lighting {
    pub direction: [f32; 3],
    pub light_color: [f32; 3],
    pub light_intensity: f32,
    pub ambient_color: [f32; 3],
    pub ambient_intensity: f32,
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            direction: [0.6, -0.7, 0.3],
            light_color: [1.0, 1.0, 1.0],
            light_intensity: 1.0,
            ambient_color: [0.4, 0.4, 0.5],
            ambient_intensity: 0.4,
        }
    }
}

impl Lighting {
    /// Function-level comment: Convert high-level Lighting to BasicLightingUniforms for GPU upload.
    /// Maps the simple direction/intensity to the full uniform structure.
    pub fn to_basic_uniforms(&self) -> BasicLightingUniforms {
        BasicLightingUniforms {
            light_direction: self.direction,
            _padding1: 0.0,
            light_color: self.light_color,
            light_intensity: self.light_intensity,
            ambient_color: self.ambient_color,
            ambient_intensity: self.ambient_intensity,
            padding2: [0.0, 0.0, 0.0],
            opacity: 1.0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
}

// Manual implementation to avoid potential bytemuck derive issues
unsafe impl bytemuck::Zeroable for MeshVertex {}
unsafe impl bytemuck::Pod for MeshVertex {}

#[derive(Default, Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    /// Function-level comment: Creates a unit cube mesh with different colors for each face
    /// Returns a cube with 24 vertices (4 per face) and 12 triangles for colorful 3D rendering
    pub fn unit_cube() -> Self {
        // Define distinct colors for each face
        let front_color = [0.0, 1.0, 0.0]; // Green
        let back_color = [0.0, 1.0, 0.0]; // Green
        let bottom_color = [1.0, 0.0, 0.0]; // Red
        let top_color = [1.0, 0.0, 0.0]; // Red
        let left_color = [0.0, 0.0, 1.0]; // Blue
        let right_color = [0.0, 0.0, 1.0]; // Blue

        // Define face normals for unit cube
        let front_normal = [0.0, 0.0, 1.0]; // +Z
        let back_normal = [0.0, 0.0, -1.0]; // -Z
        let bottom_normal = [0.0, -1.0, 0.0]; // -Y
        let top_normal = [0.0, 1.0, 0.0]; // +Y
        let left_normal = [-1.0, 0.0, 0.0]; // -X
        let right_normal = [1.0, 0.0, 0.0]; // +X

        let vertices = vec![
            // Front face (Red) - vertices 0-3
            MeshVertex {
                position: [-1.0, -1.0, 1.0],
                normal: front_normal,
                color: front_color,
            }, // 0
            MeshVertex {
                position: [1.0, -1.0, 1.0],
                normal: front_normal,
                color: front_color,
            }, // 1
            MeshVertex {
                position: [1.0, 1.0, 1.0],
                normal: front_normal,
                color: front_color,
            }, // 2
            MeshVertex {
                position: [-1.0, 1.0, 1.0],
                normal: front_normal,
                color: front_color,
            }, // 3
            // Back face (Green) - vertices 4-7
            MeshVertex {
                position: [1.0, -1.0, -1.0],
                normal: back_normal,
                color: back_color,
            }, // 4
            MeshVertex {
                position: [-1.0, -1.0, -1.0],
                normal: back_normal,
                color: back_color,
            }, // 5
            MeshVertex {
                position: [-1.0, 1.0, -1.0],
                normal: back_normal,
                color: back_color,
            }, // 6
            MeshVertex {
                position: [1.0, 1.0, -1.0],
                normal: back_normal,
                color: back_color,
            }, // 7
            // Bottom face (Blue) - vertices 8-11
            MeshVertex {
                position: [-1.0, -1.0, -1.0],
                normal: bottom_normal,
                color: bottom_color,
            }, // 8
            MeshVertex {
                position: [1.0, -1.0, -1.0],
                normal: bottom_normal,
                color: bottom_color,
            }, // 9
            MeshVertex {
                position: [1.0, -1.0, 1.0],
                normal: bottom_normal,
                color: bottom_color,
            }, // 10
            MeshVertex {
                position: [-1.0, -1.0, 1.0],
                normal: bottom_normal,
                color: bottom_color,
            }, // 11
            // Top face (Yellow) - vertices 12-15
            MeshVertex {
                position: [-1.0, 1.0, 1.0],
                normal: top_normal,
                color: top_color,
            }, // 12
            MeshVertex {
                position: [1.0, 1.0, 1.0],
                normal: top_normal,
                color: top_color,
            }, // 13
            MeshVertex {
                position: [1.0, 1.0, -1.0],
                normal: top_normal,
                color: top_color,
            }, // 14
            MeshVertex {
                position: [-1.0, 1.0, -1.0],
                normal: top_normal,
                color: top_color,
            }, // 15
            // Left face (Magenta) - vertices 16-19
            MeshVertex {
                position: [-1.0, -1.0, -1.0],
                normal: left_normal,
                color: left_color,
            }, // 16
            MeshVertex {
                position: [-1.0, -1.0, 1.0],
                normal: left_normal,
                color: left_color,
            }, // 17
            MeshVertex {
                position: [-1.0, 1.0, 1.0],
                normal: left_normal,
                color: left_color,
            }, // 18
            MeshVertex {
                position: [-1.0, 1.0, -1.0],
                normal: left_normal,
                color: left_color,
            }, // 19
            // Right face (Cyan) - vertices 20-23
            MeshVertex {
                position: [1.0, -1.0, 1.0],
                normal: right_normal,
                color: right_color,
            }, // 20
            MeshVertex {
                position: [1.0, -1.0, -1.0],
                normal: right_normal,
                color: right_color,
            }, // 21
            MeshVertex {
                position: [1.0, 1.0, -1.0],
                normal: right_normal,
                color: right_color,
            }, // 22
            MeshVertex {
                position: [1.0, 1.0, 1.0],
                normal: right_normal,
                color: right_color,
            }, // 23
        ];

        // Create cube indices for each colored face (CCW winding)
        let indices: Vec<u32> = vec![
            // Front face (Red)
            0, 1, 2, 2, 3, 0, // Back face (Green)
            4, 5, 6, 6, 7, 4, // Bottom face (Blue)
            8, 9, 10, 10, 11, 8, // Top face (Yellow)
            12, 13, 14, 14, 15, 12, // Left face (Magenta)
            16, 17, 18, 18, 19, 16, // Right face (Cyan)
            20, 21, 22, 22, 23, 20,
        ];

        Self { vertices, indices }
    }

    /// Function-level comment: Creates a simplified spine vertebra mesh for medical imaging visualization
    /// Represents a thoracic vertebra with body, arch, and processes for anatomical reference
    pub fn spine_vertebra() -> Self {
        // Vertebra bone color (light beige/cream)
        let bone_color = [0.9, 0.85, 0.75];

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut vertex_index = 0u16;

        // Helper function to add a box with proper normals
        let mut add_box = |center: [f32; 3], size: [f32; 3], color: [f32; 3]| {
            let half_size = [size[0] * 0.5, size[1] * 0.5, size[2] * 0.5];

            // Define face normals
            let front_normal = [0.0, 0.0, 1.0];
            let back_normal = [0.0, 0.0, -1.0];
            let bottom_normal = [0.0, -1.0, 0.0];
            let top_normal = [0.0, 1.0, 0.0];
            let left_normal = [-1.0, 0.0, 0.0];
            let right_normal = [1.0, 0.0, 0.0];

            // Define vertices relative to center
            let base_index = vertex_index;

            // Front face vertices
            vertices.push(MeshVertex {
                position: [
                    center[0] - half_size[0],
                    center[1] - half_size[1],
                    center[2] + half_size[2],
                ],
                normal: front_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [
                    center[0] + half_size[0],
                    center[1] - half_size[1],
                    center[2] + half_size[2],
                ],
                normal: front_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [
                    center[0] + half_size[0],
                    center[1] + half_size[1],
                    center[2] + half_size[2],
                ],
                normal: front_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [
                    center[0] - half_size[0],
                    center[1] + half_size[1],
                    center[2] + half_size[2],
                ],
                normal: front_normal,
                color,
            });

            // Back face vertices
            vertices.push(MeshVertex {
                position: [
                    center[0] + half_size[0],
                    center[1] - half_size[1],
                    center[2] - half_size[2],
                ],
                normal: back_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [
                    center[0] - half_size[0],
                    center[1] - half_size[1],
                    center[2] - half_size[2],
                ],
                normal: back_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [
                    center[0] - half_size[0],
                    center[1] + half_size[1],
                    center[2] - half_size[2],
                ],
                normal: back_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [
                    center[0] + half_size[0],
                    center[1] + half_size[1],
                    center[2] - half_size[2],
                ],
                normal: back_normal,
                color,
            });

            // Bottom face vertices
            vertices.push(MeshVertex {
                position: [
                    center[0] - half_size[0],
                    center[1] - half_size[1],
                    center[2] - half_size[2],
                ],
                normal: bottom_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [
                    center[0] + half_size[0],
                    center[1] - half_size[1],
                    center[2] - half_size[2],
                ],
                normal: bottom_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [
                    center[0] + half_size[0],
                    center[1] - half_size[1],
                    center[2] + half_size[2],
                ],
                normal: bottom_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [
                    center[0] - half_size[0],
                    center[1] - half_size[1],
                    center[2] + half_size[2],
                ],
                normal: bottom_normal,
                color,
            });

            // Top face vertices
            vertices.push(MeshVertex {
                position: [
                    center[0] - half_size[0],
                    center[1] + half_size[1],
                    center[2] + half_size[2],
                ],
                normal: top_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [
                    center[0] + half_size[0],
                    center[1] + half_size[1],
                    center[2] + half_size[2],
                ],
                normal: top_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [
                    center[0] + half_size[0],
                    center[1] + half_size[1],
                    center[2] - half_size[2],
                ],
                normal: top_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [
                    center[0] - half_size[0],
                    center[1] + half_size[1],
                    center[2] - half_size[2],
                ],
                normal: top_normal,
                color,
            });

            // Left face vertices
            vertices.push(MeshVertex {
                position: [
                    center[0] - half_size[0],
                    center[1] - half_size[1],
                    center[2] - half_size[2],
                ],
                normal: left_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [
                    center[0] - half_size[0],
                    center[1] - half_size[1],
                    center[2] + half_size[2],
                ],
                normal: left_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [
                    center[0] - half_size[0],
                    center[1] + half_size[1],
                    center[2] + half_size[2],
                ],
                normal: left_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [
                    center[0] - half_size[0],
                    center[1] + half_size[1],
                    center[2] - half_size[2],
                ],
                normal: left_normal,
                color,
            });

            // Right face vertices
            vertices.push(MeshVertex {
                position: [
                    center[0] + half_size[0],
                    center[1] - half_size[1],
                    center[2] + half_size[2],
                ],
                normal: right_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [
                    center[0] + half_size[0],
                    center[1] - half_size[1],
                    center[2] - half_size[2],
                ],
                normal: right_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [
                    center[0] + half_size[0],
                    center[1] + half_size[1],
                    center[2] - half_size[2],
                ],
                normal: right_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [
                    center[0] + half_size[0],
                    center[1] + half_size[1],
                    center[2] + half_size[2],
                ],
                normal: right_normal,
                color,
            });

            // Add indices for each face
            indices.extend_from_slice(&[
                // Front face
                (base_index + 0) as u32,
                (base_index + 1) as u32,
                (base_index + 2) as u32,
                (base_index + 2) as u32,
                (base_index + 3) as u32,
                (base_index + 0) as u32,
                // Back face
                (base_index + 4) as u32,
                (base_index + 5) as u32,
                (base_index + 6) as u32,
                (base_index + 6) as u32,
                (base_index + 7) as u32,
                (base_index + 4) as u32,
                // Bottom face
                (base_index + 8) as u32,
                (base_index + 9) as u32,
                (base_index + 10) as u32,
                (base_index + 10) as u32,
                (base_index + 11) as u32,
                (base_index + 8) as u32,
                // Top face
                (base_index + 12) as u32,
                (base_index + 13) as u32,
                (base_index + 14) as u32,
                (base_index + 14) as u32,
                (base_index + 15) as u32,
                (base_index + 12) as u32,
                // Left face
                (base_index + 16) as u32,
                (base_index + 17) as u32,
                (base_index + 18) as u32,
                (base_index + 18) as u32,
                (base_index + 19) as u32,
                (base_index + 16) as u32,
                // Right face
                (base_index + 20) as u32,
                (base_index + 21) as u32,
                (base_index + 22) as u32,
                (base_index + 22) as u32,
                (base_index + 23) as u32,
                (base_index + 20) as u32,
            ]);

            vertex_index += 24;
        };

        // Vertebra body (main cylindrical part)
        add_box([0.0, 0.0, 0.0], [1.2, 0.8, 1.0], bone_color);

        // Vertebral arch (posterior ring structure)
        add_box([0.0, 0.0, -0.8], [1.0, 0.6, 0.4], bone_color);

        // Spinous process (posterior projection)
        add_box([0.0, 0.0, -1.4], [0.4, 1.2, 0.3], bone_color);

        // Transverse processes (lateral projections)
        add_box([-0.9, 0.0, -0.4], [0.3, 0.4, 0.8], bone_color);
        add_box([0.9, 0.0, -0.4], [0.3, 0.4, 0.8], bone_color);

        // Superior articular processes
        add_box([-0.5, 0.5, 0.2], [0.2, 0.3, 0.4], bone_color);
        add_box([0.5, 0.5, 0.2], [0.2, 0.3, 0.4], bone_color);

        // Inferior articular processes
        add_box([-0.5, -0.5, -0.8], [0.2, 0.3, 0.4], bone_color);
        add_box([0.5, -0.5, -0.8], [0.2, 0.3, 0.4], bone_color);

        Self { vertices, indices }
    }

    pub fn new(
        _ctvolume: &CTVolume,
        _iso_min: f32,
        _iso_max: f32,
        _world_min: Option<[f32; 3]>,
        _world_max: Option<[f32; 3]>,
    ) -> Self {
        log::info!("Surface mesh extraction is disabled. Using volume rendering.");
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

}

impl MeshVertex {
    /// Function-level comment: Vertex attribute array for position, normal, and color
    /// Defines position, normal, and color attributes for the vertex shader
    const ATTRS: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3];

    /// Function-level comment: Creates vertex buffer layout descriptor for lighting-enabled mesh
    /// Returns layout for position, normal, and color vertex attributes
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MeshVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRS,
        }
    }
}
