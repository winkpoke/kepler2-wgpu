#![allow(dead_code)]

// Mesh module is compiled only when the `mesh` feature is enabled.
// This module contains inert data structures without rendering logic.

pub mod basic_mesh_context;
pub mod mesh;
pub mod mesh_texture_pool;
pub mod mesh_view;
pub mod performance;

// Re-export commonly used types for easier access
pub use basic_mesh_context::MultiMeshContext;
pub use mesh::Mesh;
pub use mesh_view::{MeshRenderError, MeshView};
pub use performance::QualityLevel;
