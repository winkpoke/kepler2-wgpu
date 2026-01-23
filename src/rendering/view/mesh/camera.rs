#![allow(dead_code)]

use glam::{Mat4, Vec3};

/// Function-level comment: Projection type enumeration for camera configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProjectionType {
    /// Perspective projection with field of view - objects appear smaller with distance
    Perspective,
    /// Orthogonal projection - maintains object size regardless of distance (ideal for medical imaging)
    Orthogonal,
}

impl Default for ProjectionType {
    fn default() -> Self {
        // Default to orthogonal for medical visualization accuracy
        ProjectionType::Orthogonal
    }
}

#[derive(Default, Debug, Clone)]
pub struct Camera {
    pub eye: [f32; 3],
    pub center: [f32; 3],
    pub up: [f32; 3],
    pub fov_y_radians: f32,
    pub near: f32,
    pub far: f32,
    /// Projection type - orthogonal is preferred for medical imaging
    pub projection_type: ProjectionType,
    /// Orthogonal projection bounds - defines the visible volume for orthogonal projection
    pub ortho_left: f32,
    pub ortho_right: f32,
    pub ortho_bottom: f32,
    pub ortho_top: f32,
}

impl Camera {
    /// Function-level comment: Create a new camera with default values suitable for medical mesh viewing
    /// Uses orthogonal projection by default for accurate dimensional representation
    pub fn new() -> Self {
        Self {
            eye: [0.0, 0.0, 3.0], // Position camera closer to the cube
            center: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            fov_y_radians: std::f32::consts::PI / 4.0, // 45 degrees (used for perspective mode)
            near: 0.1,  // Adjusted near plane to be positive for perspective
            far: 100.0, // Adjusted far plane
            projection_type: ProjectionType::Orthogonal, // Default to orthogonal for medical accuracy
            // Orthogonal bounds - defines viewing volume to make cube prominent
            // Unit cube spans from -1 to +1, smaller bounds = larger cube on screen
            ortho_left: -2.5,
            ortho_right: 2.5,
            ortho_bottom: -2.5,
            ortho_top: 2.5,
        }
    }

    /// Function-level comment: Create a camera with perspective projection for traditional 3D viewing
    pub fn new_perspective() -> Self {
        let mut camera = Self::new();
        camera.projection_type = ProjectionType::Perspective;
        camera
    }

    /// Function-level comment: Set orthogonal projection bounds based on aspect ratio and zoom level
    /// This ensures the orthogonal view maintains proper proportions
    pub fn set_orthogonal_bounds(&mut self, width: f32, height: f32, zoom: f32) {
        let half_width = (width * zoom) / 2.0;
        let half_height = (height * zoom) / 2.0;

        self.ortho_left = -half_width;
        self.ortho_right = half_width;
        self.ortho_bottom = -half_height;
        self.ortho_top = half_height;
    }

    /// Function-level comment: Generate view matrix using look-at transformation
    pub fn view_matrix(&self) -> Mat4 {
        let eye = Vec3::from(self.eye);
        let center = Vec3::from(self.center);
        let up = Vec3::from(self.up);

        Mat4::look_at_rh(eye, center, up)
    }

    /// Function-level comment: Generate projection matrix based on camera projection type
    /// For medical visualization, orthogonal projection maintains accurate dimensional representation
    pub fn projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        match self.projection_type {
            ProjectionType::Perspective => self.perspective_projection_matrix(aspect_ratio),
            ProjectionType::Orthogonal => self.orthogonal_projection_matrix(aspect_ratio),
        }
    }

    /// Function-level comment: Generate perspective projection matrix for traditional 3D viewing
    fn perspective_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_rh_gl(self.fov_y_radians, aspect_ratio, self.near, self.far)
    }

    /// Function-level comment: Generate orthogonal projection matrix for medical visualization
    /// Maintains object size regardless of distance, ensuring accurate dimensional representation
    fn orthogonal_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        // Adjust orthogonal bounds to maintain aspect ratio
        let width = self.ortho_right - self.ortho_left;
        let height = self.ortho_top - self.ortho_bottom;

        let (left, right, bottom, top) = if width / height > aspect_ratio {
            // Width is constraining factor - adjust height
            let adjusted_height = width / aspect_ratio;
            let center_y = (self.ortho_top + self.ortho_bottom) / 2.0;
            let half_height = adjusted_height / 2.0;
            (
                self.ortho_left,
                self.ortho_right,
                center_y - half_height,
                center_y + half_height,
            )
        } else {
            // Height is constraining factor - adjust width
            let adjusted_width = height * aspect_ratio;
            let center_x = (self.ortho_left + self.ortho_right) / 2.0;
            let half_width = adjusted_width / 2.0;
            (
                center_x - half_width,
                center_x + half_width,
                self.ortho_bottom,
                self.ortho_top,
            )
        };

        Mat4::orthographic_rh_gl(left, right, bottom, top, self.near, self.far)
    }

    /// Function-level comment: Generate combined view-projection matrix for efficiency
    pub fn view_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        let view = self.view_matrix();
        let projection = self.projection_matrix(aspect_ratio);
        projection * view
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
