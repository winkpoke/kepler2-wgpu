#![allow(dead_code)]

// Mesh module is compiled only when the `mesh` feature is enabled.
// This module contains inert data structures without rendering logic.

pub mod mesh;
pub mod material;
pub mod camera;
pub mod lighting;
pub mod mesh_view;
pub mod mesh_render_context;
pub mod basic_mesh_context;
pub mod texture_pool;
pub mod shader_validation;
pub mod performance;

// Re-export commonly used types for easier access
pub use mesh_view::{MeshView, MeshRenderError};
pub use basic_mesh_context::BasicMeshContext;
pub use shader_validation::ShaderValidationError;
pub use performance::QualityLevel;