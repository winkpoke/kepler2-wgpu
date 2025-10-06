#![allow(dead_code)]

/// Minimal texture pool that can hold onto a depth texture and view for reuse.
use std::collections::HashMap;

pub struct TexturePool {
    width: u32,
    height: u32,
    /// Offscreen color views keyed by pass id (e.g., "mesh_offscreen")
    color_views: HashMap<String, wgpu::TextureView>,
    /// Offscreen color textures to keep GPU resources alive
    color_textures: HashMap<String, wgpu::Texture>,
    /// Holds the depth texture to keep GPU resource alive.
    depth_texture: Option<wgpu::Texture>,
    /// Cached view derived from the depth texture for render pass attachments.
    depth_view: Option<wgpu::TextureView>,
}

impl TexturePool {
    /// Creates a new empty texture pool.
    pub fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            depth_texture: None,
            depth_view: None,
            color_views: HashMap::new(),
            color_textures: HashMap::new(),
        }
    }

    /// Ensures that the color and depth textures are created and correctly sized.
    pub fn ensure_textures(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        use_depth: bool,
    ) {
        // Update size if it has changed
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            self.color_textures.clear(); // Re-create all textures
            self.color_views.clear();
            self.depth_texture = None; // Re-create depth texture
            self.depth_view = None;
        }

        // Ensure depth texture is created if required
        if use_depth && self.depth_texture.is_none() {
            let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("depth_texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: crate::pipeline::get_mesh_depth_format(),
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            self.depth_view = Some(depth_texture.create_view(&wgpu::TextureViewDescriptor::default()));
            self.depth_texture = Some(depth_texture);
        }
    }

    /// Sets the depth texture and its view (for backward compatibility)
    pub fn set_depth(&mut self, texture: wgpu::Texture, view: wgpu::TextureView) {
        self.depth_texture = Some(texture);
        self.depth_view = Some(view);
    }

    /// Gets a reference to the current depth view.
    pub fn depth_view(&self) -> Option<&wgpu::TextureView> {
        self.depth_view.as_ref()
    }

    /// Gets a reference to an offscreen color view by key if present.
    pub fn get_color_view(&self, key: &str) -> Option<&wgpu::TextureView> {
        self.color_views.get(key)
    }

    /// Gets both the mesh offscreen color view and depth view as owned values.
    /// This avoids borrow checker issues when both views are needed.
    pub fn get_mesh_views(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        use_depth: bool,
    ) -> (Option<wgpu::TextureView>, Option<wgpu::TextureView>) {
        // Ensure both color and depth textures exist
        self.ensure_textures(device, width, height, use_depth);
        self.ensure_mesh_offscreen_texture(device, width, height, format);
        
        // Create a new color view from the texture
        let color_view = self.color_textures.get("mesh_offscreen")
            .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));

        // Create a new depth view from the depth texture if needed
        let depth_view = if use_depth {
            self.depth_texture.as_ref()
                .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()))
        } else {
            None
        };

        (color_view, depth_view)
    }

    /// Creates or updates the mesh pass offscreen texture for the given size.
    /// Returns the texture view for use in render passes.
    pub fn ensure_mesh_offscreen_texture(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Option<&wgpu::TextureView> {
        if width == 0 || height == 0 {
            return None;
        }

        let key = "mesh_offscreen";

        // Check if we need to recreate the texture (size change or doesn't exist)
        if self.color_textures.get(key).is_none() {
            let size = wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            };

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Mesh Offscreen Color Texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let key_str: String = key.into();
            self.color_textures.insert(key_str.clone(), texture);
            self.color_views.insert(key_str, view);
        }

        self.get_color_view(key)
    }
}