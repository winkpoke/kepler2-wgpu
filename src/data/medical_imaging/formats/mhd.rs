use crate::data::medical_imaging::{
    error::*, 
    get_header, 
    metadata::{MedicalVolume,  ImageMetadata, PixelData},
    ImageFormat
};
use std::{collections::HashMap, io::Read};
use std::{fs::File, io::{BufRead,BufReader}};
use std::path::PathBuf;

/// 功能级注释：解析具有独立数据文件的 MHD（MetaIO）文件
/// 处理引用外部原始或压缩数据的头文件
pub struct MhdParser {
    /// 验证 MHD 头文件格式
    validator: Option<String>,
    /// 解析相对于头文件的数据文件路径
    path_resolver: PathBuf,
    /// 处理各种数据文件格式
    data_loader: PathBuf,
}

impl MhdParser {
    /// 创建新的 MHD 解析器实例
    pub fn new(
        validator: Option<String>, 
        path_resolver: PathBuf, 
        data_loader: PathBuf
    ) -> Self {
        Self {
            validator,
            path_resolver,
            data_loader,
        }
    }
    
    /// 解析 MHD 头文件并加载关联的数据文件
    pub fn parse_file(path: PathBuf) -> MedicalImagingResult<MedicalVolume> {
        let mut mhd = MhdParser::new(None, path.clone(), path.clone());
        let metadata = mhd.parse_header()?;
        mhd.data_loader = metadata.clone().element_data_file.into();
        let pixel_data = mhd.load_data_file(&metadata)?;
        MedicalVolume::new(metadata, pixel_data, ImageFormat::MHD)
    }
    
    /// 仅解析头文件而不加载数据文件
    pub fn parse_header(&self) -> MedicalImagingResult<ImageMetadata>{
        let f = File::open(&self.path_resolver)?;
        let r = BufReader::new(f);
        let mut kv: HashMap<String, String> = HashMap::new();
        let data_offset: Option<usize> = None;

        // find header lines
            for line in r.lines() {
            let l = line?;
            let l = l.split('#').next().unwrap_or("").trim(); 
            if l.is_empty() {
                continue;
            }
            if let Some((k, v)) = l.split_once('=') {
                kv.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
        
        // analyze header key-values
        get_header(kv, data_offset)
    }
    
    /// 单独加载数据文件
    pub fn load_data_file(self, metadata: &ImageMetadata) -> MedicalImagingResult<PixelData>{
        let dims = metadata.dimensions.clone();
        let n = dims[0] * dims[1] * dims[2];
        let mut temp_buf = vec![0u8; n * 4];
        let mut f = File::open(self.data_loader.clone())?;
        f.read_exact(&mut temp_buf)?;
        Ok(PixelData::UInt8(temp_buf))
    }
}