#![allow(dead_code)]

// Mesh module is compiled only when the `mesh` feature is enabled.
// This module contains inert data structures without rendering logic.

pub mod mesh;
// pub mod material;
pub mod camera;
pub mod mesh_view;
// pub mod mesh_render_context;
pub mod basic_mesh_context;
pub mod mesh_texture_pool;
pub mod performance;
pub mod mesh_processing;

// Re-export commonly used types for easier access
pub use mesh_view::{MeshView, MeshRenderError};
pub use basic_mesh_context::BasicMeshContext;
pub use performance::QualityLevel;