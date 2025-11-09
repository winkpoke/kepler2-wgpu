//! Application layer and UI
//! 
//! This module contains application-level coordination and user interface functionality.
//! Dependencies: All other modules as needed.

// Application modules
pub mod render_app;
pub mod gl_canvas;
pub mod app_model;
pub mod appview;
pub mod app;

// Re-exports for convenience
pub use render_app::*;
pub use gl_canvas::*;
pub use app_model::*;
pub use appview::*;
pub use app::*;