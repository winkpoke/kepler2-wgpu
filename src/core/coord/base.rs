use core::fmt;

use super::Matrix4x4;

#[derive(Clone)]
pub struct Base<T>
where
    T: Copy
        + num::Zero
        + num::One
        + num::Signed
        + PartialOrd
        + std::ops::DivAssign
        + std::ops::SubAssign,
{
    pub label: String,
    pub matrix: Matrix4x4<T>,
}
impl<T> Base<T>
where
    T: Copy
        + num::Zero
        + num::One
        + num::Signed
        + num::Float
        + PartialOrd
        + std::ops::DivAssign
        + std::ops::SubAssign
        + std::ops::AddAssign
        + num_traits::NumCast
        + std::fmt::Debug,
{
    pub fn to_base(&self, base: &Base<T>) -> Matrix4x4<T> {
        if let Some(m) = base.matrix.inv() {
            m.multiply(&self.matrix)
        } else {
            unreachable!()
        }
    }

    /// Get matrix as a Matrix4x4<T>
    pub fn get_matrix(&self) -> Matrix4x4<T> {
        self.matrix
    }

    pub fn get_scale_factors(&self) -> [T; 3] {
        let col0 = self.matrix.get_column(0);
        let col1 = self.matrix.get_column(1);
        let col2 = self.matrix.get_column(2);

        let sx = (col0[0] * col0[0] + col0[1] * col0[1] + col0[2] * col0[2]).sqrt();
        let sy = (col1[0] * col1[0] + col1[1] * col1[1] + col1[2] * col1[2]).sqrt();
        let sz = (col2[0] * col2[0] + col2[1] * col2[1] + col2[2] * col2[2]).sqrt();

        [sx, sy, sz]
    }

    pub fn scale(&mut self, scale: [T; 3]) {
        let one = T::one();
        let zero = T::zero();

        let s = Matrix4x4::from_array([one / scale[0], zero, zero, zero,
                                       zero, one / scale[1], zero, zero,
                                       zero, zero, one / scale[2], zero,
                                       zero, zero, zero, one]);
        self.matrix = self.matrix.multiply(&s);
    }

    pub fn translate(&mut self, translate: [T; 3]) {
        let one = T::one();
        let zero = T::zero();
        let t = Matrix4x4::from_array([one, zero, zero, translate[0],
                                       zero, one, zero, translate[1],
                                       zero, zero, one, translate[2],
                                       zero, zero, zero, one]);
        self.matrix = self.matrix.multiply(&t);
    }

    fn _translate_in_screen_coord(&mut self, translate: [T; 3]) {
        let mut trans = [T::one(); 4];
        for i in 0..3 {
            trans[i] = -translate[i];
        }
        let transformed = self.matrix.apply(&trans);
        for i in 0..3 {    
            self.matrix.data[i][3] = transformed[i];
        }
    }
}

impl<T> Default for Base<T>
where
    T: Copy
        + num::Zero
        + num::One
        + num::Signed
        + PartialOrd
        + std::ops::DivAssign
        + std::ops::SubAssign,
{
    fn default() -> Self {
        Self {
            label: String::from("Default"),
            matrix: Matrix4x4::eye(),
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Base<T> 
where
    T: Copy
        + num::Zero
        + num::One
        + num::Signed
        + PartialOrd
        + std::ops::DivAssign
        + std::ops::SubAssign,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Base")
            .field("label", &self.label)
            .field("matrix", &self.matrix)
            .finish()
    }
}