/// 功能级注释：全面的医学影像元数据
/// 保留所有空间和采集信息
#[derive(Debug, Clone, PartialEq)]
pub struct ImageMetadata {
    /// 图像尺寸 [宽度, 高度, 深度]
    pub dimensions: [usize; 3],
    /// 像素间距，单位 mm [x, y, z]
    pub spacing: [f64; 3],
    /// 世界坐标系中的图像原点 [x, y, z]
    pub origin: [f64; 3],
    /// 方向矩阵 (3x3)
    pub orientation: [[f64; 3]; 3],
    /// 像素数据类型
    pub pixel_type: PixelType,
    /// 每个像素的组件数量
    pub components: usize,
    /// 像素数据的字节序
    pub endianness: Endianness,
    /// 压缩类型（如果有）
    pub compression: Option<CompressionType>,
    /// 额外的格式特定元数据
    pub custom_fields: HashMap<String, MetadataValue>,
}

impl ImageMetadata {
    /// 验证元数据一致性
    pub fn validate(&self) -> ValidationResult;
    
    /// 计算总像素数
    pub fn total_pixels(&self) -> usize;
    
    /// 计算数据大小（字节）
    pub fn data_size_bytes(&self) -> usize;
    
    /// 将世界坐标转换为体素索引
    pub fn world_to_voxel(&self, world_pos: [f64; 3]) -> [f64; 3];
    
    /// 将体素索引转换为世界坐标
    pub fn voxel_to_world(&self, voxel_pos: [f64; 3]) -> [f64; 3];
}