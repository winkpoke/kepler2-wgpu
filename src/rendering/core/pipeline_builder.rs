//! PipelineBuilder: A modular builder for constructing and managing wgpu render pipelines via PipelineManager.
//!
//! This component provides:
//! - Flexible pipeline definition through a modular architecture
//! - Support for various pipeline types with configurable stages
//! - Robust error handling and validation during pipeline creation
//! - Clear interface for pipeline visualization and status tracking
//! - Efficient execution management through the PipelineManager
//!
//! It is designed to scale as new pipeline types are added.

use std::sync::Arc;

use crate::core::error::{KeplerError, KeplerResult};
use crate::rendering::core::pipeline::{PipelineKey, PipelineManager, vertex_layout_signature, default_slice_bgl_signature};

/// Enumerates supported pipeline types handled by PipelineBuilder.
/// Additional types can be added as the application grows.
#[derive(Clone, Debug)]
pub enum PipelineType {
    /// 2D texture quad pipeline used by MPR views.
    TextureQuad,
    /// Basic mesh pipeline for point/line/triangle lists. Requires Cargo feature `mesh`.
    MeshBasic,
    /// Custom pipelines defined via a signature-only scheme.
    Custom,
}

/// Parameters common to all pipelines, with optional overrides per type.
#[derive(Debug, Default)]
pub struct PipelineParams {
    /// Target color format; defaults to swapchain format when omitted.
    pub target_format: Option<wgpu::TextureFormat>,
    /// Vertex buffer layouts used by the pipeline.
    pub vertex_buffers: Option<Vec<wgpu::VertexBufferLayout<'static>>>,
    /// Bind group layouts passed to the pipeline layout. Ordering matters.
    pub bind_group_layouts: Option<Vec<wgpu::BindGroupLayout>>,
    /// Primitive topology for mesh pipelines.
    pub topology: Option<wgpu::PrimitiveTopology>,
    /// Custom signature for Custom pipelines.
    pub custom_signature: Option<String>,
}

/// A single pipeline build request combining type and parameters.
#[derive(Debug)]
pub struct PipelineRequest {
    pub ty: PipelineType,
    pub params: PipelineParams,
}

impl PipelineRequest {
    /// Validates the request and returns a descriptive error on failure.
    pub fn validate(&self) -> KeplerResult<()> {
        match self.ty {
            PipelineType::TextureQuad => {
                // Must have 3 bind group layouts (texture, vertex uniform, fragment uniform)
                if let Some(bgls) = &self.params.bind_group_layouts {
                    if bgls.len() != 3 {
                        return Err(KeplerError::Validation("TextureQuad requires 3 bind group layouts".into()));
                    }
                } else {
                    return Err(KeplerError::Validation("TextureQuad requires bind group layouts".into()));
                }
                // Vertex buffers must be provided
                if self.params.vertex_buffers.as_ref().map_or(true, |v| v.is_empty()) {
                    return Err(KeplerError::Validation("TextureQuad requires vertex buffer layouts".into()));
                }
                Ok(())
            }
            PipelineType::MeshBasic => {
                // Mesh requires topology; vertex layout provided by mesh module; no bind groups.
                if self.params.topology.is_none() {
                    return Err(KeplerError::Validation("MeshBasic requires a primitive topology".into()));
                }
                Ok(())
            }
            PipelineType::Custom => {
                if self.params.custom_signature.as_ref().map_or(true, |s| s.trim().is_empty()) {
                    return Err(KeplerError::Validation("Custom requires a non-empty signature".into()));
                }
                Ok(())
            }
        }
    }
}

/// PipelineBuilder orchestrates the creation and caching of pipelines via PipelineManager.
/// It centralizes the monitoring of hits/misses and exposes basic status interfaces.
pub struct PipelineBuilder<'a> {
    pub device: &'a wgpu::Device,
    pub manager: &'a mut PipelineManager,
}

impl<'a> PipelineBuilder<'a> {
    /// Creates a new builder bound to a device and the shared PipelineManager.
    pub fn new(device: &'a wgpu::Device, manager: &'a mut PipelineManager) -> Self { Self { device, manager } }

    /// Builds or retrieves a single pipeline according to request parameters.
    pub fn build(&mut self, req: &PipelineRequest) -> KeplerResult<Arc<wgpu::RenderPipeline>> {
        req.validate()?;
        match req.ty {
            PipelineType::TextureQuad => self.build_texture_quad(req),
            PipelineType::MeshBasic => self.build_mesh_basic(req),
            PipelineType::Custom => self.build_custom(req),
        }
    }

    /// Builds multiple pipelines, short-circuiting on the first error.
    pub fn build_many(&mut self, reqs: &[PipelineRequest]) -> KeplerResult<Vec<Arc<wgpu::RenderPipeline>>> {
        let mut out = Vec::with_capacity(reqs.len());
        for r in reqs {
            out.push(self.build(r)?);
        }
        Ok(out)
    }

    /// Returns current monitoring information from the manager for visualization.
    pub fn status(&self) -> PipelineStatus {
        PipelineStatus {
            cache_size: self.manager.cache_size(),
            hits: self.manager.hits(),
            misses: self.manager.misses(),
            keys: self.manager.keys_snapshot(),
        }
    }

    /// Internal: build TextureQuad using existing pipeline creation utilities and cache keys.
    fn build_texture_quad(&mut self, req: &PipelineRequest) -> KeplerResult<Arc<wgpu::RenderPipeline>> {
        let target_format = match req.params.target_format {
            Some(fmt) => fmt,
            None => crate::rendering::core::pipeline::get_swapchain_format().unwrap_or(wgpu::TextureFormat::Rgba8Unorm),
        };
        let bgls = req.params.bind_group_layouts.as_ref().unwrap();
        let vertex_buffers = req.params.vertex_buffers.as_ref().unwrap();

        let vertex_sig = vertex_layout_signature(vertex_buffers);
        let bgl_sig = default_slice_bgl_signature();
        let key = PipelineKey::VolumeSliceQuad { target_format, vertex_sig, bgl_sig };

        if let Some(p) = self.manager.get(&key).cloned() {
            // hit
            self.hit();
            return Ok(p);
        }

        let bgl_arr: [&wgpu::BindGroupLayout; 3] = [&bgls[0], &bgls[1], &bgls[2]];
        let pipeline = crate::rendering::core::pipeline::create_texture_quad_pipeline(self.device, bgl_arr, vertex_buffers, target_format);
        let pipeline = Arc::new(pipeline);
        self.miss();
        self.manager.insert(key, pipeline.clone());
        Ok(pipeline)
    }

    /// Internal: build MeshBasic using existing mesh utilities.
    fn build_mesh_basic(&mut self, _req: &PipelineRequest) -> KeplerResult<Arc<wgpu::RenderPipeline>> {
        let p = crate::rendering::core::pipeline::get_or_create_mesh_pipeline(self.manager, self.device);
        Ok(p)
    }

    /// Internal: build Custom using a signature-only key; the caller is responsible for creating the pipeline externally.
    /// This path allows integration of bespoke pipelines while still leveraging centralized cache and monitoring.
    fn build_custom(&mut self, req: &PipelineRequest) -> KeplerResult<Arc<wgpu::RenderPipeline>> {
        let sig = req.params.custom_signature.clone().unwrap();
        let key = PipelineKey::Custom { signature: sig };
        if let Some(p) = self.manager.get(&key).cloned() {
            self.hit();
            return Ok(p);
        }
        Err(KeplerError::Validation("Custom pipeline not found in cache; please insert manually via PipelineManager::insert".into()))
    }

    /// Marks a cache hit for monitoring.
    fn hit(&mut self) {
        self.manager.record_hit();
        log::trace!("PipelineBuilder hit. Hits={}, Misses={}, Size={}", self.manager.hits(), self.manager.misses(), self.manager.cache_size());
    }

    /// Marks a cache miss for monitoring.
    fn miss(&mut self) {
        self.manager.record_miss();
        log::trace!("PipelineBuilder miss. Hits={}, Misses={}, Size={}", self.manager.hits(), self.manager.misses(), self.manager.cache_size());
    }
}

/// Status view for visualization and tracking.
#[derive(Clone, Debug)]
pub struct PipelineStatus {
    pub cache_size: usize,
    pub hits: usize,
    pub misses: usize,
    pub keys: Vec<PipelineKey>,
}