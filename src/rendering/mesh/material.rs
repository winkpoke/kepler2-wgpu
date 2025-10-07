#![allow(dead_code)]

#[derive(Debug, Clone)]
pub struct Material {
    pub base_color: [f32; 4],
}

impl Default for Material {
    /// Function-level comment: Create a default material with a visible purple color.
    /// Uses a bright purple color to ensure the mesh is clearly visible during testing.
    fn default() -> Self {
        Self {
            base_color: [0.6, 0.2, 0.8, 1.0], // Purple color with full opacity
        }
    }
}