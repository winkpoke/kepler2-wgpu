/// Spatial information utilities for medical imaging
/// Handles coordinate transformations and spatial relationships between voxel and world coordinates

use crate::data::medical_imaging::error::{MedicalImagingError, MedicalImagingResult};
use crate::data::medical_imaging::validation::ValidationResult;


/// Spatial information container
/// Handles coordinate transformations and spatial relationships
#[derive(Debug, Clone, PartialEq)]
pub struct SpatialInfo {
    pub spacing: [f64; 3],
    pub origin: [f64; 3],
    pub orientation: [[f64; 3]; 3],
}

impl SpatialInfo {
    /// Creates spatial info with validation
    pub fn new(
        spacing: [f64; 3], 
        origin: [f64; 3], 
        orientation: [[f64; 3]; 3]
    ) -> MedicalImagingResult<Self> {
        let spatial_info = Self {
            spacing,
            origin,
            orientation,
        };
        
        spatial_info.validate_orientation()?;
        Ok(spatial_info)
    }

    /// Validates orientation matrix orthogonality
    pub fn validate_orientation(&self) -> ValidationResult{
        const TOLERANCE: f64 = 1e-6;

        // Check if matrix is orthogonal (A * A^T = I)
        for i in 0..3 {
            for j in 0..3 {
                let mut dot_product = 0.0;
                for k in 0..3 {
                    dot_product += self.orientation[i][k] * self.orientation[j][k];
                }
                
                let expected = if i == j { 1.0 } else { 0.0 };
                if (dot_product - expected).abs() > TOLERANCE {
                    return Err(MedicalImagingError::MetadataValidation {
                        field: "orientation".to_string(),
                        reason: format!(
                            "Orientation matrix is not orthogonal: dot({}, {}) = {}, expected {}",
                            i, j, dot_product, expected
                        ),
                    });
                }
            }
        }
        
        Ok(())
    }
    
    /// Calculates transformation matrix (4x4 homogeneous)
    pub fn transformation_matrix(&self) -> [[f64; 4]; 4] {
        let mut matrix = [[0.0; 4]; 4];
        
        // Fill rotation and scaling part
        for i in 0..3 {
            for j in 0..3 {
                matrix[i][j] = self.orientation[i][j] * self.spacing[j];
            }
        }
        
        // Fill translation part
        matrix[0][3] = self.origin[0];
        matrix[1][3] = self.origin[1];
        matrix[2][3] = self.origin[2];
        
        // Homogeneous coordinate
        matrix[3][3] = 1.0;
        
        matrix
    }
}