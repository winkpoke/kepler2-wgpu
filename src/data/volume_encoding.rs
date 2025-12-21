#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VolumeEncoding {
    HuPackedRg8 { offset: f32 },
    HuFloat,
}

impl VolumeEncoding {
    pub const DEFAULT_HU_OFFSET: f32 = 1100.0;
}
