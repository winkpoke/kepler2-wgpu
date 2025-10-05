#![allow(dead_code)]

#[derive(Default, Debug, Clone)]
pub struct Camera {
    pub eye: [f32; 3],
    pub center: [f32; 3],
    pub up: [f32; 3],
    pub fov_y_radians: f32,
    pub near: f32,
    pub far: f32,
}