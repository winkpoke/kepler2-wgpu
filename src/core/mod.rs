//! Core utilities and foundational types
//! 
//! This module contains core utilities with minimal dependencies.

pub mod coord;
pub mod timing;
pub mod error;
pub mod geometry;

// Re-export commonly used types
pub use coord::*;
pub use timing::*;
pub use error::*;
pub use geometry::*;