/// 功能级注释：医学影像操作的全面错误类型
/// 为调试和用户反馈提供详细的错误信息
#[derive(Debug, thiserror::Error)]
pub enum MedicalImagingError {
    /// 文件格式不支持
    #[error("不支持的文件格式: {format}")]
    UnsupportedFormat { format: String },
    
    /// 无效的文件头
    #[error("无效的文件头: {reason}")]
    InvalidHeader { reason: String },
    
    /// 元数据验证失败
    #[error("元数据验证失败: {field} - {reason}")]
    MetadataValidation { field: String, reason: String },
    
    /// 检测到像素数据损坏
    #[error("像素数据损坏: 预期 {expected} 字节，实际 {actual}")]
    DataCorruption { expected: usize, actual: usize },
    
    /// 压缩/解压缩错误
    #[error("压缩错误: {algorithm} - {reason}")]
    CompressionError { algorithm: String, reason: String },
    
    /// 字节序转换错误
    #[error("字节序转换失败: {reason}")]
    EndiannessError { reason: String },
    
    /// 文件 I/O 错误
    #[error("文件 I/O 错误: {0}")]
    Io(#[from] std::io::Error),
    
    /// 内存分配错误
    #[error("内存分配失败: 请求 {size} 字节")]
    MemoryAllocation { size: usize },
    
    /// 数据类型转换错误
    #[error("数据类型转换失败: 从 {from} 到 {to}")]
    TypeConversion { from: String, to: String },
    
    /// 验证错误
    #[error("验证失败: {0}")]
    Validation(String),
}

/// 医学影像操作的结果类型别名
pub type MedicalImagingResult<T> = Result<T, MedicalImagingError>;

/// 功能级注释：用于详细诊断的错误上下文
/// 为错误分析和调试提供额外的上下文
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub file_path: Option<PathBuf>,
    pub operation: String,
    pub line_number: Option<usize>,
    pub additional_info: HashMap<String, String>,
}

impl ErrorContext {
    /// 创建新的错误上下文
    pub fn new(operation: &str) -> Self;
    
    /// 添加文件路径上下文
    pub fn with_file<P: AsRef<Path>>(mut self, path: P) -> Self;
    
    /// 添加行号上下文
    pub fn with_line(mut self, line: usize) -> Self;
    
    /// 添加上下文信息
    pub fn with_info(mut self, key: &str, value: &str) -> Self;
}