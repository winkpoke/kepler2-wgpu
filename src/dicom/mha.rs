use crate::coord::{Base, Matrix4x4};
use crate::ct_volume::CTVolume;
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

// ------------------------ MHD ------------------------
#[derive(Debug, Clone)]
pub enum  PatientPosition{
    HFS,
    HFP,
    FFS,
    FFP,
    HFDR,
    HFDL,
    FFDR,
    FFDL,
    Unknown,
}
#[derive(Debug, Clone)]
pub struct MHXVolume {
    pub dim: Vec<usize>,             // cols(nx), rows(ny), slices(nz)
    pub element_type: String,        // 只处理 MET_FLOAT
    pub element_data_file: String,   // ElementDataFile
    pub spacing: Vec<f32>,           // ElementSpacing = [dx, dy, dz]
    pub offset: Vec<f32>,            // Offset = [ox, oy, oz]
    pub transform: Vec<f32>,         // TransformMatrix = 6 or 9
    pub patient_position: PatientPosition,
    pub data_offset: Option<u64>,    // 如果是 .mha，这里保存数据在文件中的偏移
    pub data: Vec<u8>,
}

// ------------------------ read functions ------------------------
fn parse_ints(s: &str) -> Result<Vec<usize>> {
    s.split_whitespace()
        .map(|x| x.parse().map_err(|e| anyhow!("parse int: {}", e)))
        .collect()
}

fn parse_floats(s: &str) -> Result<Vec<f32>> {
    s.split_whitespace()
        .map(|x| x.parse().map_err(|e| anyhow!("parse float: {}", e)))
        .collect()
}

// TransformMatrix → (row_dir, col_dir, slice_dir)
pub fn orientation_dirs(transform: &[f32]) -> ([f32; 3], [f32; 3], [f32; 3]) {
    match transform.len() {
        9 => {
            let col= [transform[0], transform[1], transform[2]]; // x 轴
            let row = [transform[3], transform[4], transform[5]]; // y 轴
            let slice = [transform[6], transform[7], transform[8]]; // z 轴
            (col, row, slice)
        }
        6 => {
            let col = [transform[0], transform[1], transform[2]];
            let row = [transform[3], transform[4], transform[5]];
            let slice = [0.0, 0.0, 1.0];
            (col, row, slice)
        }
        _ => ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
    }
}

fn create_patient_position(anatomical_orientation: &str)-> PatientPosition{
    match anatomical_orientation {
        "HFS" => PatientPosition::HFS,  // Head First-Supine (头先进仰卧)
        "HFP" => PatientPosition::HFP,  // Head First-Prone (头先进俯卧) 
        "FFS" => PatientPosition::FFS,  // Feet First-Supine (脚先进仰卧)
        "FFP" => PatientPosition::FFP,  // Feet First-Prone (脚先进俯卧)
        "HFDR" => PatientPosition::HFDR, // Head First-Decubitus Right (头先进右侧卧)
        "HFDL" => PatientPosition::HFDL, // Head First-Decubitus Left (头先进左侧卧)
        "FFDR" => PatientPosition::FFDR, // Feet First-Decubitus Right (脚先进右侧卧)
        "FFDL" => PatientPosition::FFDL, // Feet First-Decubitus Left (脚先进左侧卧)
        // ========================
        // 解剖方向到标准体位的映射
        // ========================
        // 仰卧位 (Supine) - 头先进
        "RAI" => PatientPosition::HFS,  // 右前上 -> 头先进仰卧
        "LPS" => PatientPosition::HFS,  // 左后上 -> 头先进仰卧
        "LAI" => PatientPosition::HFS,  // 左前上 -> 头先进仰卧
        "RPS" => PatientPosition::HFS,  // 右后上 -> 头先进仰卧

        // 俯卧位 (Prone) - 头先进  
        "RPI" => PatientPosition::HFP,  // 右后上 -> 头先进俯卧
        "LAS" => PatientPosition::HFP,  // 左前下 -> 头先进俯卧
        "LPI" => PatientPosition::HFP,  // 左后上 -> 头先进俯卧
        "RAS" => PatientPosition::HFP,  // 右前下 -> 头先进俯卧

        // 仰卧位 (Supine) - 脚先进
        "RSA" => PatientPosition::FFS,  // 右上前 -> 脚先进仰卧
        "LSP" => PatientPosition::FFS,  // 左上后 -> 脚先进仰卧
        "LSA" => PatientPosition::FFS,  // 左上前 -> 脚先进仰卧
        "RSP" => PatientPosition::FFS,  // 右上后 -> 脚先进仰卧

        // 俯卧位 (Prone) - 脚先进
        "RPA" => PatientPosition::FFP,  // 右后前 -> 脚先进俯卧
        "LIA" => PatientPosition::FFP,  // 左下前 -> 脚先进俯卧
        "LPA" => PatientPosition::FFP,  // 左后前 -> 脚先进俯卧
        "RIA" => PatientPosition::FFP,  // 右下前 -> 脚先进俯卧

        // ========================
        // 侧卧位 (Decubitus)
        // ========================
        // 右侧卧位
        "ARI" => PatientPosition::HFDR, // 前右上 -> 头先进右侧卧
        "PRI" => PatientPosition::HFDR, // 后右上 -> 头先进右侧卧
        "ARS" => PatientPosition::FFDR, // 前右下 -> 脚先进右侧卧
        "PRS" => PatientPosition::FFDR, // 后右下 -> 脚先进右侧卧

        // 左侧卧位
        "ALI" => PatientPosition::HFDL, // 前左上 -> 头先进左侧卧
        "PLI" => PatientPosition::HFDL, // 后左上 -> 头先进左侧卧
        "ALS" => PatientPosition::FFDL, // 前左下 -> 脚先进左侧卧
        "PLS" => PatientPosition::FFDL, // 后左下 -> 脚先进左侧卧

        // ========================
        // 特殊情况
        // ========================
        "AIL" => PatientPosition::HFS,  // 前上左 -> 头先进仰卧
        "PIL" => PatientPosition::HFS,  // 后上左 -> 头先进仰卧
        "AIR" => PatientPosition::HFS,  // 前上右 -> 头先进仰卧
        "PIR" => PatientPosition::HFS,  // 后上右 -> 头先进仰卧

        // ========================
        // 默认情况
        // ========================
        _ => {
            log::info!("Unknown anatomical orientation: {}, defaulting to HFS", anatomical_orientation);
            PatientPosition::HFS
        }
    }
}

impl  MHXVolume{
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut kv: HashMap<String, String> = HashMap::new();
        let mut data_offset: Option<usize> = None;

        // find header lines
        let max_size = 64 * 1024; //最多读前 64KB，避免把整个大体积数据读进来
        let header_region = &data[..std::cmp::min(data.len(), max_size)];
        let mut cursor: usize = 0;
        for (line_no, raw_line) in header_region.split(|&b| b == b'\n').enumerate() {
            let line = std::str::from_utf8(raw_line)
                .map_err(|_| anyhow!("Header line {} is not valid UTF-8", line_no))?
                .trim();

            cursor += raw_line.len() + 1; // +1 表示 '\n'

            let l = line.split('#').next().unwrap_or("").trim();
            if l.is_empty() {
                continue;
            }

            if let Some((k, v)) = l.split_once('=') {
                let key = k.trim();
                let val = v.trim();
                kv.insert(key.to_string(), val.to_string());

                if key.eq_ignore_ascii_case("ElementDataFile") {
                    if val.eq_ignore_ascii_case("LOCAL") {
                        data_offset = Some(cursor);
                    }
                    break; // ElementDataFile 通常是最后一行
                }
            } else {
                return Err(anyhow!("Invalid line {}: {}", line_no, l));
            }
        }

        // analyze header key-values
        let dim = parse_ints(
            kv.get("DimSize").ok_or_else(|| anyhow!("Missing DimSize"))?,
        )?;
        let element_type = kv
            .get("ElementType")
            .ok_or_else(|| anyhow!("Missing ElementType"))?
            .to_string();
        let spacing = parse_floats(
            kv.get("ElementSpacing")
                .ok_or_else(|| anyhow!("Missing ElementSpacing"))?,
        )?;
        let offset = kv
            .get("Offset")
            .map(|s| parse_floats(s.as_str()))
            .transpose()?
            .unwrap_or_else(|| vec![0.0, 0.0, 0.0]);
        let transform = kv
            .get("TransformMatrix")
            .map(|s| parse_floats(s.as_str()))
            .transpose()?
            .unwrap_or_else(|| vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
        let element_data_file = kv
            .get("ElementDataFile")
            .ok_or_else(|| anyhow!("Missing ElementDataFile"))?
            .to_string();

        let anatomical_orientation = kv
            .get("AnatomicalOrientation")
            .ok_or_else(|| anyhow!("Missing AnatomicalOrientation"))?
            .to_string();

        let anatomical_orientation = anatomical_orientation.as_str();
        let patient_position = create_patient_position(anatomical_orientation);

        let data_offset_u64 = data_offset.map(|v| v as u64);

        // read voxel data
            let start = data_offset.ok_or_else(|| anyhow!("Missing data_offset"))?;
            let raw = &data[start..];
            let voxels = raw.to_vec();

        Ok(MHXVolume {
            dim,
            element_type,
            element_data_file,
            spacing,
            offset,
            transform,
            patient_position,
            data_offset: data_offset_u64,
            data: voxels,
        })
    }

    // ------------------------ read MHX ------------------------
    pub fn read_mhd_head<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let f = File::open(path).with_context(|| format!("open {:?}", path))?;
        let mut r = BufReader::new(&f);
        let mut kv = HashMap::<String, String>::new();
        let mut header_bytes: u64 = 0;
        let mut line_buf = String::new();
        let mut data_offset: Option<u64> = None;
        loop {
            line_buf.clear();
            let len = r.read_line(&mut line_buf)?;
            if len == 0 {
                break; 
            }
            header_bytes += len as u64;
            let l = line_buf.split('#').next().unwrap_or("").trim();
            if l.is_empty() {
                continue;
            }
            if let Some((k, v)) = l.split_once('=') {
                let key = k.trim();
                let val = v.trim();
                kv.insert(key.to_string(), val.to_string());
                // 如果是 .mha，ElementDataFile=LOCAL，记录偏移并停止 header 解析
                if key.eq_ignore_ascii_case("ElementDataFile") && val.eq_ignore_ascii_case("LOCAL") {
                    data_offset = Some(header_bytes);
                    break;
                }
            }
        }
        let dim = parse_ints(kv
            .get("DimSize")
            .ok_or_else(|| anyhow!("Missing DimSize"))?)?;
        let element_type = kv
            .get("ElementType")
            .ok_or_else(|| anyhow!("Missing ElementType"))?
            .to_string();
        let spacing = parse_floats(kv
            .get("ElementSpacing")
            .ok_or_else(|| anyhow!("Missing ElementSpacing"))?)?;
        let offset = kv
            .get("Offset")
            .map(|s| parse_floats(s.as_str()))
            .transpose()?
            .unwrap_or_else(|| vec![0.0, 0.0, 0.0]);
        let transform = kv
            .get("TransformMatrix")
            .map(|s| parse_floats(s.as_str()))
            .transpose()?
            .unwrap_or_else(|| vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
        let element_data_file = kv
            .get("ElementDataFile")
            .ok_or_else(|| anyhow!("Missing ElementDataFile"))?
            .to_string();
        let anatomical_orientation = kv
            .get("AnatomicalOrientation")
            .ok_or_else(|| anyhow!("Missing AnatomicalOrientation"))?
            .to_string();
        let anatomical_orientation = anatomical_orientation.as_str();
        let patient_position = create_patient_position(anatomical_orientation);

        Ok(MHXVolume {
            dim,
            element_type,
            element_data_file,
            spacing,
            offset,
            transform,
            patient_position,
            data_offset,
            data:vec![],
        })
    }

    // ------------------------ read RAW------------------------
    pub fn add_data(&self,path:PathBuf) -> Result<Vec<u8>> {
        let (nx, ny, nz) = (self.dim[0], self.dim[1], *self.dim.get(2).unwrap_or(&1));
        let n = nx * ny * nz;
        let mut buf = vec![0u8; n * 4];
        let mut f = File::open(&path)?;
        if let Some(offset) = self.data_offset {
            // .mha
            f.seek(SeekFrom::Start(offset))?;
        } else {
            // .mhd
            let raw_path = path.parent().unwrap().join(&self.element_data_file);
            f = File::open(&raw_path)?;
        }
        f.read_exact(&mut buf)?;
        
        Ok(buf)
    }

    // ------------------------ 将图像数据转换为CTVolume -----------------
    pub fn generate_ct_volume_mha(&self, slope:f32, intercept:f32) -> Result<CTVolume, String> {
        let col = self.dim[0]; // x
        let row = self.dim[1]; // y
        let depth = self.dim[2]; // z
        let data = &self.data;
        let raw = data.clone();

        let voxel_count = col * row * depth;
        let mut voxel_data = Vec::with_capacity(voxel_count);

        // analyze raw data according to ElementType
        match self.element_type.as_str() {
            "MET_SHORT" | "MET_INT16" => {
                for chunk in raw.chunks_exact(2).take(voxel_count) {
                    let val = i16::from_le_bytes([chunk[0], chunk[1]]);
                    voxel_data.push(val);
                }
            }
            "MET_FLOAT" => {
                for chunk in raw.chunks_exact(4).take(voxel_count) {
                    let val = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    let val = (val * slope + intercept).round() as i16;
                    voxel_data.push(val);
                }
            }
            other => return Err(format!("Unsupported ElementType: {}", other)),
        }

        for value in &mut voxel_data {
            if *value < -1024 {
                *value = -1024;
            }
        }

        // series
        let uid = "1.2.392.200036.9116.2.5.1.144.3437232930.1426478676.365119";

        // scaling matrix
        let scaling_matrix = Matrix4x4::from_array([
            self.spacing[0], 0.0, 0.0, 0.0,
            0.0, self.spacing[1], 0.0, 0.0,
            0.0, 0.0, self.spacing[2], 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]);

        // translation matrix
        let translation_matrix = Matrix4x4::from_array([
            1.0, 0.0, 0.0, self.offset[0],
            0.0, 1.0, 0.0, self.offset[1],
            0.0, 0.0, 1.0, self.offset[2],
            0.0, 0.0, 0.0, 1.0,
        ]);

        let direction_matrix = Matrix4x4::from_array([
            self.transform[0], self.transform[1], self.transform[2], 0.0,
            self.transform[3], self.transform[4], self.transform[5], 0.0,
            self.transform[6], self.transform[7], self.transform[8], 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]);

        // Multiply the scaling, direction, and translation matrices
        let base_matrix = direction_matrix
            .multiply(&translation_matrix)
            .multiply(&scaling_matrix);

        // Return the constructed CTVolume
        let ct_volume_mha = CTVolume {
            dimensions: (col, row, depth),
            voxel_spacing: (self.spacing[0], self.spacing[1], self.spacing[2]),
            voxel_data,
            base: Base {
                label: uid.to_string(),
                matrix: base_matrix,
            }
        };

        log::info!("{:?}", &ct_volume_mha.dimensions);
        log::info!("{:?}", &ct_volume_mha.voxel_spacing);
        log::info!("{:?}", &ct_volume_mha.base.matrix.data);
        for (index, &value) in ct_volume_mha.voxel_data.iter().enumerate() {
            if value < -1024 {
                log::info!("索引 {}: 值 {}", index, value);
            }
        }

        Ok(ct_volume_mha)
    }
}

impl fmt::Display for PatientPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            PatientPosition::HFS => "HFS",
            PatientPosition::HFP => "HFP",
            PatientPosition::FFS => "FFS",
            PatientPosition::FFP => "FFP",
            PatientPosition::HFDR => "HFDR",
            PatientPosition::HFDL => "HFDL",
            PatientPosition::FFDR => "FFDR",
            PatientPosition::FFDL => "FFDL",
            PatientPosition::Unknown => "Unknown",
        };
        write!(f, "{}", s)
    }
}

//------------------------------ test Code -------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_read_mhd() -> Result<()> {
        let path = "C:/share/input/CT.mhd";
        let header = MHXVolume::read_mhd_head(path)?;
        println!("=== MHDHeader 解析结果 ===");
        println!("维度 (DimSize): {:?}", header.dim);
        println!("体素间距 (ElementSpacing): {:?}", header.spacing);
        println!("数据类型 (ElementType): {}", header.element_type);
        println!("数据文件 (ElementDataFile): {}", header.element_data_file);
        println!("原点偏移 (Offset): {:?}", header.offset);
        println!("方向矩阵 (TransformMatrix): {:?}", header.transform);
        println!("数据偏移 (data_offset，仅 .mha 有): {:?}", header.data_offset);

        let path_raw = PathBuf::from(path);
        let data = MHXVolume::add_data(&header,path_raw)?;
        println!("前 10 个体素值: {:?}", &data[..10]);
        Ok(())
    }

    #[test]
    fn test_read_mha()-> Result<(), std::io::Error> {
        let path = "C:/share/input/CT_new.mha";
        let data = fs::read(path);
        let bytes_slice: &[u8] = data.as_ref().map(|v| v.as_slice()).unwrap();

        let header = MHXVolume::from_bytes(bytes_slice).unwrap();
        println!("=== MHDHeader 解析结果 ===");
        println!("维度 (DimSize): {:?}", header.dim);
        println!("体素间距 (ElementSpacing): {:?}", header.spacing);
        println!("数据类型 (ElementType): {}", header.element_type);
        println!("数据文件 (ElementDataFile): {}", header.element_data_file);
        println!("原点偏移 (Offset): {:?}", header.offset);
        println!("方向矩阵 (TransformMatrix): {:?}", header.transform);
        println!("患者体位：{:?}",header.patient_position);
        println!("数据偏移 (data_offset，仅 .mha 有): {:?}", header.data_offset);
        println!("前 20 个体素值: {:?}", &header.data[..20]);
        Ok(())
    }
}