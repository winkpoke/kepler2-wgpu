mod view;
pub use view::*;

pub use crate::rendering::mesh::mesh_view::MeshView;

mod render_context;
pub use render_context::*;

mod renderable;
pub use renderable::*;

mod layout;
pub use layout::*;

pub mod view_manager;
pub use view_manager::*;

// MIP module for Maximum Intensity Projection
pub mod mip;

// Mesh module is now always available
pub mod mesh;

// Re-exports for convenience
pub use mip::*;
pub use mesh::*;