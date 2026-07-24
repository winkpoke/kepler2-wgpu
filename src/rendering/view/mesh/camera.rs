#![allow(dead_code)]
use glam::{Mat4, Quat, Vec3};

/// Unified camera for mesh + volume rendering.
#[repr(C)]
#[derive(Default, Debug, Clone)]
pub struct Camera {
    /// VTK Position
    pub position: Vec3,
    /// VTK FocalPoint
    pub focal_point: Vec3,
    /// VTK ViewUp
    pub view_up: Vec3,
    /// Perspective / Orthographic
    pub parallel_scale: f32,
    /// Perspective
    pub distance: f32,
    /// FOV
    pub fov_y: f32,
}

/// Baseline eye-to-target distance before zoom is applied.
const BASE_ORBIT_DISTANCE: f32 = 5.0;
/// Shared perspective field-of-view used to keep orthographic and perspective
/// sizing stable when switching modes.
const PERSPECTIVE_FOV_Y_RADIANS: f32 = 45.0_f32.to_radians();
/// Extra depth range added around the camera-target distance so the whole unit
/// cube stays visible under large zoom factors.
const DEPTH_MARGIN: f32 = 4.0;

impl Camera {
    /// Function-level comment: Create a default shared camera.
    pub fn new() -> Self {
        Self {
            position: Vec3::INFINITY,
            focal_point: Vec3::INFINITY,
            view_up: Vec3::INFINITY,
            parallel_scale: 1.0,
            distance: 0.0,
            fov_y: 0.0,
        }
    }

    /// Function-level comment: Get the current orbit target in the shared world cube.
    pub fn target(&self) -> Vec3 {
        Self::scene_center() + self.pan_offset()
    }

    /// Function-level comment: Compute the world-space eye position for the current
    /// orbit + pan. The eye orbits the shared scene center and translates together
    /// with the target so panning remains projection-independent.
    pub fn eye(&self) -> Vec3 {
        let offset = self.rotation * Vec3::new(0.0, 0.0, self.distance_to_target());
        self.target() + offset
    }

    /// Function-level comment: View matrix for the orbit camera. Built via
    /// look_at_rh so the world remains a fixed `[0, 1]^3` cube regardless of
    /// rotation/pan/zoom.
    pub fn view_matrix(&self)->Mat4{
        Mat4::look_at_rh(
            self.position,
            self.focal_point,
            self.view_up,
        )
    }

    pub fn azimuth(&mut self, angle: f32){
        let axis = self.view_up.normalize();
        let q = Quat::from_axis_angle(axis, angle);
        let offset = self.position - self.focal_point;
        self.position = self.focal_point + q * offset;
    }

    pub fn elevation(&mut self, angle:f32){
        let forward = (self.focal_point-self.position).normalize();
        let right = forward.cross(self.view_up).normalize();
        let q = Quat::from_axis_angle(right, angle);
        let offset = self.position-self.focal_point;
        self.position = self.focal_point+q * offset;
        self.view_up = (q*self.view_up).normalize();
    }

    pub fn dolly(&mut self, factor:f32){
        let dir = (self.position-self.focal_point).normalize();
        self.distance /= factor;
        self.position= self.focal_point+dir*self.distance;
    }

    pub fn pan(&mut self,dx:f32,dy:f32){
        let forward= (self.focal_point-self.position).normalize();
        let right= forward.cross(self.view_up).normalize();
        let up= self.view_up.normalize();
        let delta= right*dx+ up*dy;
        self.position += delta;
        self.focal_point += delta;

    }

    /// Function-level comment: Projection matrix for the shared camera.
    ///
    /// Orthographic mode derives its frustum width/height from the same
    /// camera-target distance used by perspective mode, so toggling projection
    /// type preserves the target-plane scale as closely as possible.
    /// `aspect_ratio` is the viewport width / height.
    pub fn projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        let safe_aspect_ratio = aspect_ratio.max(1e-6);
        let distance = self.distance_to_target();
        let near_plane = (distance - DEPTH_MARGIN).max(0.01);
        let far_plane = distance + DEPTH_MARGIN;
        let half_h = distance * (PERSPECTIVE_FOV_Y_RADIANS * 0.5).tan();
        let half_w = half_h * safe_aspect_ratio;
        Mat4::orthographic_rh(-half_w, half_w, -half_h, half_h, near_plane, far_plane)
    }
    

    /// Function-level comment: Combined view-projection matrix.
    pub fn view_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        self.projection_matrix(aspect_ratio) * self.view_matrix()
    }

    /// Function-level comment: Inverse of the combined view-projection matrix.
    /// Use this in the volume shader to unproject NDC points back into world space.
    pub fn inverse_view_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        self.view_projection_matrix(aspect_ratio).inverse()
    }

    /// Function-level comment: Compute the current pan offset in world space.
    fn pan_offset(&self) -> Vec3 {
        let right = self.rotation * Vec3::X;
        let up = self.rotation * Vec3::Y;
        right * self.pan[0] + up * self.pan[1]
    }

    /// Function-level comment: Return the shared scene center used by mesh and volume.
    fn scene_center() -> Vec3 {
        Vec3::new(0.5, 0.5, 0.5)
    }

    /// Function-level comment: Convert the zoom factor into a camera-target distance.
    fn distance_to_target(&self) -> f32 {
        BASE_ORBIT_DISTANCE / self.scale.max(0.05)
    }
}
