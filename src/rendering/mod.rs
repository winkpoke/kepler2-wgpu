//! All graphics and rendering functionality
//! 
//! This module contains all rendering-related functionality organized into submodules.
//! Dependencies: Core utilities and data structures.

// Rendering submodules
pub mod core;
pub mod content;
pub mod view;
pub mod shaders;

// Mesh module is now always available
pub mod mesh;

// MIP module for Maximum Intensity Projection
pub mod mip;

// Re-exports for convenience
pub use core::*;
pub use content::*;
pub use view::*;
pub use mesh::*;
pub use mip::*;