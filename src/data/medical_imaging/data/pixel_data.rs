/// 功能级注释：类型安全的像素数据容器，具有高效访问能力
/// 支持医学影像中常见的各种像素类型
#[derive(Debug, Clone)]
pub enum PixelData {
    /// 8 位无符号整数（常用于分割）
    UInt8(Vec<u8>),
    /// 16 位有符号整数（常用于 CT）
    Int16(Vec<i16>),
    /// 16 位无符号整数
    UInt16(Vec<u16>),
    /// 32 位有符号整数
    Int32(Vec<i32>),
    /// 32 位浮点数（常用于处理后的数据）
    Float32(Vec<f32>),
    /// 64 位浮点数（高精度）
    Float64(Vec<f64>),
}

impl PixelData {
    /// 通过类型转换从原始字节创建像素数据
    pub fn from_bytes(
        bytes: &[u8], 
        pixel_type: PixelType, 
        endianness: Endianness
    ) -> MedicalImagingResult<Self>;
    
    /// 通过适当的缩放转换为不同的像素类型
    pub fn convert_to<T: PixelType>(&self) -> MedicalImagingResult<PixelData>;
    
    /// 获取 3D 坐标处的像素值
    pub fn get_pixel(&self, x: usize, y: usize, z: usize, dims: [usize; 3]) -> Option<f64>;
    
    /// 设置 3D 坐标处的像素值
    pub fn set_pixel(&mut self, x: usize, y: usize, z: usize, dims: [usize; 3], value: f64) -> MedicalImagingResult<()>;
    
    /// 返回原始字节表示
    pub fn as_bytes(&self) -> &[u8];
    
    /// 返回数据统计信息（最小值、最大值、均值、标准差）
    pub fn statistics(&self) -> PixelStatistics;
}