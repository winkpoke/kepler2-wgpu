#![allow(dead_code)]

mod view;
pub use view::*;

mod view_factory;
pub use view_factory::*;

pub use crate::rendering::mesh::mesh_view::MeshView;

mod renderable;
pub use renderable::*;

mod layout;
pub use layout::*;

pub mod view_manager;
pub use view_manager::*;

// MIP module for Maximum Intensity Projection
pub mod mip;

// Mesh module is now always available
pub mod mesh;

// Shared camera (single source of truth for rotation/pan/zoom across views)
pub mod camera;

// MPR module for Multi-Planar Reconstruction
pub mod mpr;

// Render content management
pub mod render_content;

// Re-exports for convenience
pub use mesh::*;
pub use mip::*;
pub use mpr::*;
pub use render_content::*;

pub const LABEL_NAMES: [&str; 8] = [
    "",       // index 0 unused
    "L1",
    "L2",
    "L3",
    "L4",
    "L5",
    "S1",
    "Sacrum",
];

// Colour palette for the segmentation overlay in the MPR / MIP / 3D views.
pub const LABEL_COLORS: [[f32; 4]; 8] = [
    [0.00, 0.00, 0.00, 0.0], // 0  Background
    [0.95, 0.20, 0.20, 1.0], // 1  L1      (red)
    [0.98, 0.55, 0.10, 1.0], // 2  L2      (orange)
    [0.95, 0.90, 0.20, 1.0], // 3  L3      (yellow)
    [0.20, 0.85, 0.30, 1.0], // 4  L4      (green)
    [0.20, 0.70, 0.95, 1.0], // 5  L5      (cyan)
    [0.30, 0.30, 0.95, 1.0], // 6  S1      (blue)
    [0.85, 0.20, 0.80, 1.0], // 7  sacrum  (magenta)
];

// Needle module
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct NeedleUniform {
    pub entry: [f32; 3],
    pub radius: f32,
    pub tip: [f32; 3],
    pub id: u32,
    pub color: [f32; 4],
}

impl Default for NeedleUniform {
    fn default() -> Self {
        Self {
            entry: [0.0; 3],
            radius: 0.0,
            tip: [0.0; 3],
            id: 0,
            color: [0.0; 4],
        }
    }
}

impl NeedleUniform {
    pub fn plane_from_needle(&self, angle_rad: f32) -> (glam::Vec3, glam::Vec3) {
        let entry = glam::Vec3::from(self.entry);
        let tip = glam::Vec3::from(self.tip);
        let axis = (tip - entry).normalize_or_zero();
        if axis == glam::Vec3::ZERO {
            return (glam::Vec3::ZERO, tip);
        }
        let ref_vec = glam::Quat::from_axis_angle(axis, angle_rad) * glam::Vec3::new(1.0, 0.0, 0.0);
        let normal = axis.cross(ref_vec).normalize_or_zero();
        (normal, tip)
    }
}

// Oblique plane module
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ObliquePlaneUniform {
    pub center: [f32; 3],
    pub visible: f32,
    pub normal: [f32; 3],
    pub plane_alpha: f32,
}

impl Default for ObliquePlaneUniform {
    fn default() -> Self {
        Self {
            center: [0.5; 3],
            visible: 0.0,
            normal: [0.0, 0.0, 1.0],
            plane_alpha: 0.0,
        }
    }
}