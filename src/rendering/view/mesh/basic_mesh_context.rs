#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use super::mesh::{Mesh, BasicLightingUniforms};
use crate::rendering::core::pipeline::{
    create_basic_lighting_bind_group_layout, 
    create_basic_mesh_pipeline_with_lighting,
};
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue};

/// Uniform data shared by the basic mesh vertex/fragment shader.
///
/// Contains:
/// - combined model-view-projection matrix
/// - optional MPR slice plane used to clip meshes in volume UV space
///
/// IMPORTANT:
/// Keep this ABI synchronized with the corresponding WGSL uniform struct.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BasicUniforms {
    pub model_view_proj: [[f32; 4]; 4],
    /// Slice plane normal in `[0, 1]^3` volume-UV space.
    pub plane_normal: [f32; 3],
    /// `d` in the plane equation: `normal · x + d = 0`
    pub plane_d: f32,
    /// Total accepted slab thickness in volume-UV units.
    /// The shader keeps fragments satisfying: `abs(normal · x + d) <= slice_thickness * 0.5`
    pub slice_thickness: f32,
    /// `1.0` enables slice clipping.
    /// `0.0` disables slice clipping.
    pub slice_enabled: f32,
    /// Padding required to keep the Rust/WGSL uniform layout aligned.
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

/// Semantic category of a mesh.
/// This is intentionally separate from `MeshId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshKind {
    /// Spine or vertebra mesh generated from segmentation.
    Spine,
    /// Generic segmentation-derived mesh.
    Segmentation,
    /// Needle geometry.
    Needle,
    /// Any other mesh type not requiring special renderer semantics.
    Other,
}

/// CPU-side description of one mesh upload/update.
/// This type is used to synchronize one logical CPU mesh with its GPU buffers.
/// `id` identifies the logical mesh
/// 
/// `revision` identifies the geometry version. When:
/// ```text
/// CPU revision == uploaded_revision
/// ```
/// no GPU buffer recreation is performed.
///
/// When:
/// ```text
/// CPU revision != uploaded_revision
/// ```
/// the mesh buffers are rebuilt.
///
/// This is the core Gate 2 mechanism that prevents unrelated meshes from
/// being re-uploaded when another mesh changes.
pub struct MeshUpload<'a> {
    /// Stable logical identity.
    pub id: u32,
    /// Semantic mesh category.
    pub kind: MeshKind,
    /// Optional segmentation/spine label.
    /// Generic OBJ meshes normally use `None`.
    pub label_id: Option<u8>,
    /// Optional human-readable label or asset name.
    pub label_name: Option<&'a str>,
    /// CPU-side source revision.
    pub revision: u64,
    /// Mesh geometry to upload.
    pub mesh: &'a Mesh,
}

/// GPU-resident state for one logical mesh.
/// This type deliberately contains only GPU/rendering concerns.
/// Domain identity and category are retained so the renderer can still support:
/// - visibility by `MeshId`
/// - visibility by `label_id`
/// - filtering by `MeshKind`
struct MeshSlot {
    /// Stable logical identity.
    id: u32,
    /// Semantic mesh category.
    kind: MeshKind,
    /// Optional segmentation/spine label.
    label_id: Option<u8>,
    /// Optional display name / source name.
    label_name: Option<String>,
    /// GPU vertex buffer.
    vertex_buffer: wgpu::Buffer,
    /// GPU index buffer.
    index_buffer: wgpu::Buffer,
    /// Number of indices to draw.
    num_indices: u32,
    /// Whether this mesh is currently rendered.
    visible: bool,
    /// CPU revision currently uploaded to GPU.
    uploaded_revision: u64,
}

impl MeshSlot {
    fn from_upload(
        device: &Device,
        upload: MeshUpload<'_>,
        visible: bool,
    ) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MultiMesh Vertex Buffer"),
            contents: bytemuck::cast_slice(&upload.mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MultiMesh Index Buffer"),
            contents: bytemuck::cast_slice(&upload.mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            id: upload.id,
            kind: upload.kind,
            label_id: upload.label_id,
            label_name: upload.label_name.map(str::to_owned),
            vertex_buffer,
            index_buffer,
            num_indices: upload.mesh.indices.len() as u32,
            visible,
            uploaded_revision: upload.revision,
        }
    }
}

/// GPU registry for multiple logical meshes.
///
/// The important ownership model is:
///
/// ```text
/// CPU Mesh Asset
///      │
///      │ MeshId + revision
///      ▼
/// MultiMeshContext
///      │
///      ├── MeshId(1)   -> GPU buffers
///      ├── MeshId(2)   -> GPU buffers
///      ├── MeshId(100) -> GPU buffers
///      └── MeshId(101) -> GPU buffers
/// ```
///
/// Updating one mesh does not force all other meshes to be recreated.
pub struct MultiMeshContext {
    /// Pipeline used when the render pass has a depth attachment.
    pipeline: Arc<wgpu::RenderPipeline>,
    /// Pipeline variant used when the render pass has no depth attachment.
    pipeline_no_depth: Arc<wgpu::RenderPipeline>,
    /// GPU meshes indexed by stable logical identity.
    slots: HashMap<u32, MeshSlot>,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    lighting_uniform_buffer: wgpu::Buffer,
    lighting_bind_group: wgpu::BindGroup,
    /// Cached CPU-side uniforms.
    ///
    /// Updating MVP preserves slice clipping state.
    cached_uniforms: BasicUniforms,
}

impl MultiMeshContext {
    pub fn new(device: &Device, queue: &Queue) -> Self {
        let default_uniforms = BasicUniforms::default();
        let lighting = BasicLightingUniforms::default();
        let bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("MultiMesh MVP Layout"),
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
            },
        );
        let lighting_layout = create_basic_lighting_bind_group_layout(device);

        let pipeline = Arc::new(create_basic_mesh_pipeline_with_lighting(
            device, &bind_group_layout, &lighting_layout, true,
            wgpu::CompareFunction::LessEqual,
        ));

        let pipeline_no_depth = Arc::new(create_basic_mesh_pipeline_with_lighting(
            device, &bind_group_layout, &lighting_layout, false,
            wgpu::CompareFunction::Always,
        ));

        let uniform_buffer =
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("MultiMesh Uniform Buffer"),
                size: std::mem::size_of::<BasicUniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

        queue.write_buffer(
            &uniform_buffer,
            0,
            bytemuck::cast_slice(&[default_uniforms]),
        );

        let bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("MultiMesh Uniform Bind Group"),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
            });

        let lighting_uniform_buffer =
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("MultiMesh Lighting Uniform Buffer"),
                size: std::mem::size_of::<BasicLightingUniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

        queue.write_buffer(
            &lighting_uniform_buffer,
            0,
            bytemuck::cast_slice(&[lighting]),
        );

        let lighting_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("MultiMesh Lighting Bind Group"),
                layout: &lighting_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource:
                        lighting_uniform_buffer.as_entire_binding(),
                }],
            });

        Self {
            pipeline,
            pipeline_no_depth,

            slots: HashMap::new(),

            uniform_buffer,
            bind_group,

            lighting_uniform_buffer,
            lighting_bind_group,

            cached_uniforms: default_uniforms,
        }
    }

    // ---------------------------------------------------------------------
    // Mesh synchronization
    // ---------------------------------------------------------------------

    /// Synchronize one logical CPU mesh with the GPU registry.
    ///
    /// This is the primary Gate 2 mesh upload API.
    ///
    /// Behavior:
    ///
    /// ```text
    /// MeshId missing
    ///     -> create GPU buffers
    ///
    /// MeshId exists, revision unchanged
    ///     -> do nothing
    ///
    /// MeshId exists, revision changed
    ///     -> recreate only that mesh's GPU buffers
    /// ```
    ///
    /// Existing visibility is preserved when geometry is updated.
    pub fn sync_mesh(
        &mut self,
        device: &Device,
        upload: MeshUpload<'_>,
    ) {
        let id = upload.id;

        match self.slots.get(&id) {
            Some(slot)
                if slot.uploaded_revision == upload.revision =>
            {
                // Geometry is already current.
                //
                // Metadata may still change independently, so update it
                // without touching GPU buffers.
                let slot = self
                    .slots
                    .get_mut(&id)
                    .expect("slot must still exist");

                slot.kind = upload.kind;
                slot.label_id = upload.label_id;
                slot.label_name =
                    upload.label_name.map(str::to_owned);

                return;
            }

            Some(_) => {
                let visible = self
                    .slots
                    .get(&id)
                    .map(|slot| slot.visible)
                    .unwrap_or(true);

                let new_slot =
                    MeshSlot::from_upload(device, upload, visible);

                self.slots.insert(id, new_slot);

                log::debug!(
                    "Re-uploaded mesh {:?} due to revision change",
                    id
                );
            }

            None => {
                let new_slot =
                    MeshSlot::from_upload(device, upload, true);

                self.slots.insert(id, new_slot);

                log::debug!(
                    "Uploaded new mesh {:?}",
                    id
                );
            }
        }
    }

    /// Synchronize a batch of meshes.
    ///
    /// This does NOT automatically remove GPU meshes that are absent from
    /// `uploads`.
    ///
    /// Use [`Self::sync_meshes_replace`] when the supplied collection should
    /// exactly replace the current registry.
    pub fn sync_meshes<'a, I>(
        &mut self,
        device: &Device,
        uploads: I,
    )
    where
        I: IntoIterator<Item = MeshUpload<'a>>,
    {
        for upload in uploads {
            self.sync_mesh(device, upload);
        }
    }

    /// Synchronize a complete mesh set.
    ///
    /// Meshes absent from `uploads` are removed from the registry.
    ///
    /// Use this when the caller's CPU mesh list represents the complete
    /// desired state.
    pub fn sync_meshes_replace<'a, I>(
        &mut self,
        device: &Device,
        uploads: I,
    )
    where
        I: IntoIterator<Item = MeshUpload<'a>>,
    {
        let uploads: Vec<MeshUpload<'a>> =
            uploads.into_iter().collect();

        let incoming_ids: HashSet<u32> =
            uploads.iter().map(|upload| upload.id).collect();

        self.slots
            .retain(|id, _| incoming_ids.contains(id));

        for upload in uploads {
            self.sync_mesh(device, upload);
        }
    }

    /// Remove one logical mesh from the GPU registry.
    ///
    /// The GPU buffers are released when the removed slot is dropped.
    pub fn remove_mesh(&mut self, id: u32) -> bool {
        self.slots.remove(&id).is_some()
    }

    /// Remove all meshes from the GPU registry.
    pub fn clear_meshes(&mut self) {
        self.slots.clear();
    }

    /// Returns whether a logical mesh currently exists.
    pub fn contains_mesh(&self, id: u32) -> bool {
        self.slots.contains_key(&id)
    }

    /// Returns the number of GPU meshes currently managed.
    pub fn mesh_count(&self) -> usize {
        self.slots.len()
    }

    /// Returns the GPU-uploaded revision for one mesh.
    pub fn uploaded_revision(&self, id: u32) -> Option<u64> {
        self.slots
            .get(&id)
            .map(|slot| slot.uploaded_revision)
    }

    /// Legacy spine/segmentation synchronization with explicit revision.
    pub fn set_meshes(&mut self, id: u32, meshkind: u32, device: &Device, meshes: &[Mesh]) {
        let kind = match meshkind {
            0 => MeshKind::Spine,
            1 => MeshKind::Needle,
            3 => MeshKind::Segmentation,
            _ => MeshKind::Other,
        };
        for mesh in meshes {
            self.sync_mesh(
                device,
                MeshUpload {
                    id,
                    kind,
                    label_id: Some(mesh.label_id),
                    label_name: Some(&mesh.label_name),
                    revision: 1 as u64,
                    mesh,
                },
            );
        }
    }

    // ---------------------------------------------------------------------
    // Visibility
    // ---------------------------------------------------------------------

    /// Set visibility for one exact logical mesh.
    pub fn set_visibility(&mut self, id: u32, visible: bool) {
        if let Some(slot) = self.slots.get_mut(&id) {
            slot.visible = visible;
        }
    }

    /// Toggle visibility for all meshes with the specified segmentation label.
    ///
    /// Generic OBJ meshes normally have `label_id == None` and are unaffected.
    pub fn set_label_visibility(&mut self, label_id: u8, visible: bool) {
        for slot in self.slots.values_mut() {
            if slot.label_id == Some(label_id) {
                slot.visible = visible;
            }
        }
    }

    /// Set visibility for all meshes of a given semantic category.
    pub fn set_kind_visibility(
        &mut self,
        kind: MeshKind,
        visible: bool,
    ) {
        for slot in self.slots.values_mut() {
            if slot.kind == kind {
                slot.visible = visible;
            }
        }
    }

    /// Returns the current visibility of one mesh.
    pub fn is_visible(&self, id: u32) -> Option<bool> {
        self.slots.get(&id).map(|slot| slot.visible)
    }

    // ---------------------------------------------------------------------
    // Uniforms
    // ---------------------------------------------------------------------

    /// Update only the model-view-projection matrix.
    ///
    /// Existing slice clipping configuration is preserved.
    pub fn update_uniforms(
        &mut self,
        queue: &Queue,
        mvp: &[[f32; 4]; 4],
    ) {
        self.cached_uniforms.model_view_proj = *mvp;

        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.cached_uniforms]),
        );
    }

    /// Configure the slice plane used to clip all meshes.
    ///
    /// The plane is expressed in `[0, 1]^3` volume-UV space.
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

    pub fn update_lighting(
        &self,
        queue: &Queue,
        lighting: BasicLightingUniforms,
    ) {
        queue.write_buffer(
            &self.lighting_uniform_buffer,
            0,
            bytemuck::cast_slice(&[lighting]),
        );
    }

    // ---------------------------------------------------------------------
    // Rendering
    // ---------------------------------------------------------------------

    /// Render all visible meshes using the depth-enabled pipeline.
    pub fn render(&self,render_pass: &mut wgpu::RenderPass<'_>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_bind_group(1, &self.lighting_bind_group, &[]);

        for slot in self.slots.values() {
            if !slot.visible || slot.num_indices == 0 {
                continue;
            }

            render_pass.set_vertex_buffer(0, slot.vertex_buffer.slice(..));
            render_pass.set_index_buffer(slot.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..slot.num_indices, 0, 0..1);
        }
    }

    /// Render all visible meshes using the pipeline without a depth attachment.
    pub fn render_no_depth(&self,render_pass: &mut wgpu::RenderPass<'_>) {
        render_pass.set_pipeline(&self.pipeline_no_depth);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_bind_group(1, &self.lighting_bind_group, &[]);

        for slot in self.slots.values() {
            if !slot.visible || slot.num_indices == 0 {
                continue;
            }

            render_pass.set_vertex_buffer(0, slot.vertex_buffer.slice(..));
            render_pass.set_index_buffer(slot.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..slot.num_indices, 0, 0..1);
        }
    }

    // ---------------------------------------------------------------------
    // Diagnostics
    // ---------------------------------------------------------------------

    /// Returns total GPU buffer usage:`(vertex_bytes, index_bytes)`
    pub fn get_memory_stats(&self) -> (u64, u64) {
        self.slots.values().fold(
            (0, 0),
            |(vertex_total, index_total), slot| {
                (
                    vertex_total + slot.vertex_buffer.size(),
                    index_total + slot.index_buffer.size(),
                )
            },
        )
    }

    /// Returns the number of visible meshes.
    pub fn visible_mesh_count(&self) -> usize {
        self.slots.values().filter(|slot| slot.visible).count()
    }

    /// Returns all currently registered mesh IDs.
    pub fn mesh_ids(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self.slots.keys().copied().collect();
        ids.sort();
        ids
    }
}
