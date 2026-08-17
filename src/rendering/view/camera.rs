#![allow(dead_code)]
use glam::{Mat3, Mat4, Quat, Vec2, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProjectionType {
    Perspective,
    Orthogonal,
}

impl Default for ProjectionType {
    fn default() -> Self {
        ProjectionType::Orthogonal
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

/// Unified camera for mesh + volume rendering.
///
/// Interaction model (shared by mesh raster and DVR/MIP):
///   * Rotation — orbit around `focal_point`
///   * Pan      — `pan_offset` on the view plane
///   * Zoom     — `zoom` magnification
///
/// Source of truth for orbit is always `position` + `view_up`.
/// `rotation` is derived from that pose so mesh / DVR never diverge
/// under sequential dx→dy orbits.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Vec3,
    pub focal_point: Vec3,
    pub view_up: Vec3,
    pub parallel_scale: f32,
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
    pub projection: ProjectionType,
    pub zoom: f32,
    pub pan_offset: Vec2,
    /// Model-rotation quaternion, kept in sync with pose.
    pub rotation: Quat,
}

impl Camera {
    pub const DEFAULT_DISTANCE: f32 = 5.0;

    pub fn new() -> Self {
        let mut cam = Self {
            position: Vec3::new(0.5, 0.5, 0.5 + Self::DEFAULT_DISTANCE),
            focal_point: Vec3::new(0.5, 0.5, 0.5),
            view_up: Vec3::Y,
            parallel_scale: 1.0,
            fov_y: 45.0_f32.to_radians(),
            near: 0.1,
            far: 100.0,
            projection: ProjectionType::Orthogonal,
            zoom: 1.0,
            pan_offset: Vec2::ZERO,
            rotation: Quat::IDENTITY,
        };
        cam.sync_pose_from_rotation();
        cam
    }

    // -------------------------------------------------------------------
    // Pose ↔ quaternion
    // -------------------------------------------------------------------

    /// pose ← rotation
    fn sync_pose_from_rotation(&mut self) {
        let cam_rot = self.rotation.inverse();
        let d = self.distance().max(1e-3);
        self.position = self.focal_point + cam_rot * (Vec3::Z * d);
        self.view_up = (cam_rot * Vec3::Y).normalize();
        self.orthonormalize_up();
    }

    /// rotation ← pose
    fn rebuild_rotation_from_pose(&mut self) {
        let d = self.distance().max(1e-3);
        let back = (self.position - self.focal_point) / d;
        let up = self.view_up.normalize();
        let right = up.cross(back).normalize_or_zero();
        if right.length_squared() < 1e-12 {
            return;
        }
        let up = back.cross(right).normalize();
        self.rotation = Quat::from_mat3(&Mat3::from_cols(right, up, back))
            .inverse()
            .normalize();
    }

    fn orthonormalize_up(&mut self) {
        let forward = (self.focal_point - self.position).normalize_or_zero();
        if forward.length_squared() < 1e-12 {
            return;
        }
        let right = forward.cross(self.view_up).normalize_or_zero();
        if right.length_squared() < 1e-12 {
            return;
        }
        self.view_up = right.cross(forward).normalize();
    }

    pub fn distance(&self) -> f32 {
        self.position.distance(self.focal_point)
    }

    // -------------------------------------------------------------------
    // Zoom / Pan
    // -------------------------------------------------------------------

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(0.001, 100.0);
    }

    pub fn get_zoom(&self) -> f32 {
        self.zoom
    }

    pub fn zoom_by(&mut self, factor: f32) {
        self.set_zoom(self.zoom * factor);
    }

    pub fn set_pan(&mut self, dx: f32, dy: f32) {
        const MAX_PAN: f32 = 10_000.0;
        self.pan_offset = Vec2::new(dx.clamp(-MAX_PAN, MAX_PAN), dy.clamp(-MAX_PAN, MAX_PAN));
    }

    pub fn get_pan(&self) -> Vec2 {
        self.pan_offset
    }

    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.pan_offset += Vec2::new(dx, dy);
    }

    // -------------------------------------------------------------------
    // Rotation (orbit) — pose first, then rebuild quat
    // -------------------------------------------------------------------

    fn orbit_around_axis(&mut self, axis: Vec3, angle: f32) {
        let axis = axis.normalize_or_zero();
        if axis.length_squared() < 1e-12 {
            return;
        }
        let q = Quat::from_axis_angle(axis, angle);
        self.position = self.focal_point + q * (self.position - self.focal_point);
        self.view_up = (q * self.view_up).normalize();
        self.orthonormalize_up();
        self.rebuild_rotation_from_pose();
    }

    pub fn azimuth(&mut self, angle: f32) {
        let axis = self.view_up;
        self.orbit_around_axis(axis, angle);
    }

    pub fn elevation(&mut self, angle: f32) {
        let forward = (self.focal_point - self.position).normalize();
        let right = forward.cross(self.view_up.normalize()).normalize();
        self.orbit_around_axis(right, angle);
    }

    pub fn dolly(&mut self, factor: f32) {
        let dir = (self.position - self.focal_point).normalize();
        let new_d = (self.distance() / factor).max(1e-3);
        self.position = self.focal_point + dir * new_d;
        self.rebuild_rotation_from_pose();
    }

    /// `dx` = azimuth, `dy` = elevation.
    /// Positive angles rotate the *model* CCW as seen by the user.
    pub fn orbit_angles(&mut self, dx: f32, dy: f32) {
        self.azimuth(-dx);
        self.elevation(-dy);
    }

    pub fn model_rotation_quat(&self) -> Quat {
        self.rotation
    }

    pub fn set_model_rotation_quat(&mut self, q: Quat) {
        self.rotation = q.normalize();
        self.sync_pose_from_rotation();
    }

    pub fn reset(&mut self) {
        self.rotation = Quat::IDENTITY;
        let d = self.distance().max(1e-3);
        self.position = self.focal_point + Vec3::Z * d;
        self.view_up = Vec3::Y;
        self.zoom = 1.0;
        self.pan_offset = Vec2::ZERO;
    }

    // -------------------------------------------------------------------
    // Matrices
    // -------------------------------------------------------------------

    pub fn view_matrix(&self) -> Mat4 {
        let forward = (self.focal_point - self.position).normalize();
        let right = forward.cross(self.view_up).normalize();
        let up = right.cross(forward).normalize();
        let pan_world = right * self.pan_offset.x + up * self.pan_offset.y;
        let eye = self.position - pan_world;
        let target = self.focal_point - pan_world;
        Mat4::look_at_rh(eye, target, self.view_up)
    }

    pub fn projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        let aspect = aspect_ratio.max(1e-6);
        let zoom = self.zoom.max(1e-4);
        match self.projection {
            ProjectionType::Orthogonal => {
                let half_h = (self.parallel_scale / zoom) * 0.5;
                let half_w = half_h * aspect;
                Mat4::orthographic_rh(-half_w, half_w, -half_h, half_h, self.near, self.far)
            }
            ProjectionType::Perspective => {
                let fov = (self.fov_y / zoom).clamp(0.001, std::f32::consts::PI - 0.001);
                Mat4::perspective_rh(fov, aspect, self.near, self.far)
            }
        }
    }

    pub fn view_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        self.projection_matrix(aspect_ratio) * self.view_matrix()
    }

    pub fn inverse_view_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        self.view_projection_matrix(aspect_ratio).inverse()
    }

    pub fn effective_position(&self) -> Vec3 {
        let forward = (self.focal_point - self.position).normalize();
        let right = forward.cross(self.view_up).normalize();
        let up = right.cross(forward).normalize();
        self.position - (right * self.pan_offset.x + up * self.pan_offset.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_rotation_quat_roundtrip() {
        let mut cam = Camera::new();
        let q = Quat::from_euler(glam::EulerRot::XYZ, 0.4, -0.8, 0.25).normalize();
        cam.set_model_rotation_quat(q);
        let out = cam.model_rotation_quat();
        assert!(out.abs_diff_eq(q, 1e-5), "roundtrip mismatch: {:?} vs {:?}", q, out);
    }

    #[test]
    fn test_extreme_orbit_preserves_rotation() {
        let mut cam = Camera::new();
        cam.orbit_angles(170.0_f32.to_radians(), 85.0_f32.to_radians());
        cam.orbit_angles(170.0_f32.to_radians(), -85.0_f32.to_radians());
        cam.orbit_angles(10.0_f32.to_radians(), 90.0_f32.to_radians());
        let q = cam.model_rotation_quat();
        assert!(
            !q.abs_diff_eq(Quat::IDENTITY, 1e-3),
            "rotation collapsed to identity; got {:?}",
            q,
        );
    }

    #[test]
    fn test_default_identity() {
        let cam = Camera::new();
        assert!(
            cam.model_rotation_quat().abs_diff_eq(Quat::IDENTITY, 1e-5)
                || cam.model_rotation_quat().abs_diff_eq(-Quat::IDENTITY, 1e-5)
        );
    }

    #[test]
    fn test_zoom_scales_projection() {
        let mut cam = Camera::new();
        cam.set_zoom(2.0);
        let proj = cam.projection_matrix(1.0);
        assert!((proj.col(0).x - 4.0).abs() < 1e-4);
    }

    #[test]
    fn test_pan_shifts_view() {
        let mut cam = Camera::new();
        cam.set_pan(0.25, 0.0);
        let vp = cam.view_projection_matrix(1.0);
        let ndc = vp * glam::Vec4::new(0.5, 0.5, 0.5, 1.0);
        assert!(ndc.x / ndc.w > 0.1);
    }

    #[test]
    fn test_orbit_accumulation() {
        let mut cam = Camera::new();
        cam.orbit_angles(90.0_f32.to_radians(), 0.0);
        cam.orbit_angles(90.0_f32.to_radians(), 0.0);
        let q = cam.model_rotation_quat();
        let expected = Quat::from_rotation_y(180.0_f32.to_radians());
        assert!(q.dot(expected).abs() > 0.999, "expected {:?}, got {:?}", expected, q);
    }

    #[test]
    fn test_compound_orbit_pose_quat_consistent() {
        let mut cam = Camera::new();
        cam.orbit_angles(40.0_f32.to_radians(), 0.0);
        cam.orbit_angles(0.0, 35.0_f32.to_radians());
        cam.orbit_angles(-15.0_f32.to_radians(), 20.0_f32.to_radians());

        let q = cam.model_rotation_quat();
        let d = cam.distance();
        let cam_rot = q.inverse();
        let pos_from_q = cam.focal_point + cam_rot * (Vec3::Z * d);
        let up_from_q = (cam_rot * Vec3::Y).normalize();

        assert!(
            pos_from_q.abs_diff_eq(cam.position, 1e-4),
            "pos mismatch: {:?} vs {:?}",
            pos_from_q,
            cam.position
        );
        assert!(
            up_from_q.dot(cam.view_up).abs() > 0.999,
            "up mismatch: {:?} vs {:?}",
            up_from_q,
            cam.view_up
        );
    }
}