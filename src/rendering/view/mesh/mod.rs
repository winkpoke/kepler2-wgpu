#![allow(dead_code)]

// Mesh module is compiled only when the `mesh` feature is enabled.
// This module contains inert data structures without rendering logic.

pub mod basic_mesh_context;
pub mod camera;
pub mod mesh;
pub mod mesh_processing;
pub mod mesh_texture_pool;
pub mod mesh_view;
pub mod performance;

// Re-export commonly used types for easier access
pub use basic_mesh_context::BasicMeshContext;
pub use mesh_view::{MeshRenderError, MeshView};
pub use performance::QualityLevel;
