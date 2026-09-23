#![allow(dead_code)]

use crate::data::volume_encoding::VolumeEncoding;
use crate::rendering::view::render_content::RenderContent;
use crate::rendering::core::pipeline::*;
use crate::rendering::view::{LABEL_COLORS, LABEL_NAMES, NeedleUniform, ObliquePlaneUniform};
use mcubes::{MarchingCubes, MeshSide};
use std::sync::Arc;
use tobj;
use serde::{Serialize, Deserialize};
use glam::{Mat4,Vec3};
use wgpu::{BindGroup, BindGroupLayout, Buffer, BufferUsages, Device, RenderPipeline};

/// Volume rendering parameters (sent to fragment shader)
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VolumeUniforms {
    pub ray_step_size: f32,
    pub max_steps: f32,
    pub is_packed_rg8: f32,
    pub bias: f32,
    pub roi_min: [f32; 3],
    pub window: f32,
    pub roi_max: [f32; 3],
    pub level: f32,
    pub vol_dims: [f32; 3],
    pub opacity: f32,
    pub view_proj: [f32; 16],
    pub inv_view_proj: [f32; 16],
    pub camera_position: [f32; 3],
    pub ball_enabled: f32,
    pub balls: [[f32; 4]; 4],
    pub volume_scale: [f32; 3],
    pub _volume_scale_pad: f32,
    pub light_dir: [f32; 3],
    pub aspect_ratio: f32,
    pub needle_count: u32,
    pub needle_enabled:f32,
    pub needle_index: u32,
    pub plane_rotation_angle: f32,
    pub oblique_planes: [ObliquePlaneUniform; 4],
    pub needles: [NeedleUniform; 32],
}

impl Default for VolumeUniforms {
    fn default() -> Self {
        Self {
            ray_step_size: 0.0004,
            max_steps: 1500.0,
            is_packed_rg8: 1.0,
            bias: VolumeEncoding::DEFAULT_HU_OFFSET,
            roi_min: [0.0, 0.0, 0.0],
            window: 1500.0,
            roi_max: [1.0, 1.0, 1.0],
            level: 400.0,
            vol_dims: [512.0, 512.0, 300.0],
            opacity: 1.0,
            light_dir: [0.5, 0.5, -1.0],
            aspect_ratio: 1.0,
            view_proj: Mat4::IDENTITY.to_cols_array(),
            inv_view_proj: Mat4::IDENTITY.to_cols_array(),
            camera_position: [0.0, 0.0, 0.0],
            ball_enabled: 0.0,
            balls: [[0.0; 4]; 4],
            volume_scale: [1.0, 1.0, 1.0],
            _volume_scale_pad: 0.0,
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
        let min_binding_size = std::num::NonZeroU64::new(std::mem::size_of::<VolumeUniforms>() as u64);
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
            size: std::mem::size_of::<VolumeUniforms>() as u64,
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

    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &VolumeUniforms) {
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

    /// Load a mesh from an in-memory OBJ buffer into the renderer's `[0,1]^3` UV space.
    pub fn import_obj_bytes(bytes: &[u8], kind: u32) -> Result<Vec<Mesh>, Box<dyn std::error::Error>> {
        let mut reader = std::io::Cursor::new(bytes);
        let (models, _materials) = tobj::load_obj_buf(
            &mut reader,
            &tobj::LoadOptions{
                triangulate:true,
                single_index:true,
                ..Default::default()
            },
            |_path| Ok((Vec::new(), Default::default())),
        )?;

        let mut result = Vec::new();
        for (obj_index, m) in models.into_iter().enumerate() {
            let mesh = &m.mesh;
            let vertex_count = mesh.positions.len() / 3;
            let normals: Vec<f32> = if mesh.normals.len() == mesh.positions.len() {
                mesh.normals.clone()
            } else {
                compute_vertex_normals(&mesh.positions, &mesh.indices)
            };

            let label_id = match Self::label_id_for_name(&m.name) {
                Some(id) => id,
                None => {
                    let ordinal = (obj_index + 1).min(7) as u8;
                    if obj_index + 1 > 7 {
                        log::warn!(
                            "OBJ has more than 7 objects ('{}' is #{}); reusing label id {} because the GPU slot layout only reserves 8 per mesh id",
                            m.name, obj_index + 1, ordinal
                        );
                    }
                    ordinal
                }
            };

            let mut vertices = Vec::<MeshVertex>::with_capacity(vertex_count);
            for i in 0..vertex_count {
                let pos = [mesh.positions[i * 3], mesh.positions[i * 3 + 1], mesh.positions[i * 3 + 2]];
                let normal = [normals[i * 3], normals[i * 3 + 1], normals[i * 3 + 2]];
                vertices.push(MeshVertex {
                    position:pos,
                    normal,
                    color:[1.0,1.0,1.0],
                });
            }

            result.push(Mesh {
                label_id,
                label_name:m.name,
                vertices,
                indices:mesh.indices.clone(),
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

        let extent = max - min;
        if extent.max_element() <= 0.0 {
            return Err("OBJ mesh has zero or invalid extent".into());
        }

        log::info!(
            "OBJ imported: {} object(s) min={:?}, max={:?}, extent={:?}",
            result.len(), min, max, extent
        );
        
        if kind == 0 {
            for mesh in &result {
                log::info!(
                    "  obj '{}' -> label_id {} ({} verts, {} tris)",
                    mesh.label_name,
                    mesh.label_id,
                    mesh.vertices.len(),
                    mesh.indices.len() / 3
                );
            }

            const UV_TOLERANCE: f32 = 0.05;
            if min.min_element() < -UV_TOLERANCE || max.max_element() > 1.0 + UV_TOLERANCE {
                log::warn!(
                    "OBJ vertices lie outside the expected [0,1]^3 UV space \
                    (min={:?}, max={:?}); the mesh may not align with the volume. \
                    Expected an OBJ exported by meshes_to_obj().",
                    min,
                    max
                );
            }
        } else {
            let volume_size_mm = Vec3::new(512.0, 512.0, 512.0);
            for mesh in &mut result {
                for v in &mut mesh.vertices {
                    let p = Vec3::from(v.position);
                    let uv = (p + volume_size_mm * 0.5) / volume_size_mm;
                    v.position = uv.to_array();
                }
            }
        }

        Ok(result)
    }

    /// Resolve an OBJ object name back to a segmentation label id.
    fn label_id_for_name(name: &str) -> Option<u8> {
        let needle = name.trim();
        LABEL_NAMES.iter().enumerate().skip(1) // index 0 is the empty background entry
            .find(|(_, candidate)| candidate.eq_ignore_ascii_case(needle))
            .map(|(id, _)| id as u8)
    }

    /// Serialize a single mesh as a standalone OBJ body (no `o` header).
    fn write_obj_body(obj: &mut String, mesh: &Self, offset: u32) {
        for v in &mesh.vertices {
            obj.push_str(&format!(
                "v {} {} {}\n",
                v.position[0], v.position[1], v.position[2]
            ));
        }
        for v in &mesh.vertices {
            obj.push_str(&format!(
                "vn {} {} {}\n",
                v.normal[0], v.normal[1], v.normal[2]
            ));
        }
        for tri in mesh.indices.chunks(3) {
            if tri.len() < 3 {
                continue;
            }
            obj.push_str(&format!(
                "f {}//{} {}//{} {}//{}\n",
                tri[0] + 1 + offset, tri[0] + 1 + offset,
                tri[1] + 1 + offset, tri[1] + 1 + offset,
                tri[2] + 1 + offset, tri[2] + 1 + offset,
            ));
        }
    }

    /// Serialize one mesh as a **complete, standalone** OBJ document.
    ///
    /// Used by the per-label ("split") export: each vertebra gets its own file
    /// whose vertex indices start at 1, so the file opens correctly in any
    /// viewer that assumes a single object. The `o` line is still written so
    /// the name survives a round trip through [`Self::import_obj`].
    pub fn mesh_to_obj_single(mesh: &Self) -> String {
        let mut obj = format!("o {}\n", mesh.label_name);
        Self::write_obj_body(&mut obj, mesh, 0);
        obj
    }

    /// Split a mesh list into one standalone OBJ document per object.
    pub fn meshes_to_obj_split(meshes: &Vec<Self>) -> Vec<(String, String)> {
        meshes.iter()
            .filter(|m| !m.indices.is_empty() && !m.vertices.is_empty())
            .map(|m| {
                let stem = if m.label_name.trim().is_empty() {
                    format!("label_{}", m.label_id)
                } else {
                    m.label_name.trim().to_string()
                };
                (stem, Self::mesh_to_obj_single(m))
            }).collect()
    }

    pub fn meshes_to_obj(meshes: &Vec<Self>) -> String {
        let mut obj = String::new();
        let mut offset = 0;
        for mesh in meshes {
            obj.push_str(&format!("o {}\n", mesh.label_name));
            Self::write_obj_body(&mut obj, mesh, offset);
            offset += mesh.vertices.len() as u32;
        }
        obj
    }
}

/// Taubin (λ|μ) smoothing on an indexed triangle mesh.
///
/// # Why not a plain Laplacian
///
/// Repeated Laplacian smoothing shrinks the mesh: every pass pulls each vertex
/// toward the centroid of its neighbours, and on a closed convex surface that
/// centroid is always slightly inside, so the volume bleeds away roughly
/// linearly with the pass count. For a vertebra that means the cortical rim
/// visibly deflates after only a handful of passes - not acceptable for
/// anything that may be measured later.
///
/// Taubin's two-pass scheme cancels the shrinkage to second order: a
/// contracting step with a positive factor `λ` followed by an *inflating* step
/// with a negative factor `μ`. The pass/stop band of the combined transfer
/// function is unchanged, so it still removes the high-frequency ripple we care
/// about, while the DC gain stays at 1.
///
/// # Choosing `μ` (measured, do not copy the folklore value)
///
/// The widely-quoted `μ ≈ -0.53 λ` is too weak: on a subdivided octasphere over
/// 10 rounds it still loses ~5.2% of the volume. The shrinkage-cancelling
/// relation uses Taubin's pass-band gain `KPB` (0.1 for a low-pass response):
///
/// ```text
/// μ = 1 / (KPB - 1/λ)
/// ```
///
/// For `λ = 0.5` that gives `μ = -0.526316`. Measured volume change of the same
/// octasphere over 10 rounds: plain Laplacian (`μ = 0`) `-10.541%`,
/// `μ = -0.53 λ = -0.265` `-5.170%`, closed form above `+0.481%`.
///
/// # Why `positions` is vertex-shared here
///
/// Marching cubes emits *unshared* vertices - every triangle pushes three
/// fresh vertices and indexes them `0,1,2,3,...` - so neighbouring triangles
/// share no index and a Laplacian built on `indices` alone would have no
/// neighbours to average. The caller therefore welds first (see
/// [`weld_positions`]) and this function operates on the welded, indexed
/// mesh. Normals are recomputed afterwards from the smoothed geometry.
///
/// `iterations` is the number of λ|μ *rounds* (two passes each).
fn taubin_smooth(
    positions: &mut [[f32; 3]],
    indices: &[u32],
    lambda: f32,
    mu: f32,
    iterations: usize,
) {
    let n = positions.len();
    if n == 0 || iterations == 0 {
        return;
    }

    // Build the 1-ring adjacency once: interior mesh topology does not change
    // while we only move vertices.
    let mut adj: Vec<Vec<u32>> = vec![Vec::new(); n];
    for tri in indices.chunks_exact(3) {
        let (a, b, c) = (tri[0], tri[1], tri[2]);
        for &(i, j) in &[(a, b), (b, c), (c, a)] {
            let (i, j) = (i as usize, j as usize);
            if !adj[i].contains(&(j as u32)) {
                adj[i].push(j as u32);
            }
            if !adj[j].contains(&(i as u32)) {
                adj[j].push(i as u32);
            }
        }
    }

    let mut scratch = positions.to_vec();

    // One Laplacian step: p += factor * (mean(neighbours) - p).
    // Boundary vertices (fewer than 3 neighbours) are pinned, since averaging
    // them pulls the open rim inward and marching cubes output can have them.
    let step = |src: &[[f32; 3]], dst: &mut [[f32; 3]], factor: f32| {
        for i in 0..n {
            let nb = &adj[i];
            if nb.len() < 3 {
                dst[i] = src[i];
                continue;
            }
            let inv = 1.0 / nb.len() as f32;
            let mut acc = [0.0f32; 3];
            for &j in nb {
                let p = src[j as usize];
                acc[0] += p[0];
                acc[1] += p[1];
                acc[2] += p[2];
            }
            let p = src[i];
            dst[i] = [
                p[0] + factor * (acc[0] * inv - p[0]),
                p[1] + factor * (acc[1] * inv - p[1]),
                p[2] + factor * (acc[2] * inv - p[2]),
            ];
        }
    };

    for _ in 0..iterations {
        step(positions, &mut scratch, lambda);
        step(&scratch, positions, mu);
    }
}

/// Weld vertices that share a position (within `tolerance`) and rebuild the
/// triangle index buffer against the welded array.
///
/// Marching cubes emits unshared vertices, so without this every triangle is
/// topologically isolated and no smoothing or normal averaging can cross a
/// face boundary. Returns the welded positions plus remapped indices, with
/// degenerate triangles (two or more corners collapsing to one vertex) dropped.
fn weld_positions(positions: &[[f32; 3]], indices: &[u32], tolerance: f32) -> (Vec<[f32; 3]>, Vec<u32>) {
    use std::collections::HashMap;

    let inv = 1.0 / tolerance.max(1e-9);
    let key = |p: &[f32; 3]| -> (i64, i64, i64) {
        (
            (p[0] * inv).round() as i64,
            (p[1] * inv).round() as i64,
            (p[2] * inv).round() as i64,
        )
    };

    let mut map: HashMap<(i64, i64, i64), u32> = HashMap::new();
    let mut welded: Vec<[f32; 3]> = Vec::with_capacity(positions.len());
    let mut remap: Vec<u32> = Vec::with_capacity(positions.len());

    for p in positions {
        let k = key(p);
        let idx = *map.entry(k).or_insert_with(|| {
            welded.push(*p);
            (welded.len() - 1) as u32
        });
        remap.push(idx);
    }

    let mut out_idx = Vec::with_capacity(indices.len());
    for tri in indices.chunks_exact(3) {
        let (a, b, c) = (
            remap[tri[0] as usize],
            remap[tri[1] as usize],
            remap[tri[2] as usize],
        );
        // A collapsed triangle has no area and no normal; skip it.
        if a != b && b != c && a != c {
            out_idx.push(a);
            out_idx.push(b);
            out_idx.push(c);
        }
    }

    (welded, out_idx)
}

fn compute_vertex_normals(positions:&[f32],indices:&[u32])->Vec<f32>{
    let vertex_count = positions.len()/3;

    // each vertex accumulate normal
    let mut normals = vec![glam::Vec3::ZERO;vertex_count];

    /*
        triangle:
             v2
            / \
           /   \
          v0---v1
        normal: (v1-v0)×(v2-v0)
    */
    for tri in indices.chunks(3) {
        if tri.len()!=3 {
            continue;
        }
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        let p0 = glam::Vec3::new(
            positions[i0*3],
            positions[i0*3+1],
            positions[i0*3+2],
        );
        let p1 = glam::Vec3::new(
            positions[i1*3],
            positions[i1*3+1],
            positions[i1*3+2],
        );
        let p2 = glam::Vec3::new(
            positions[i2*3],
            positions[i2*3+1],
            positions[i2*3+2],
        );
        let face_normal = (p1-p0).cross(p2-p0).normalize_or_zero();
        normals[i0]+=face_normal;
        normals[i1]+=face_normal;
        normals[i2]+=face_normal;
    }

    // normalize each vertex normal
    let mut result = Vec::<f32>::with_capacity(vertex_count*3);
    for n in normals {
        let n = n.normalize_or_zero();
        result.push(n.x);
        result.push(n.y);
        result.push(n.z);
    }
    result
}

/// Taubin smoothing tunables for [`spine`].
///
/// The per-round transfer function is `H(w) = (1 - lambda*w)(1 - mu*w)`, whose
/// `w^2` coefficient is `lambda*mu`. Taubin's shrinkage-cancelling choice uses
/// a pass band edge of `KPB = 0.1`, giving the closed form
///
/// ```text
/// mu = 1 / (KPB - 1/lambda)
/// ```
///
/// which for `lambda = 0.5` is `mu = -0.526316`. Note this is NOT
/// `-0.53 * lambda`: that value (-0.265) is too weak to cancel the contraction
/// and measured 5.2% volume loss on a subdivided sphere. With the correct pair
/// the DC gain of `H` is exactly 1 for the continuous Laplacian, so there is no
/// systematic deflation; the small residual drift of the discrete 1-ring
/// operator falls off as `1/mesh_density` and is negligible at the density of a
/// marching-cubes spine mesh (see the volume test). The iteration count is
/// therefore effectively a pure smoothness knob.
///
/// Round count matters more than it looks. Measured on the real spine mask
/// (`scripts/pick_taubin_strength.py`), residual Z-ripple after N rounds as a
/// fraction of the unsmoothed ripple:
///
/// ```text
/// N =  2 -> 85%    N = 10 -> 62%    N = 20 -> 48%
/// N =  5 -> 73%    N = 15 -> 54%    N = 30 -> 40%
/// ```
///
/// The earlier value of 10 left ~62% of the ripple standing, which is exactly
/// why stripes remained visible. The surviving corrugation peaks at 6-18 slice
/// wavelengths (6-18 mm) whereas the vertebral body itself is 36 mm and larger,
/// so widening the stop band to reach the 6-18 band costs no anatomy. Kept at
/// module scope so the tests can assert against the very values production uses.
const SMOOTH_ITERATIONS: usize = 40;
const SMOOTH_LAMBDA: f32 = 0.5;
const SMOOTH_KPB: f32 = 0.1;
const SMOOTH_MU: f32 = 1.0 / (SMOOTH_KPB - 1.0 / SMOOTH_LAMBDA);

pub fn spine(
    segmentation: &[u8],
    dims: (usize, usize, usize),
    spacing: (f32, f32, f32),
    label_ids:&[u8],
    _iso_legacy: f32,
)-> Vec<Mesh> {
    let (rows, cols, slices) = dims;
    let mut results = Vec::new();

    // NOTE on the Z corrugation ("stacked bread slices"):
    //
    // A previous attempt blurred this binary field along Z before marching
    // cubes. That was measured on the real mask and does NOT work, so it was
    // removed. The reason is structural: the corrugation comes from the mask
    // containing exactly-replicated slices (one identical slice pair every 3
    // layers, produced by nnU-Net's `order_z = 0` nearest-neighbour projection
    // onto the finer native grid). Blurring redistributes the samples that are
    // already there but cannot recreate the interpolation the projection threw
    // away, so the 0.5 level set stays pinned to the same lattice. Measured
    // with `scripts/measure_z_corrugation.py` on `spine.raw`, the fraction of
    // stalled boundary advances is flat across sigma:
    //
    //     sigma = 0.0 -> 0.927     sigma = 1.5 -> 0.930
    //     sigma = 0.8 -> 0.927     sigma = 3.0 -> 0.933
    //
    // The fix belongs upstream, in the projection itself: see
    // `server::resample::upsample_logits_argmax`'s `linear_aniso_axis`.
    let iso = _iso_legacy.clamp(0.05, 0.95);

    for &label_id in label_ids {
        let mut binary = vec![0.0f32; segmentation.len()];
        let mut has_voxel = false;
        for (i, &v) in segmentation.iter().enumerate() {
            if v == label_id {
                binary[i] = 1.0;
                has_voxel = true;
            }
        }

        // Skip the empty label.
        //
        // NOTE: the binary field goes into marching cubes unblurred. See the
        // comment above `iso` for why a Z blur was tried and removed.
        if !has_voxel {
            continue;
        }

        let field = binary;

        let mc_dims = (cols, rows, slices);
        let mc = MarchingCubes::new(
            mc_dims,
            (spacing.0, spacing.1, spacing.2),
            (1.0, 1.0, 1.0),
            lin_alg::f32::Vec3::new(0.0, 0.0, 0.0),
            field,
            iso,
        ).expect("mcubes init: dims/values length mismatch");
        let cube_mesh = mc.generate(MeshSide::InsideOnly);

        let color = LABEL_COLORS.get(label_id as usize).copied().unwrap_or([1.0,1.0,1.0,1.0]);

        // Marching cubes hands back *unshared* vertices: one fresh triple per
        // triangle, indexed 0,1,2,3,... So the three inputs below are welded
        // into a real indexed mesh first, which is what makes both the Taubin
        // pass and the normal averaging able to cross face boundaries at all.
        //
        // Without the weld, `compute_vertex_normals` would average each vertex
        // against itself and collapse to a flat per-face normal, which is why
        // mcubes' own `-grad(rho)` normals were kept previously. After welding
        // the gradient normals are stale (the geometry has moved), so normals
        // MUST be recomputed from the smoothed positions.
        let raw_positions: Vec<[f32; 3]> = cube_mesh
            .vertices
            .iter()
            .map(|v| [v.posit.x, v.posit.y, v.posit.z])
            .collect();
        let raw_indices: Vec<u32> =
            cube_mesh.indices.iter().map(|&i| i as u32).collect();

        // Weld tolerance is a thousandth of the smallest voxel edge. Anything
        // looser starts merging genuinely distinct surface points.
        let min_pitch = spacing.0.min(spacing.1).min(spacing.2).max(1e-6);
        let (mut positions, indices) = weld_positions(&raw_positions, &raw_indices, min_pitch * 1e-3);

        // Taubin lambda|mu - see the module-level constants above for the
        // derivation and the measured round-count response.
        taubin_smooth(
            &mut positions,
            &indices,
            SMOOTH_LAMBDA,
            SMOOTH_MU,
            SMOOTH_ITERATIONS,
        );

        let flat: Vec<f32> = positions.iter().flat_map(|p| p.iter().copied()).collect();
        let normals = compute_vertex_normals(&flat, &indices);

        let vertices: Vec<MeshVertex> = positions
            .iter()
            .enumerate()
            .map(|(i, p)| MeshVertex {
                position: *p,
                normal: [normals[i * 3], normals[i * 3 + 1], normals[i * 3 + 2]],
                color: [color[0], color[1], color[2]],
            })
            .collect();

        results.push(Mesh {
            label_id: label_id,
            label_name: LABEL_NAMES[label_id as usize].to_string(),
            vertices,
            indices,
        });
    }

    let phys_x = cols as f32 * spacing.0;   // mcubes X extent = texture/volume Y
    let phys_y = rows as f32 * spacing.1;   // mcubes Y extent = texture/volume X
    let phys_z = slices as f32 * spacing.2; // mcubes Z extent = texture/volume Z
    let inv_x = 1.0 / phys_x.max(1e-6);
    let inv_y = 1.0 / phys_y.max(1e-6);
    let inv_z = 1.0 / phys_z.max(1e-6);

    for mesh in &mut results {
        for v in &mut mesh.vertices {
            let px = v.position[0] * inv_x;
            let py = v.position[1] * inv_y;
            let pz = v.position[2] * inv_z;
            v.position = [px, py, pz];
        }
    }

    results
}

impl MeshVertex {
    /// Function-level comment: Vertex attribute array for position, normal, and color
    /// Defines position, normal, and color attributes for the vertex shader
    const ATTRS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3];

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