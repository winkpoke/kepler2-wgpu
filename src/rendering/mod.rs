//! All graphics and rendering functionality
//! 
//! This module contains all rendering-related functionality organized into submodules.
//! Dependencies: Core utilities and data structures.

// Rendering submodules
pub mod core;
pub mod content;
pub mod view;
pub mod shaders;

// Feature-gated mesh module
#[cfg(feature = "mesh")]
pub mod mesh;

// Re-exports for convenience
pub use core::*;
pub use content::*;
pub use view::*;

#[cfg(feature = "mesh")]
pub use mesh::*;