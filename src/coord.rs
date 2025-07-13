use std::{fmt, ops::{Mul, Add, Sub, Neg, Div}};

#[derive(Copy, Clone)]
pub struct Matrix4x4<T> {
    pub data: [[T; 4]; 4], // row major
}

impl<
        T: Copy
            + num::Zero
            + num::One
            + num::Signed
            + PartialOrd
            + std::ops::DivAssign
            + std::ops::SubAssign,
    > Matrix4x4<T>
{
    pub fn from_array(data: [T; 16]) -> Self {
        Self {
            data: *slice_to_array(&data),
        }
    }

    pub fn multiply(&self, other: &Matrix4x4<T>) -> Matrix4x4<T> {
        let mut result = Matrix4x4 {
            data: [[T::zero(); 4]; 4],
        };

        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    result.data[i][j] = result.data[i][j] + (self.data[i][k] * other.data[k][j]);
                }
            }
        }

        result
    }

    pub fn inv(&self) -> Option<Matrix4x4<T>> {
        let mut augmented = [[T::zero(); 8]; 4]; // Augmented matrix [A | I]
    
        // Create the augmented matrix [A | I]
        for i in 0..4 {
            for j in 0..4 {
                augmented[i][j] = self.data[i][j];
            }
            augmented[i][i + 4] = T::one(); // Identity matrix on the right side
        }
    
        // Perform Gaussian elimination
        for i in 0..4 {
            // Step 1: Find the pivot row by looking for the largest element in the column
            let mut max_row = i;
            for k in i + 1..4 {
                if augmented[k][i].abs() > augmented[max_row][i].abs() {
                    max_row = k;
                }
            }
    
            // If pivot is zero, matrix is singular and cannot be inverted
            if augmented[max_row][i].is_zero() {
                return None; // Matrix is singular
            }
    
            // Step 2: Swap the current row with the row containing the pivot element
            augmented.swap(i, max_row);
    
            // Step 3: Scale the pivot row to make the pivot element equal to 1
            let pivot = augmented[i][i];
            for j in 0..8 {
                augmented[i][j] /= pivot;
            }
    
            // Step 4: Eliminate the other rows by making them 0 in the current column
            for k in 0..4 {
                if k != i {
                    let factor = augmented[k][i];
                    for j in 0..8 {
                        augmented[k][j] -= factor * augmented[i][j];
                    }
                }
            }
        }
    
        // Extract the right half of the augmented matrix, which is the inverse
        let mut inverse = [[T::zero(); 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                inverse[i][j] = augmented[i][j + 4];
            }
        }
    
        Some(Matrix4x4 { data: inverse })
    }
    

    pub fn apply(&self, v: &[T; 4]) -> [T; 4] {
        let mut result = [T::zero(); 4]; // Initialize result vector with zeros
    
        for i in 0..4 {
            result[i] = self.data[i][0] * v[0]
                + self.data[i][1] * v[1]
                + self.data[i][2] * v[2]
                + self.data[i][3] * v[3];
        }
    
        result
    }

    pub fn eye() -> Matrix4x4<T> {
        Self {
            data: [
                [T::one(), T::zero(), T::zero(), T::zero()],
                [T::zero(), T::one(), T::zero(), T::zero()],
                [T::zero(), T::zero(), T::one(), T::zero()],
                [T::zero(), T::zero(), T::zero(), T::one()],
            ],
        }
    }
    
     // Transpose the matrix (swap rows and columns)
     pub fn transpose(&self) -> Matrix4x4<T> {
        let mut transposed_data = [[T::zero(); 4]; 4]; // Initialize a 4x4 matrix with zeros

        // Swap rows and columns
        for i in 0..4 {
            for j in 0..4 {
                transposed_data[j][i] = self.data[i][j];
            }
        }

        Matrix4x4 {
            data: transposed_data,
        }
    }

    // Get the nth row of the matrix
    pub fn get_row(&self, n: usize) -> [T; 4] {
        self.data[n]
    }

    // Get the nth column of the matrix
    pub fn get_column(&self, n: usize) -> [T; 4] {
        [
            self.data[0][n],
            self.data[1][n],
            self.data[2][n],
            self.data[3][n],
        ]
    }
}

impl<T> Mul for Matrix4x4<T>
where
    T: Copy
        + num::Zero
        + num::One
        + num::Signed
        + PartialOrd
        + std::ops::DivAssign
        + std::ops::SubAssign,
{
    type Output = Matrix4x4<T>;
    fn mul(self, other: Matrix4x4<T>) -> Matrix4x4<T> {
        self.multiply(&other)
    }
}

impl<T: fmt::Debug> fmt::Debug for Matrix4x4<T> {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        print!("[");
        print!("{}", &format_matrix(&self.data));
        print!("]");
        println!("");
        Ok(())
    }
}

fn format_matrix<T: fmt::Debug>(matrix: &[[T; 4]; 4]) -> String {
    matrix
        .iter()
        .map(|row| format!("{:?}", row))
        .collect::<Vec<String>>()
        .join("\n ") // Join the rows with newlines
}

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
        + PartialOrd
        + std::ops::DivAssign
        + std::ops::SubAssign,
{
    pub fn to_base(&self, base: &Base<T>) -> Matrix4x4<T> {
        if let Some(m) = base.matrix.inv() {
            m.multiply(&self.matrix)
        } else {
            unreachable!()
        }
    }

    pub fn scale(&mut self, scale: T) {
        for i in 0..3 {
            for j in 0..3 {
                self.matrix.data[i][j] /= scale;
            }
        }
    }

    pub fn translate(&mut self, translate: [T; 3]) {
        for i in 0..3 {
            self.matrix.data[i][3] -= translate[i];
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

pub fn array_to_slice<T>(matrix: &[[T; 4]; 4]) -> &[T; 16] {
    // Safe to cast because we know the underlying representation is the same
    unsafe { &*(matrix as *const [[T; 4]; 4] as *const [T; 16]) }
}

pub fn slice_to_array<T>(slice: &[T; 16]) -> &[[T; 4]; 4] {
    // Safe to cast for the same reason
    unsafe { &*(slice as *const [T; 16] as *const [[T; 4]; 4]) }
}

// Vector3 type for convenience
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vector3<T> { 
    data: [T; 3],
}

impl<T> Vector3<T> {
    pub fn new(data: [T; 3]) -> Self {
        Self { data }
    }

}

// Vector3 Operations =======================================================
impl<T> Add for Vector3<T>
where
    T: Add<Output = T> + Copy,
{
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new([self.data[0] + rhs.data[0],
            self.data[1] + rhs.data[1],
            self.data[2] + rhs.data[2],])
    }
}

impl<T> Sub for Vector3<T>
where
    T: Sub<Output = T> + Copy,
{
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new([self.data[0] - rhs.data[0],
            self.data[1] - rhs.data[1],
            self.data[2] - rhs.data[2],])
    }
}

impl<T> Mul<T> for Vector3<T>
where
    T: Mul<Output = T> + Copy,
{
    type Output = Self;
    fn mul(self, scalar: T) -> Self {
        Self {
            data: [
                self.data[0] * scalar,
                self.data[1] * scalar,
                self.data[2] * scalar,
            ],
        }
    }
}

impl<T> Div<T> for Vector3<T>
where
    T: Div<Output = T> + Copy,
{
    type Output = Self;
    fn div(self, scalar: T) -> Self {
        Self {
            data: [
                self.data[0] / scalar,
                self.data[1] / scalar,
                self.data[2] / scalar,
            ],
        }
    }
}

impl<T> Neg for Vector3<T>
where
    T: Neg<Output = T> + Copy,
{
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            data: [
                -self.data[0],
                -self.data[1],
                -self.data[2],
            ],
        }
    }
}

impl<T> Vector3<T>
where
    T: Add<Output = T> + Mul<Output = T> + Sub<Output = T> + Copy,
{
    pub fn dot(self, rhs: Self) -> T {
        self.data[0] * rhs.data[0] + self.data[1] * rhs.data[1] + self.data[2] * rhs.data[2]
    }

    pub fn cross(self, rhs: Self) -> Self {
        Self {
            data: [self.data[1] * rhs.data[2] - self.data[2] * rhs.data[1],
                   self.data[2] * rhs.data[0] - self.data[0] * rhs.data[2],
                   self.data[0] * rhs.data[1] - self.data[1] * rhs.data[0],]
        }
    }

    pub fn magnitude_squared(self) -> T {
        self.dot(self)
    }
}


// impl<T> Vector3<T>
// where
//     T: Float,
// {
//     pub fn magnitude(self) -> T {
//         self.magnitude_squared().sqrt()
//     }

//     pub fn normalize(self) -> Self {
//         let mag = self.magnitude();
//         if mag < T::epsilon() {
//             Self::new(T::zero(), T::zero(), T::zero())
//         } else {
//             self * (T::one() / mag)
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_basic() {
        let m = [
            1., 0.5, 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
        ];
        let matrix = Matrix4x4::<f64>::from_array(m);
        println!("{:?}", matrix);
        println!("{:?}", matrix.apply(&[3., 2., 1., 1.]));
        let base0 = Base::<f64> {
            label: "world coordinate".to_string(),
            matrix: Matrix4x4::<f64>::eye(),
        };
        let base1 = Base::<f64> {
            label: "system coordinate".to_string(),
            matrix: matrix,
        };
        let transorm_matrix = base0.to_base(&base1);
        println!("{:?}", transorm_matrix);
    }

    #[test]
    fn test_base_nontrivial() {
        let matrix0 = Matrix4x4 {
            data: [
                [-0.51469487, 1.16777869, 0.11198701, -0.44676615],
                [-1.79107111, -1.18206274, -0.18222625, -1.25953278],
                [1.72667095, 1.85407961, 2.36366226, 1.58998366],
                [0.0, 0.0, 0.0, 1.0],
            ],
        };
        let matrix1 = Matrix4x4 {
            data: [
                [-0.53832315, 1.36244315, -0.11961783, 2.41102403],
                [1.17852419, -0.84371312, -1.13160416, -1.61392419],
                [0.00636648, -0.7648334, -0.19224463, -0.09854762],
                [0.0, 0.0, 0.0, 1.0],
            ],
        };
        println!("{:?}", matrix0);
        println!("{:?}", matrix1.apply(&[3., 2., 1., 1.]));
        let base0 = Base::<f32> {
            label: "world coordinate".to_string(),
            matrix: matrix0,
        };
        let base1 = Base::<f32> {
            label: "system coordinate".to_string(),
            matrix: matrix1,
        };
        let transorm_matrix = base0.to_base(&base1);
        println!("{:?}", transorm_matrix);
    }
}
