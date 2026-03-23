//! All graphics and rendering functionality
//!
//! This module contains all rendering-related functionality organized into submodules.
//! Dependencies: Core utilities and data structures.

// Rendering submodules
pub mod core;
pub mod shaders;
pub mod view;

// Re-exports for convenience
pub use core::*;
pub use view::*;
