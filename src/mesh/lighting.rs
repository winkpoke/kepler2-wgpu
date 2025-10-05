#![allow(dead_code)]

#[derive(Default, Debug, Clone)]
pub struct Lighting {
    pub direction: [f32; 3],
    pub intensity: f32,
}