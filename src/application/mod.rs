//! Application layer and UI
//!
//! This module contains application-level coordination and user interface functionality.
//! Dependencies: All other modules as needed.

// Application modules
pub mod app;
pub mod app_model;
pub mod appview;
pub mod gl_canvas;
pub mod render_app;

// Re-exports for convenience
pub use app::*;
pub use app_model::*;
pub use appview::*;
pub use gl_canvas::*;
pub use render_app::*;
