#![allow(dead_code)]

// Mesh module is compiled only when the `mesh` feature is enabled.
// This module contains inert data structures without rendering logic.

pub mod mesh;
pub mod material;
pub mod camera;
pub mod lighting;
pub mod mesh_view;
pub mod mesh_render_context;
pub mod pipeline_manager;
pub mod texture_pool;