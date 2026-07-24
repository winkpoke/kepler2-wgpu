#![allow(dead_code)]

use crate::data::volume_encoding::VolumeEncoding;
use crate::rendering::view::render_content::RenderContent;
use crate::rendering::core::pipeline::*;
use crate::rendering::view::{LABEL_COLORS, LABEL_NAMES, NeedleUniform, ObliquePlaneUniform};
use mcubes::{MarchingCubes, MeshSide};
use std::sync::Arc;
use tobj;
use serde::{Serialize, Deserialize};
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
    /// Function-level comment: Inverse view-projection matrix shared by mesh and
    /// volume so volume rays are generated in the same world space as the mesh MVP.
    pub inv_view_proj: [f32; 16],
    /// Function-level comment: Forward view-projection matrix used for correct
    /// volume depth writes in the shared camera coordinate system.
    pub view_proj: [f32; 16],
    pub vol_dims: [f32; 3],
    pub preset: f32,
    pub needle_count: u32,
    pub needle_enabled:f32,
    pub needle_index: u32,
    pub plane_rotation_angle: f32,
    pub oblique_planes: [ObliquePlaneUniform; 4],
    pub needles: [NeedleUniform; 32],
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
            inv_view_proj: Mat4::IDENTITY.to_cols_array(),
            view_proj: Mat4::IDENTITY.to_cols_array(),
            vol_dims: [512.0, 512.0, 300.0],
            preset: 1.0,
            needle_count: 0,
            needle_enabled: 0.0,
            needle_index: 0,
            plane_rotation_angle: 180.0,
            oblique_planes: [ObliquePlaneUniform::default(); 4],
            needles: [NeedleUniform::default(); 32],
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

        // Texture Bind Group: volume + segmentation overlay.
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
    pub opacity: f32,
    pub light_color: [f32; 3],
    pub light_intensity: f32,
    pub ambient_color: [f32; 3],
    pub ambient_intensity: f32,
}

impl Default for BasicLightingUniforms {
    /// Function-level comment: Creates default lighting configuration for basic mesh rendering.
    /// Provides reasonable defaults for directional lighting from top-left-front direction.
    fn default() -> Self {
        Self {
            light_direction: [0.6, -0.7, 0.3], // Top-left-front direction
            opacity: 1.0,
            light_color: [1.0, 1.0, 1.0], // White light
            light_intensity: 1.0,
            ambient_color: [0.4, 0.4, 0.4],
            ambient_intensity: 0.5,
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
            opacity: 1.0,
            light_color: self.light_color,
            light_intensity: self.light_intensity,
            ambient_color: self.ambient_color,
            ambient_intensity: self.ambient_intensity,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
}

// Manual implementation to avoid potential bytemuck derive issues
unsafe impl bytemuck::Zeroable for MeshVertex {}
unsafe impl bytemuck::Pod for MeshVertex {}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Mesh {
    pub label_id: u8,
    pub label_name: String,
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
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

        Self { label_id: 0, label_name: "Cube".to_string(), vertices, indices }
    }

    pub fn import_obj(path:&str)->Result<Vec<Mesh>, Box<dyn std::error::Error>>{
        let (models, _materials) = tobj::load_obj(
            path,
            &tobj::LoadOptions{
                triangulate:true,
                single_index:true,
                ..Default::default()
            }
        )?;

        let mut result = Vec::new();
        for m in models {
            let mesh = &m.mesh;
            let vertex_count = mesh.positions.len() / 3;
            let normals: Vec<f32> = if mesh.normals.len() == mesh.positions.len() {
                mesh.normals.clone()
            } else {
                compute_vertex_normals(&mesh.positions, &mesh.indices)
            };
            let mut vertices = Vec::<MeshVertex>::with_capacity(vertex_count);
            for i in 0..vertex_count {
                let pos = [
                    mesh.positions[i * 3],
                    mesh.positions[i * 3 + 1],
                    mesh.positions[i * 3 + 2],
                ];
                let normal = [normals[i * 3], normals[i * 3 + 1], normals[i * 3 + 2]];
                vertices.push(MeshVertex {
                    position: pos,
                    normal,
                    color: [1.0, 1.0, 1.0],
                });
            }

            result.push(Mesh {
                label_id: 0,
                label_name: m.name,
                vertices,
                indices: mesh.indices.clone(),
            });
        }

        // Compute actual bounding box from all vertices
        let mut min = glam::Vec3::splat(f32::MAX);
        let mut max = glam::Vec3::splat(f32::MIN);
        for mesh in &result {
            for v in &mesh.vertices {
                let p = glam::Vec3::from(v.position);
                min = min.min(p);
                max = max.max(p);
            }
        }

        log::info!("OBJ bbox: min={:?}, max={:?}", min, max);

        let center = (min + max) * 0.5;
        let extent = max - min;
        let max_dim = extent.max_element();

        if max_dim <= 0.0 {
            return Err("OBJ mesh has zero or invalid extent".into());
        }

        // Function-level comment: Normalize OBJ meshes into the shared `[0, 1]^3`
        // world cube so imported meshes and ray-marched volumes consume the same
        // camera transform without per-object correction terms.
        let norm_scale = 1.0 / max_dim;
        for mesh in &mut result {
            for v in &mut mesh.vertices {
                let mut p = glam::Vec3::from(v.position);
                p = (p - center) * norm_scale + glam::Vec3::splat(0.5);
                v.position = p.to_array();
            }
        }

        log::info!("OBJ normalized: center={:?}, max_dim={}", center, max_dim);

        Ok(result)
    }

    pub fn meshes_to_obj(meshes: &Vec<Self>) -> String {
        let mut obj = String::new();
        let mut offset = 0;
        for mesh in meshes {
            obj.push_str(&format!("o {}\n", mesh.label_name));
            for v in &mesh.vertices {
                obj.push_str(&format!(
                    "v {} {} {}\n",
                    v.position[0], v.position[1], v.position[2],
                ));
            }
            for v in &mesh.vertices {
                obj.push_str(&format!(
                    "vn {} {} {}\n",
                    v.normal[0], v.normal[1], v.normal[2],
                ));
            }
            for tri in mesh.indices.chunks(3) {
                obj.push_str(&format!(
                    "f {}//{} {}//{} {}//{}\n",
                    tri[0] + 1 + offset,
                    tri[0] + 1 + offset,
                    tri[1] + 1 + offset,
                    tri[1] + 1 + offset,
                    tri[2] + 1 + offset,
                    tri[2] + 1 + offset,
                ));
            }
            offset += mesh.vertices.len() as u32;
        }
        obj
    }
}

fn compute_vertex_normals(positions: &Vec<f32>, indices: &Vec<u32>) -> Vec<f32> {
    let vertex_count = positions.len() / 3;

    // each vertex accumulate normal
    let mut normals = vec![glam::Vec3::ZERO; vertex_count];

    /*
        triangle:
             v2
            / \
           /   \
          v0---v1
        normal: (v1-v0)×(v2-v0)
    */
    for tri in indices.chunks(3) {
        if tri.len() != 3 {
            continue;
        }
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        let p0 = glam::Vec3::new(
            positions[i0 * 3],
            positions[i0 * 3 + 1],
            positions[i0 * 3 + 2],
        );
        let p1 = glam::Vec3::new(
            positions[i1 * 3],
            positions[i1 * 3 + 1],
            positions[i1 * 3 + 2],
        );
        let p2 = glam::Vec3::new(
            positions[i2 * 3],
            positions[i2 * 3 + 1],
            positions[i2 * 3 + 2],
        );
        let face_normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();
        normals[i0] += face_normal;
        normals[i1] += face_normal;
        normals[i2] += face_normal;
    }

    // normalize each vertex normal
    let mut result = Vec::<f32>::with_capacity(vertex_count * 3);
    for n in normals {
        let n = n.normalize_or_zero();
        result.push(n.x);
        result.push(n.y);
        result.push(n.z);
    }
    result
}

pub fn spine(
    segmentation: &[u8],
    dims: (usize, usize, usize), // (rows, cols, slices)
    spacing: (f32, f32, f32),
    label_ids: &[u8],
    iso: f32,
) -> Vec<Mesh> {
    let (rows, cols, slices) = dims;
    let mut results = Vec::new();

    for &label_id in label_ids {
        let mut field = vec![0.0f32; segmentation.len()];
        let mut has_voxel = false;
        for (i, &v) in segmentation.iter().enumerate() {
            if v == label_id {
                field[i] = 1.0;
                has_voxel = true;
            }
        }

        // skip empty label
        if !has_voxel {
            continue;
        }

        // marching cubes
        let mc = MarchingCubes::new(
            dims,
            (spacing.0, spacing.1, spacing.2),
            (1.0, 1.0, 1.0),
            lin_alg::f32::Vec3::new(0.0, 0.0, 0.0),
            field,
            iso,
        )
        .expect("mcubes init: dims/values length mismatch");
        let cube_mesh = mc.generate(MeshSide::InsideOnly);

        let color = LABEL_COLORS
            .get(label_id as usize)
            .copied()
            .unwrap_or([1.0, 1.0, 1.0, 1.0]);

        let vertices: Vec<MeshVertex> = cube_mesh
            .vertices
            .iter()
            .map(|v| MeshVertex {
                position: [v.posit.x, v.posit.y, v.posit.z],
                normal: [v.normal.x, v.normal.y, v.normal.z],
                color: [color[0], color[1], color[2]],
            })
            .collect();
        let indices: Vec<u32> = cube_mesh.indices.iter().map(|&i| i as u32).collect();

        results.push(Mesh {
            label_id: label_id,
            label_name: LABEL_NAMES[label_id as usize].to_string(),
            vertices,
            indices,
        });
    }

    // Function-level comment: Map segmentation-derived meshes into the same
    // normalized world cube as the volume texture so one camera controls both.
    let extent_x = cols as f32 * spacing.0;
    let extent_y = rows as f32 * spacing.1;
    let extent_z = slices as f32 * spacing.2;

    for mesh in &mut results {
        for v in &mut mesh.vertices {
            v.position[0] /= extent_x.max(1e-6);
            v.position[1] /= extent_y.max(1e-6);
            v.position[2] /= extent_z.max(1e-6);
        }
    }

    results
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