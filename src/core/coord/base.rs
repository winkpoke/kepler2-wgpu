use core::fmt;
use glam::{Mat4, DMat4, Vec3, DVec3, Vec4, DVec4};
use super::Matrix4x4;

pub trait GeometricScalar: 
    Copy 
    + num::Zero 
    + num::One 
    + num::Signed 
    + num::Float 
    + PartialOrd 
    + std::ops::DivAssign 
    + std::ops::SubAssign 
    + std::ops::AddAssign 
    + num_traits::NumCast 
    + std::fmt::Debug
    + 'static
{
    type GlamMat: Copy + Clone + fmt::Debug + PartialEq + Default;
    
    fn identity() -> Self::GlamMat;
    fn invert(m: &Self::GlamMat) -> Option<Self::GlamMat>;
    fn multiply(a: &Self::GlamMat, b: &Self::GlamMat) -> Self::GlamMat;
    fn transform_point(m: &Self::GlamMat, p: [Self; 3]) -> [Self; 3];
    
    fn from_scale(s: [Self; 3]) -> Self::GlamMat;
    fn from_translation(t: [Self; 3]) -> Self::GlamMat;
    
    fn get_col(m: &Self::GlamMat, c: usize) -> [Self; 4];
    fn set_col_xyz(m: &mut Self::GlamMat, c: usize, xyz: [Self; 3]);
    
    fn to_matrix4x4(m: &Self::GlamMat) -> Matrix4x4<Self>;
    fn from_matrix4x4(m: &Matrix4x4<Self>) -> Self::GlamMat;
}

impl GeometricScalar for f32 {
    type GlamMat = Mat4;
    
    fn identity() -> Mat4 { Mat4::IDENTITY }
    fn invert(m: &Mat4) -> Option<Mat4> { 
        if m.determinant().abs() < 1e-6 { None } else { Some(m.inverse()) }
    }
    fn multiply(a: &Mat4, b: &Mat4) -> Mat4 { *a * *b }
    fn transform_point(m: &Mat4, p: [f32; 3]) -> [f32; 3] {
        m.transform_point3(Vec3::from(p)).into()
    }
    fn from_scale(s: [f32; 3]) -> Mat4 { Mat4::from_scale(Vec3::from(s)) }
    fn from_translation(t: [f32; 3]) -> Mat4 { Mat4::from_translation(Vec3::from(t)) }
    fn get_col(m: &Mat4, c: usize) -> [f32; 4] { m.col(c).into() }
    fn set_col_xyz(m: &mut Mat4, c: usize, xyz: [f32; 3]) {
        let mut col = m.col(c);
        col.x = xyz[0];
        col.y = xyz[1];
        col.z = xyz[2];
        *m.col_mut(c) = col;
    }
    fn to_matrix4x4(m: &Mat4) -> Matrix4x4<f32> {
        Matrix4x4 { columns: m.to_cols_array_2d() }
    }
    fn from_matrix4x4(m: &Matrix4x4<f32>) -> Mat4 {
        Mat4::from_cols_array_2d(&m.columns)
    }
}

impl GeometricScalar for f64 {
    type GlamMat = DMat4;
    
    fn identity() -> DMat4 { DMat4::IDENTITY }
    fn invert(m: &DMat4) -> Option<DMat4> {
        let det = m.determinant();
        if det.abs() < 1e-12 { None } else { Some(m.inverse()) }
    }
    fn multiply(a: &DMat4, b: &DMat4) -> DMat4 { *a * *b }
    fn transform_point(m: &DMat4, p: [f64; 3]) -> [f64; 3] {
        m.transform_point3(DVec3::from(p)).into()
    }
    fn from_scale(s: [f64; 3]) -> DMat4 { DMat4::from_scale(DVec3::from(s)) }
    fn from_translation(t: [f64; 3]) -> DMat4 { DMat4::from_translation(DVec3::from(t)) }
    fn get_col(m: &DMat4, c: usize) -> [f64; 4] { m.col(c).into() }
    fn set_col_xyz(m: &mut DMat4, c: usize, xyz: [f64; 3]) {
        let mut col = m.col(c);
        col.x = xyz[0];
        col.y = xyz[1];
        col.z = xyz[2];
        *m.col_mut(c) = col;
    }
    fn to_matrix4x4(m: &DMat4) -> Matrix4x4<f64> {
        Matrix4x4 { columns: m.to_cols_array_2d() }
    }
    fn from_matrix4x4(m: &Matrix4x4<f64>) -> DMat4 {
        DMat4::from_cols_array_2d(&m.columns)
    }
}

#[derive(Clone)]
pub struct Base<T>
where
    T: GeometricScalar,
{
    pub label: String,
    pub matrix: T::GlamMat,
}

impl<T> Base<T>
where
    T: GeometricScalar,
{
    pub fn to_base(&self, base: &Base<T>) -> Matrix4x4<T> {
        if let Some(inv) = T::invert(&base.matrix) {
            let m = T::multiply(&inv, &self.matrix);
            T::to_matrix4x4(&m)
        } else {
            // Fallback or panic? Original code panicked with unreachable!()
            // But matrix inversion might fail.
            // Let's panic to match original behavior, or maybe handle it.
            // Original: if let Some(m) = base.matrix.inv() { ... } else { unreachable!() }
            panic!("Base matrix is not invertible")
        }
    }

    /// Get matrix as a Matrix4x4<T>
    pub fn get_matrix(&self) -> Matrix4x4<T> {
        T::to_matrix4x4(&self.matrix)
    }

    pub fn get_scale_factors(&self) -> [T; 3] {
        let col0 = T::get_col(&self.matrix, 0);
        let col1 = T::get_col(&self.matrix, 1);
        let col2 = T::get_col(&self.matrix, 2);

        let sx = (col0[0] * col0[0] + col0[1] * col0[1] + col0[2] * col0[2]).sqrt();
        let sy = (col1[0] * col1[0] + col1[1] * col1[1] + col1[2] * col1[2]).sqrt();
        let sz = (col2[0] * col2[0] + col2[1] * col2[1] + col2[2] * col2[2]).sqrt();

        [sx, sy, sz]
    }

    pub fn scale(&mut self, scale: [T; 3]) {
        let s = T::from_scale(scale);
        self.matrix = T::multiply(&self.matrix, &s);
    }

    pub fn translate(&mut self, translate: [T; 3]) {
        let t = T::from_translation(translate);
        self.matrix = T::multiply(&self.matrix, &t);
    }
}

impl<T> Default for Base<T>
where
    T: GeometricScalar,
{
    fn default() -> Self {
        Self {
            label: String::from("Default"),
            matrix: T::identity(),
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Base<T> 
where
    T: GeometricScalar,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Base")
            .field("label", &self.label)
            .field("matrix", &self.matrix)
            .finish()
    }
}
