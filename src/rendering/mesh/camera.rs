#![allow(dead_code)]

use crate::core::coord::{Matrix4x4, Vector3};

#[derive(Default, Debug, Clone)]
pub struct Camera {
    pub eye: [f32; 3],
    pub center: [f32; 3],
    pub up: [f32; 3],
    pub fov_y_radians: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    /// Function-level comment: Create a new camera with default values suitable for mesh viewing
    pub fn new() -> Self {
        Self {
            eye: [0.0, 0.0, 3.0],
            center: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            fov_y_radians: std::f32::consts::PI / 4.0, // 45 degrees
            near: 0.1,
            far: 100.0,
        }
    }

    /// Function-level comment: Generate view matrix using look-at transformation
    pub fn view_matrix(&self) -> Matrix4x4<f32> {
        let eye = Vector3::new(self.eye);
        let center = Vector3::new(self.center);
        let up = Vector3::new(self.up);

        // Calculate camera coordinate system
        let forward = (center - eye).normalize();
        let right = forward.cross(up).normalize();
        let camera_up = right.cross(forward);

        // Create view matrix (inverse of camera transform)
        Matrix4x4::from_array([
            right.x(), camera_up.x(), -forward.x(), 0.0,
            right.y(), camera_up.y(), -forward.y(), 0.0,
            right.z(), camera_up.z(), -forward.z(), 0.0,
            -right.dot(eye), -camera_up.dot(eye), forward.dot(eye), 1.0,
        ])
    }

    /// Function-level comment: Generate perspective projection matrix
    pub fn projection_matrix(&self, aspect_ratio: f32) -> Matrix4x4<f32> {
        let f = 1.0 / (self.fov_y_radians / 2.0).tan();
        let range_inv = 1.0 / (self.near - self.far);

        Matrix4x4::from_array([
            f / aspect_ratio, 0.0, 0.0, 0.0,
            0.0, f, 0.0, 0.0,
            0.0, 0.0, (self.near + self.far) * range_inv, -1.0,
            0.0, 0.0, 2.0 * self.near * self.far * range_inv, 0.0,
        ])
    }

    /// Function-level comment: Generate combined view-projection matrix for efficiency
    pub fn view_projection_matrix(&self, aspect_ratio: f32) -> Matrix4x4<f32> {
        let view = self.view_matrix();
        let projection = self.projection_matrix(aspect_ratio);
        projection.multiply(&view)
    }

    /// Function-level comment: Set camera to orbit around a target point
    pub fn set_orbit(&mut self, target: [f32; 3], distance: f32, azimuth: f32, elevation: f32) {
        self.center = target;
        
        // Convert spherical coordinates to Cartesian
        let x = distance * elevation.cos() * azimuth.sin();
        let y = distance * elevation.sin();
        let z = distance * elevation.cos() * azimuth.cos();
        
        self.eye = [target[0] + x, target[1] + y, target[2] + z];
    }
}