mod base;
pub use base::*;


use std::{fmt, ops::{Add, Div, Index, Mul, Neg, Sub}};
use num::Float;


// Vector3 type for convenience
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vector3<T> { 
    data: [T; 3],
}

impl<T> Vector3<T> {
    pub fn new(data: [T; 3]) -> Self {
        Self { data }
    }

    /// Function-level comment: Get the x component of the vector
    pub fn x(&self) -> T where T: Copy {
        self.data[0]
    }

    /// Function-level comment: Get the y component of the vector
    pub fn y(&self) -> T where T: Copy {
        self.data[1]
    }

    /// Function-level comment: Get the z component of the vector
    pub fn z(&self) -> T where T: Copy {
        self.data[2]
    }

    /// Function-level comment: Get the raw data array
    pub fn as_array(&self) -> &[T; 3] {
        &self.data
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

impl<T> Vector3<T>
where
    T: Float,
{
    pub fn magnitude(self) -> T {
        self.magnitude_squared().sqrt()
    }

    pub fn normalize(self) -> Self {
        let mag = self.magnitude();
        if mag < T::epsilon() {
            Self::new([T::zero(), T::zero(), T::zero()])
        } else {
            self * (T::one() / mag)
        }
    }
}

/// A generic 4x4 matrix struct, stored in **row-major** order.
#[derive( Copy, Clone)]
pub struct Matrix4x4<T> {
    pub data: [[T; 4]; 4], // Each row is a [T; 4] array
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
    /// Creates a 4x4 matrix from a flat array of 16 elements (row-major order).
    pub fn from_array(data: [T; 16]) -> Self {
        Self {
            data: *slice_to_array(&data),
        }
    }

    /// Multiplies two 4x4 matrices and returns the resulting matrix.
    pub fn multiply(&self, other: &Matrix4x4<T>) -> Matrix4x4<T> {
        let mut result = Matrix4x4 {
            data: [[T::zero(); 4]; 4],
        };

        // Standard triple-nested loop for matrix multiplication
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    result.data[i][j] = result.data[i][j] + (self.data[i][k] * other.data[k][j]);
                }
            }
        }

        result
    }

    /// Multiplies a 3D point by the matrix.
    pub fn multiply_point3(&self, point: [T; 3]) -> [T; 3] {
        // Convert the 3D point to homogeneous coordinates
        let homogenous_point = [point[0], point[1], point[2], T::one()];
        let mut result = [T::zero(); 4];
        for i in 0..4 {
            for j in 0..4 {
                result[i] = result[i] + (self.data[i][j] * homogenous_point[j]);
            }
        }
        for i in 0..3 {
            result[i] = result[i] / result[3]; // Convert back to Cartesian coordinates
        }
        result[0..3].try_into().unwrap()
    }

    /// Computes the inverse of the matrix using Gaussian elimination.
    /// Returns `None` if the matrix is singular (non-invertible).
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
    
    /// Applies the matrix to a 4D vector and returns the transformed vector.
    pub fn apply(&self, v: &[T; 4]) -> [T; 4] {
        let mut result = [T::zero(); 4]; // Initialize result vector with zeros
    
        // Perform matrix-vector multiplication
        for i in 0..4 {
            result[i] = self.data[i][0] * v[0]
                + self.data[i][1] * v[1]
                + self.data[i][2] * v[2]
                + self.data[i][3] * v[3];
        }
    
        result
    }

    /// Returns a 4x4 identity matrix.
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
    
    /// Returns the transpose of the matrix (swap rows and columns).
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

    /// Returns the nth row of the matrix as a 4-element array.
    pub fn get_row(&self, n: usize) -> [T; 4] {
        self.data[n]
    }

    /// Returns the nth column of the matrix as a 4-element array.
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

pub fn array_to_slice<T>(matrix: &[[T; 4]; 4]) -> &[T; 16] {
    // Safe to cast because we know the underlying representation is the same
    unsafe { &*(matrix as *const [[T; 4]; 4] as *const [T; 16]) }
}

pub fn slice_to_array<T>(slice: &[T; 16]) -> &[[T; 4]; 4] {
    // Safe to cast for the same reason
    unsafe { &*(slice as *const [T; 16] as *const [[T; 4]; 4]) }
}

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

    #[test]
    fn test_matrix_inv() {
        // Test 1: Identity matrix inverse should be itself
        let identity = Matrix4x4::<f64>::eye();
        let identity_inv = identity.inv().expect("Identity matrix should be invertible");
        
        // Check if A * A^-1 = I
        let result = identity.multiply(&identity_inv);
        for i in 0..4 {
            for j in 0..4 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((result.data[i][j] - expected).abs() < 1e-10, 
                    "Identity matrix inverse failed at [{}, {}]: expected {}, got {}", 
                    i, j, expected, result.data[i][j]);
            }
        }

        // Test 2: General invertible matrix
        let matrix = Matrix4x4::<f64>::from_array([
            2.0, 1.0, 0.0, 1.0,
            1.0, 2.0, 1.0, 0.0,
            0.0, 1.0, 2.0, 1.0,
            0.0, 0.0, 0.0, 1.0,
        ]);
        
        let matrix_inv = matrix.inv().expect("Matrix should be invertible");
        
        // Check if A * A^-1 = I
        let result = matrix.multiply(&matrix_inv);
        for i in 0..4 {
            for j in 0..4 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((result.data[i][j] - expected).abs() < 1e-10, 
                    "Matrix inverse failed at [{}, {}]: expected {}, got {}", 
                    i, j, expected, result.data[i][j]);
            }
        }

        // Check if A^-1 * A = I
        let result2 = matrix_inv.multiply(&matrix);
        for i in 0..4 {
            for j in 0..4 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((result2.data[i][j] - expected).abs() < 1e-10, 
                    "Matrix inverse failed (reverse multiplication) at [{}, {}]: expected {}, got {}", 
                    i, j, expected, result2.data[i][j]);
            }
        }

        // Test 3: Singular matrix (should return None)
        let singular_matrix = Matrix4x4::<f64>::from_array([
            1.0, 2.0, 3.0, 4.0,
            2.0, 4.0, 6.0, 8.0,  // This row is 2x the first row
            5.0, 6.0, 7.0, 8.0,
            9.0, 10.0, 11.0, 12.0,
        ]);
        
        assert!(singular_matrix.inv().is_none(), "Singular matrix should not be invertible");

        // Test 4: Translation matrix
        let translation = Matrix4x4::<f64>::from_array([
            1.0, 0.0, 0.0, 5.0,
            0.0, 1.0, 0.0, 3.0,
            0.0, 0.0, 1.0, 2.0,
            0.0, 0.0, 0.0, 1.0,
        ]);
        
        let translation_inv = translation.inv().expect("Translation matrix should be invertible");
        
        // The inverse of a translation matrix should have negated translation components
        let expected_inv = Matrix4x4::<f64>::from_array([
            1.0, 0.0, 0.0, -5.0,
            0.0, 1.0, 0.0, -3.0,
            0.0, 0.0, 1.0, -2.0,
            0.0, 0.0, 0.0, 1.0,
        ]);
        
        for i in 0..4 {
            for j in 0..4 {
                assert!((translation_inv.data[i][j] - expected_inv.data[i][j]).abs() < 1e-10,
                    "Translation matrix inverse failed at [{}, {}]: expected {}, got {}",
                    i, j, expected_inv.data[i][j], translation_inv.data[i][j]);
            }
        }

        // Test 5: Scale matrix
        let scale = Matrix4x4::<f64>::from_array([
            2.0, 0.0, 0.0, 0.0,
            0.0, 3.0, 0.0, 0.0,
            0.0, 0.0, 4.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]);
        
        let scale_inv = scale.inv().expect("Scale matrix should be invertible");
        
        // The inverse of a scale matrix should have reciprocal scale factors
        let expected_scale_inv = Matrix4x4::<f64>::from_array([
            0.5, 0.0, 0.0, 0.0,
            0.0, 1.0/3.0, 0.0, 0.0,
            0.0, 0.0, 0.25, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]);
        
        for i in 0..4 {
            for j in 0..4 {
                assert!((scale_inv.data[i][j] - expected_scale_inv.data[i][j]).abs() < 1e-10,
                    "Scale matrix inverse failed at [{}, {}]: expected {}, got {}",
                    i, j, expected_scale_inv.data[i][j], scale_inv.data[i][j]);
            }
        }
    }
}
