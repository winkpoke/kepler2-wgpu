#![allow(dead_code)]

use std::collections::HashMap;

/// Minimal no-op pipeline cache for mesh rendering pipelines.
pub struct PipelineManager {
    pipelines: HashMap<String, wgpu::RenderPipeline>,
}

impl PipelineManager {
    pub fn new() -> Self {
        Self { pipelines: HashMap::new() }
    }

    /// Returns a reference to a cached pipeline by key if it exists.
    pub fn get(&self, key: &str) -> Option<&wgpu::RenderPipeline> {
        self.pipelines.get(key)
    }

    /// Inserts or replaces a pipeline under the given key.
    pub fn insert(&mut self, key: impl Into<String>, pipeline: wgpu::RenderPipeline) {
        self.pipelines.insert(key.into(), pipeline);
    }

    /// Removes a pipeline from the cache and returns it if present.
    pub fn remove(&mut self, key: &str) -> Option<wgpu::RenderPipeline> {
        self.pipelines.remove(key)
    }

    /// Clears all cached pipelines.
    pub fn clear(&mut self) {
        self.pipelines.clear();
    }

    /// Checks if a pipeline with the given key exists.
    pub fn exists(&self, key: &str) -> bool {
        self.pipelines.contains_key(key)
    }
}