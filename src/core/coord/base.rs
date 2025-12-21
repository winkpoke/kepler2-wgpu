use core::fmt;
use glam::{Mat4, Vec3};

#[derive(Clone)]
pub struct Base {
    pub label: String,
    pub matrix: Mat4,
}

impl Base {
    pub fn to_base(&self, base: &Base) -> Mat4 {
        // Original logic: base.inv() * self
        if base.matrix.determinant().abs() > 1e-6 {
            let inv = base.matrix.inverse();
            inv * self.matrix
        } else {
            panic!("Base matrix is not invertible")
        }
    }


    pub fn get_scale_factors(&self) -> [f32; 3] {
        let col0 = self.matrix.col(0);
        let col1 = self.matrix.col(1);
        let col2 = self.matrix.col(2);

        let sx = col0.truncate().length();
        let sy = col1.truncate().length();
        let sz = col2.truncate().length();

        [sx, sy, sz]
    }

    pub fn scale(&mut self, scale: [f32; 3]) {
        let s = Mat4::from_scale(Vec3::from(scale));
        self.matrix = self.matrix * s;
    }

    pub fn translate(&mut self, translate: [f32; 3]) {
        let t = Mat4::from_translation(Vec3::from(translate));
        self.matrix = self.matrix * t;
    }
}

impl Default for Base {
    fn default() -> Self {
        Self {
            label: String::from("Default"),
            matrix: Mat4::IDENTITY,
        }
    }
}

impl fmt::Debug for Base {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Base")
            .field("label", &self.label)
            .field("matrix", &self.matrix)
            .finish()
    }
}
