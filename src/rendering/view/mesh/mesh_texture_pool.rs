/// Attachment compatibility within a single device's graphics context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DepthKey {
    width: u32,
    height: u32,
    sample_count: u32,
    format: wgpu::TextureFormat,
}

struct CachedDepth {
    key: DepthKey,
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

/// A bounded, device-scoped cache for the mesh pass's full-surface depth attachment.
/// Discard the pool when replacing the device. Mesh color is rendered directly to the surface.
#[derive(Default)]
pub struct MeshTexturePool {
    depth: Option<CachedDepth>,
    allocations: u64,
}

impl MeshTexturePool {
    /// Creates an empty pool belonging to the current graphics device.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reuses both texture and view until the attachment key changes.
    /// Zero-sized requests do not allocate or return an attachment.
    pub fn ensure_depth(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        sample_count: u32,
        format: wgpu::TextureFormat,
    ) -> Option<&wgpu::TextureView> {
        if width == 0 || height == 0 {
            return None;
        }
        let key = DepthKey {
            width,
            height,
            sample_count,
            format,
        };
        if self.depth.as_ref().map(|depth| depth.key) != Some(key) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Mesh Cached Depth"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            self.depth = Some(CachedDepth {
                key,
                _texture: texture,
                view,
            });
            self.allocations += 1;
        }
        self.depth_view()
    }

    /// Compatibility helper for single-sample mesh depth preparation.
    pub fn ensure_textures(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        use_depth: bool,
    ) {
        if use_depth {
            self.ensure_depth(
                device,
                width,
                height,
                1,
                crate::rendering::core::pipeline::get_mesh_depth_format(),
            );
        }
    }

    /// Adopts an externally created depth attachment on the pool's device.
    pub fn set_depth(&mut self, texture: wgpu::Texture, view: wgpu::TextureView) {
        let key = DepthKey {
            width: texture.width(),
            height: texture.height(),
            sample_count: texture.sample_count(),
            format: texture.format(),
        };
        self.depth = Some(CachedDepth {
            key,
            _texture: texture,
            view,
        });
    }

    /// Cached attachment view; borrowing this does not create another GPU object.
    pub fn depth_view(&self) -> Option<&wgpu::TextureView> {
        self.depth.as_ref().map(|depth| &depth.view)
    }

    /// Number of depth texture/view pairs allocated by this pool, for diagnostics.
    pub fn allocation_count(&self) -> u64 {
        self.allocations
    }

    /// Releases the cached attachment, including when no mesh views remain.
    pub fn clear_depth_view(&mut self) {
        self.depth = None;
    }
}
