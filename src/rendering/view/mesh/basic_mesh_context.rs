#![allow(dead_code)]

use super::mesh::{Mesh, BasicLightingUniforms};
use crate::rendering::core::pipeline::{
    create_basic_lighting_bind_group_layout, 
    create_basic_mesh_pipeline_with_lighting,
};
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue};

/// Function-level comment: Simplified uniform data structure for basic mesh rendering
/// Contains the combined model-view-projection matrix and an optional slice plane
/// used to clip the mesh against the current MPR plane (volume-UV space).
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BasicUniforms {
    pub model_view_proj: [[f32; 4]; 4],
    /// Slice plane normal in `[0, 1]^3` volume-UV space.
    pub plane_normal: [f32; 3],
    /// `d` of the plane equation `normal · x + d = 0`.
    pub plane_d: f32,
    /// Half-thickness of the accepted slice in volume-UV units. The shader
    /// keeps fragments with `|normal · x + d| <= slice_thickness * 0.5`.
    pub slice_thickness: f32,
    /// `1.0` enables slice clipping, `0.0` disables it (mesh rendered as-is).
    pub slice_enabled: f32,
    /// Pad the struct to a 16-byte boundary so the WGSL layout matches.
    pub _pad: [f32; 2],
}

impl Default for BasicUniforms {
    fn default() -> Self {
        Self {
            model_view_proj: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            plane_normal: [0.0, 0.0, 1.0],
            plane_d: 0.0,
            slice_thickness: 0.0,
            slice_enabled: 0.0,
            _pad: [0.0; 2],
        }
    }
}

/// Function-level comment: Basic mesh context for simplified rendering operations
/// This struct provides a minimal interface for mesh rendering without complex features
pub struct BasicMeshContext {
    pub pipeline: std::sync::Arc<wgpu::RenderPipeline>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_vertices: u32,
    pub num_indices: u32,
    // Simplified uniform handling - only basic uniforms needed
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    /// Function-level comment: Uniform buffer for lighting parameters
    pub lighting_uniform_buffer: wgpu::Buffer,
    pub lighting_bind_group: wgpu::BindGroup,
    /// Cached CPU-side copy of the latest uniforms so that updating only the
    /// MVP matrix preserves the slice plane configuration.
    cached_uniforms: BasicUniforms,
}

impl BasicMeshContext {
    /// Create a new basic mesh context with simplified pipeline and uniforms
    pub fn new(device: &Device, queue: &Queue, mesh: &super::mesh::Mesh, use_depth: bool) -> Self {
        log::info!(
            "Creating BasicMeshContext with {} vertices, {} indices, depth: {}",
            mesh.vertices.len(),
            mesh.indices.len(),
            use_depth
        );

        // Create vertex buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Basic Mesh Vertex Buffer"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Create index buffer
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Basic Mesh Index Buffer"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Basic Mesh Uniform Buffer"),
            size: std::mem::size_of::<BasicUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Initialize with default uniforms
        let default_uniforms = BasicUniforms::default();
        queue.write_buffer(
            &uniform_buffer,
            0,
            bytemuck::cast_slice(&[default_uniforms]),
        );

        // Create bind group layout for basic uniforms
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Basic Mesh Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Create lighting uniform buffer
        let lighting_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Basic Lighting Uniform Buffer"),
            size: std::mem::size_of::<BasicLightingUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Initialize with default lighting uniforms
        let default_lighting_uniforms = BasicLightingUniforms::default();
        queue.write_buffer(
            &lighting_uniform_buffer,
            0,
            bytemuck::cast_slice(&[default_lighting_uniforms]),
        );

        // Create lighting bind group layout
        let lighting_bind_group_layout = create_basic_lighting_bind_group_layout(device);

        // Create lighting-enabled mesh pipeline with depth testing enabled to match render pass
        let pipeline = std::sync::Arc::new(create_basic_mesh_pipeline_with_lighting(
            device,
            &bind_group_layout,
            &lighting_bind_group_layout,
            use_depth,
            wgpu::CompareFunction::Less,
        ));

        // Create bind group for uniforms
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Basic Mesh Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create bind group for lighting
        let lighting_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Basic Lighting Bind Group"),
            layout: &lighting_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: lighting_uniform_buffer.as_entire_binding(),
            }],
        });

        log::info!("BasicMeshContext created successfully");

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            num_vertices: mesh.vertices.len() as u32,
            num_indices: mesh.indices.len() as u32,
            uniform_buffer,
            bind_group,
            lighting_uniform_buffer,
            lighting_bind_group,
            cached_uniforms: default_uniforms,
        }
    }

    /// Function-level comment: Update uniforms with combined model-view-projection matrix
    pub fn update_uniforms(&mut self, queue: &Queue, model_view_proj_matrix: &[[f32; 4]; 4]) {
        // Preserve the most recent slice plane configuration; only the MVP matrix is refreshed.
        self.cached_uniforms.model_view_proj = *model_view_proj_matrix;
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.cached_uniforms]),
        );
        log::trace!("Updated basic mesh uniforms with MVP matrix");
    }

    /// Configure the slice plane used by the fragment shader to clip the mesh
    /// to a thin slab around the current MPR plane (in `[0, 1]^3` UV space).
    ///
    /// `plane_normal` must be unit length. `thickness` is the total slab
    /// thickness in UV units (0.003 ≈ 0.77 mm for a 256 mm volume). When
    /// `enabled` is `false` the mesh is rendered in full.
    pub fn set_slice_plane(
        &mut self,
        queue: &Queue,
        plane_normal: [f32; 3],
        plane_d: f32,
        thickness: f32,
        enabled: bool,
    ) {
        self.cached_uniforms.plane_normal = plane_normal;
        self.cached_uniforms.plane_d = plane_d;
        self.cached_uniforms.slice_thickness = thickness.max(0.0);
        self.cached_uniforms.slice_enabled = if enabled { 1.0 } else { 0.0 };
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.cached_uniforms]),
        );
    }

    pub fn update_lighting(&self, queue: &Queue, uniforms: BasicLightingUniforms) {
        queue.write_buffer(
            &self.lighting_uniform_buffer,
            0,
            bytemuck::cast_slice(&[uniforms]),
        );
    }

    /// Function-level comment: Render the mesh using the basic pipeline with lighting
    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        // Single debug log per render call instead of 6 separate logs
        log::debug!(
            "[BASIC_MESH_RENDER] Rendering mesh: {} indices, {} vertices",
            self.num_indices,
            self.num_vertices
        );

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_bind_group(1, &self.lighting_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);

        log::trace!("BasicMeshContext::render - Draw call completed");
    }

    /// Function-level comment: Get basic memory statistics for the mesh context
    /// Returns (vertex_buffer_size, index_buffer_size, 0.0, 0.0) for compatibility
    pub fn get_memory_stats(&self) -> (u64, u64, f32, f32) {
        let vertex_size = self.vertex_buffer.size();
        let index_size = self.index_buffer.size();
        (vertex_size, index_size, 0.0, 0.0)
    }
}

struct MeshSlot {
    label_id: u8,
    label_name: String,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    visible: bool,
}

pub struct MultiMeshContext {
    pipeline: std::sync::Arc<wgpu::RenderPipeline>,
    /// Pipeline variant without depth-stencil attachment, for use in render
    /// passes that don't have a depth buffer (e.g. MPR slice overlay).
    pipeline_no_depth: std::sync::Arc<wgpu::RenderPipeline>,
    slots: Vec<MeshSlot>,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    lighting_uniform_buffer: wgpu::Buffer,
    lighting_bind_group: wgpu::BindGroup,
    /// Cached CPU-side copy of the latest uniforms so that updating only the
    /// MVP matrix preserves the slice plane configuration.
    cached_uniforms: BasicUniforms,
}

impl MultiMeshContext {
    pub fn new(device: &Device, queue: &Queue) -> Self {
        let lighting = BasicLightingUniforms::default();
        let bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("MultiMesh MVP Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    // The MVP/plane uniforms are read by both the vertex
                    // stage (clip-space transform) and the fragment stage
                    // (slice-plane discard), so the binding must be visible
                    // to both stages.
                    visibility: wgpu::ShaderStages::VERTEX
                        | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            },
        );
        let lighting_layout = create_basic_lighting_bind_group_layout(device);

        let pipeline = std::sync::Arc::new(create_basic_mesh_pipeline_with_lighting(
            device, &bind_group_layout, &lighting_layout, /*use_depth=*/ true,
            wgpu::CompareFunction::LessEqual,
        ));

        // Second pipeline variant without depth-stencil attachment, for use
        // in render passes that don't have a depth buffer (e.g. MPR overlay).
        let pipeline_no_depth = std::sync::Arc::new(create_basic_mesh_pipeline_with_lighting(
            device, &bind_group_layout, &lighting_layout, /*use_depth=*/ false,
            wgpu::CompareFunction::Always,
        ));

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MultiMesh MVP"),
            size: std::mem::size_of::<BasicUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[BasicUniforms::default()]));

        let mvp_entry = wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        };
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MultiMesh MVP BG"),
            layout: &bind_group_layout,
            entries: &[mvp_entry],
        });

        let lighting_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MultiMesh Lighting"),
            size: std::mem::size_of::<BasicLightingUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&lighting_uniform_buffer, 0,
            bytemuck::cast_slice(&[lighting]));

        let lighting_entry = wgpu::BindGroupEntry {
            binding: 0,
            resource: lighting_uniform_buffer.as_entire_binding(),
        };
        let lighting_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MultiMesh Lighting BG"),
            layout: &lighting_layout,
            entries: &[lighting_entry],
        });

        Self {
            pipeline, pipeline_no_depth, slots: Vec::new(),
            uniform_buffer, bind_group,
            lighting_uniform_buffer, lighting_bind_group,
            cached_uniforms: BasicUniforms::default(),
        }
    }

    pub fn set_meshes(&mut self, device: &Device, meshes: &[Mesh]) {
        self.slots.clear();
        for m in meshes {
            self.slots.push(MeshSlot {
                label_id: m.label_id,
                label_name: m.label_name.clone(),
                vertex_buffer: device.create_buffer_init(
                    &wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("vert label={}", m.label_id)),
                        contents: bytemuck::cast_slice(&m.vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    }),
                index_buffer: device.create_buffer_init(
                    &wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("idx label={}", m.label_id)),
                        contents: bytemuck::cast_slice(&m.indices),
                        usage: wgpu::BufferUsages::INDEX,
                    }),
                num_indices: m.indices.len() as u32,
                visible: true,
            });
        }
    }

    pub fn set_visibility(&mut self, label_id: u8, visible: bool) {
        for s in &mut self.slots {
            if s.label_id == label_id { s.visible = visible; }
        }
    }

    pub fn update_uniforms(&mut self, queue: &Queue, mvp: &[[f32; 4]; 4]) {
        // Preserve the most recent slice plane configuration; only the MVP
        // matrix is updated.
        self.cached_uniforms.model_view_proj = *mvp;
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.cached_uniforms]),
        );
    }

    /// Configure the slice plane used by the fragment shader to clip every
    /// mesh in this context to a thin slab around the current MPR plane
    /// (in `[0, 1]^3` UV space).
    pub fn set_slice_plane(
        &mut self,
        queue: &Queue,
        plane_normal: [f32; 3],
        plane_d: f32,
        thickness: f32,
        enabled: bool,
    ) {
        self.cached_uniforms.plane_normal = plane_normal;
        self.cached_uniforms.plane_d = plane_d;
        self.cached_uniforms.slice_thickness = thickness.max(0.0);
        self.cached_uniforms.slice_enabled = if enabled { 1.0 } else { 0.0 };
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.cached_uniforms]),
        );
    }

    pub fn update_lighting(&self, queue: &Queue, l: BasicLightingUniforms) {
        queue.write_buffer(&self.lighting_uniform_buffer, 0,
            bytemuck::cast_slice(&[l]));
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_bind_group(1, &self.lighting_bind_group, &[]);
        for s in &self.slots {
            if !s.visible || s.num_indices == 0 { continue; }
            render_pass.set_vertex_buffer(0, s.vertex_buffer.slice(..));
            render_pass.set_index_buffer(s.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..s.num_indices, 0, 0..1);
        }
    }

    /// Render using the pipeline variant without depth-stencil attachment.
    /// For use in render passes that don't have a depth buffer (e.g. MPR overlay).
    pub fn render_no_depth(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_pipeline(&self.pipeline_no_depth);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_bind_group(1, &self.lighting_bind_group, &[]);
        for s in &self.slots {
            if !s.visible || s.num_indices == 0 { continue; }
            render_pass.set_vertex_buffer(0, s.vertex_buffer.slice(..));
            render_pass.set_index_buffer(s.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..s.num_indices, 0, 0..1);
        }
    }
}