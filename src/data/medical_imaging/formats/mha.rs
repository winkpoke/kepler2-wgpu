/// 功能级注释：解析包含嵌入式数据的 MHA（MetaImage）文件
/// 处理带有内联图像数据的 ASCII 和二进制头文件
pub struct MhaParser {
    /// 验证 MHA 文件签名和格式
    validator: FormatValidator,
    /// 处理不同的压缩方案
    compression_handler: CompressionHandler,
    /// 管理字节序转换
    endian_converter: EndianConverter,
}

impl MhaParser {
    /// 解析完整的 MHA 文件，包括头文件和嵌入式数据
    pub async fn parse_file<P: AsRef<Path>>(path: P) -> MedicalImagingResult<MedicalVolume>;
    
    /// 从字节缓冲区解析 MHA，用于 WASM 兼容性
    pub fn parse_bytes(data: &[u8]) -> MedicalImagingResult<MedicalVolume>;
    
    /// 仅提取元数据而不加载像素数据
    pub fn parse_metadata_only<P: AsRef<Path>>(path: P) -> MedicalImagingResult<ImageMetadata>;
}