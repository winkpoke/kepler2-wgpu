//! Core utilities and foundational types
//! 
//! This module contains core utilities with minimal dependencies.

pub mod coord;
pub mod timing;
pub mod error;
pub mod geometry;
pub mod window_level;

// Re-export commonly used types
pub use coord::*;
pub use timing::*;
pub use error::*;
pub use geometry::*;
pub use window_level::*;