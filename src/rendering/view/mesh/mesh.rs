#![allow(dead_code)]

use crate::data::volume_encoding::VolumeEncoding;
use crate::rendering::view::render_content::RenderContent;
use crate::rendering::core::pipeline::*;
use std::sync::Arc;
use glam::Mat4;
use wgpu::{BindGroup, BindGroupLayout, Buffer, BufferUsages, Device, RenderPipeline};

/// Volume rendering parameters (sent to fragment shader)
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshUniforms {
    pub ray_step_size: f32,
    pub max_steps: f32,
    pub is_packed_rg8: f32,
    pub bias: f32,
    pub window: f32,
    pub level: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub roi_min: [f32; 3],
    pub scale: f32,
    pub roi_max: [f32; 3],
    pub opacity_multiplier: f32,
    pub light_dir: [f32; 3],
    pub aspect_ratio: f32,
    pub rotation: [f32; 16],
    pub vol_dims: [f32; 3],
    pub preset: f32,
    pub needle_entry : [f32; 3],
    pub needle_enabled : f32,
    pub needle_tip : [f32; 3],
    pub needle_radius : f32,
}

impl Default for MeshUniforms {
    fn default() -> Self {
        Self {
            ray_step_size: 0.0004,
            max_steps: 1500.0,
            is_packed_rg8: 1.0,
            bias: VolumeEncoding::DEFAULT_HU_OFFSET,
            window: 1500.0,
            level: 400.0,
            pan_x: 0.0,
            pan_y: 0.0,
            roi_min: [0.0, 0.0, 0.0],
            scale: 1.0,
            roi_max: [1.0, 1.0, 1.0],
            opacity_multiplier: 1.0,
            light_dir: [0.5, 0.5, -1.0],
            aspect_ratio: 1.0,
            rotation: Mat4::IDENTITY.to_cols_array(),
            vol_dims: [512.0, 512.0, 300.0],
            preset: 1.0,
            needle_entry: [0.5, 0.0, 0.5],
            needle_enabled: 0.0,
            needle_tip: [0.0, 0.0, 0.0],
            needle_radius: 0.5,
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
        target_format: wgpu::TextureFormat,
        render_content: Arc<RenderContent>,
    ) -> Self {
        let texture_bind_group_layout = create_texture_bind_group_layout(device);

        // Uniform buffer
        let min_binding_size = std::num::NonZeroU64::new(std::mem::size_of::<MeshUniforms>() as u64);
        let uniform_bind_group_layout = create_uniform_bind_group_layout(device, min_binding_size);

        // Create render pipeline
        let pipeline = Arc::new(create_volume_pipeline(
            device,
            target_format,
            &texture_bind_group_layout,
            &uniform_bind_group_layout,
        ));

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
    /// Function-level comment: Creates a colored cylinder mesh with high segment count for smooth appearance
    pub fn cylinder() -> Self {
        use std::f32::consts::TAU;

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let segments = 128;
        let radius = 0.004;
        let height = 20.0;
        let color = [1.0, 0.85, 0.0];
        let seg = segments.max(3);

        for i in 0..seg {
            let angle = (i as f32 / seg as f32) * TAU;
            let nx = angle.cos();
            let nz = angle.sin();

            // Bottom ring vertex
            vertices.push(MeshVertex {
                position: [nx * radius, -height, nz * radius],
                normal: [nx, 0.0, nz],
                color: color,
            });
            // Top ring vertex (smaller, tapered tip for surgical needle)
            vertices.push(MeshVertex {
                position: [nx * radius, height, nz * radius],
                normal: [nx, 0.0, nz],
                color: color,
            });
        }

        // Side wall indices — open tube (no caps)
        for i in 0..seg {
            let j = (i + 1) % seg;

            let b0 = (i * 2) as u32;
            let t0 = (i * 2 + 1) as u32;
            let b1 = (j * 2) as u32;
            let t1 = (j * 2 + 1) as u32;

            indices.extend_from_slice(&[b0, t0, t1, b0, t1, b1]);
        }

        // Bottom cap (flat end of needle handle)
        let bot_center = 2 * seg as u32;
        vertices.push(MeshVertex {
            position: [0.0, -height, 0.0],
            normal: [0.0, -1.0, 0.0],
            color: color,
        });
        for i in 0..seg {
            let angle = (i as f32 / seg as f32) * TAU;
            vertices.push(MeshVertex {
                position: [angle.cos() * radius, -height, angle.sin() * radius],
                normal: [0.0, -1.0, 0.0],
                color: color,
            });
        }
        for i in 0..seg {
            let a = bot_center + 1 + ((i + 1) % seg);
            let b = bot_center + 1 + i;
            indices.extend_from_slice(&[bot_center, a, b]);
        }
        Self { vertices, indices }
    }
    
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