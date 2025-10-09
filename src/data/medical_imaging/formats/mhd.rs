/// 功能级注释：解析具有独立数据文件的 MHD（MetaIO）文件
/// 处理引用外部原始或压缩数据的头文件
pub struct MhdParser {
    /// 验证 MHD 头文件格式
    validator: FormatValidator,
    /// 解析相对于头文件的数据文件路径
    path_resolver: PathResolver,
    /// 处理各种数据文件格式
    data_loader: DataFileLoader,
}

impl MhdParser {
    /// 解析 MHD 头文件并加载关联的数据文件
    pub async fn parse_file<P: AsRef<Path>>(header_path: P) -> MedicalImagingResult<MedicalVolume>;
    
    /// 仅解析头文件而不加载数据文件
    pub fn parse_header<P: AsRef<Path>>(path: P) -> MedicalImagingResult<ImageMetadata>;
    
    /// 单独加载数据文件（适用于延迟加载）
    pub async fn load_data_file<P: AsRef<Path>>(
        metadata: &ImageMetadata, 
        data_path: P
    ) -> MedicalImagingResult<PixelData>;
}