/// 功能级注释：空间信息工具
/// 处理坐标变换和空间关系
#[derive(Debug, Clone, PartialEq)]
pub struct SpatialInfo {
    pub spacing: [f64; 3],
    pub origin: [f64; 3],
    pub orientation: [[f64; 3]; 3],
}

impl SpatialInfo {
    /// 创建经过验证的空间信息
    pub fn new(spacing: [f64; 3], origin: [f64; 3], orientation: [[f64; 3]; 3]) -> MedicalImagingResult<Self>;
    
    /// 验证方向矩阵的正交性
    pub fn validate_orientation(&self) -> ValidationResult;
    
    /// 计算变换矩阵
    pub fn transformation_matrix(&self) -> [[f64; 4]; 4];
}