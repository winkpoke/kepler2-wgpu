//! Core rendering infrastructure
//!
//! This module contains core rendering infrastructure including pipeline management,
//! texture operations, and graphics state management.

// Core rendering modules
pub mod browser_backend;
pub mod capabilities;
pub mod graphics;
pub mod pipeline;
pub mod render_pass;
pub mod texture;

// Re-exports for convenience
pub use capabilities::*;
pub use graphics::*;
pub use pipeline::*;
pub use render_pass::*;
pub use texture::*;
