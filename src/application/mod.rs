//! Application layer and UI
//! 
//! This module contains application-level coordination and user interface functionality.
//! Dependencies: All other modules as needed.

// Application modules
pub mod render_app;
pub mod gl_canvas;

// Re-exports for convenience
pub use render_app::*;
pub use gl_canvas::*;