#![allow(dead_code)]

/// Minimal texture pool that can hold onto a depth texture view for reuse.
use std::collections::HashMap;

pub struct TexturePool {
    depth_view: Option<wgpu::TextureView>,
    // Offscreen color views keyed by pass id (e.g., "mesh_offscreen")
    color_views: HashMap<String, wgpu::TextureView>,
}

impl TexturePool {
    pub fn new() -> Self {
        Self { depth_view: None, color_views: HashMap::new() }
    }

    /// Sets the depth view placeholder.
    pub fn set_depth_view(&mut self, view: wgpu::TextureView) {
        self.depth_view = Some(view);
    }

    /// Gets a reference to the current depth view.
    pub fn depth_view(&self) -> Option<&wgpu::TextureView> {
        self.depth_view.as_ref()
    }

    /// Clears the stored depth view.
    pub fn clear_depth_view(&mut self) {
        self.depth_view = None;
    }

    /// Inserts or replaces an offscreen color view under the given key.
    pub fn insert_color_view(&mut self, key: impl Into<String>, view: wgpu::TextureView) {
        self.color_views.insert(key.into(), view);
    }

    /// Gets a reference to an offscreen color view by key if present.
    pub fn get_color_view(&self, key: &str) -> Option<&wgpu::TextureView> {
        self.color_views.get(key)
    }

    /// Clears all stored offscreen color views.
    pub fn clear_color_views(&mut self) {
        self.color_views.clear();
    }
}