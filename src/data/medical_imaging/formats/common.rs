/// 功能级注释：所有医学影像格式解析器的通用接口
/// 确保不同格式实现之间的一致 API
pub trait MedicalImageParser {
    /// 解析完整的医学影像文件
    async fn parse<P: AsRef<Path>>(path: P) -> MedicalImagingResult<MedicalVolume>;
    
    /// 验证文件格式而不进行完整解析
    fn validate_format<P: AsRef<Path>>(path: P) -> MedicalImagingResult<bool>;
    
    /// 提取元数据而不包含像素数据
    fn extract_metadata<P: AsRef<Path>>(path: P) -> MedicalImagingResult<ImageMetadata>;
    
    /// 返回支持的文件扩展名
    fn supported_extensions() -> &'static [&'static str];
}

/// 功能级注释：格式特定解析器的注册表
/// 支持自动格式检测和解析器选择
pub struct FormatRegistry {
    parsers: HashMap<String, Box<dyn MedicalImageParser>>,
}

impl FormatRegistry {
    /// 自动检测格式并选择合适的解析器
    pub fn parse_auto<P: AsRef<Path>>(path: P) -> MedicalImagingResult<MedicalVolume>;
    
    /// 注册新格式解析器以实现可扩展性
    pub fn register_parser<T: MedicalImageParser + 'static>(
        &mut self, 
        extensions: &[&str], 
        parser: T
    );
}