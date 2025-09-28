use crate::coord::{Base, Matrix4x4};
use crate::ct_volume::CTVolume;

use anyhow::{anyhow, Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

// ------------------------ MHD Header ------------------------
#[derive(Debug, Clone)]
pub struct MHDHeader {
    pub dim: Vec<usize>,             // cols(nx), rows(ny), slices(nz)
    pub element_type: String,        // 只处理 MET_FLOAT
    pub element_data_file: String,   // ElementDataFile
    pub spacing: Vec<f32>,           // ElementSpacing = [dx, dy, dz]
    pub offset: Vec<f32>,            // Offset = [ox, oy, oz]
    pub transform: Vec<f32>,         // TransformMatrix = 6 or 9
    pub patient_position: String,
    pub data_offset: Option<u64>,    // 如果是 .mha，这里保存数据在文件中的偏移
    pub data:Vec<i16>,
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
            let col= [transform[0], transform[3], transform[6]]; // x 轴
            let row = [transform[1], transform[4], transform[7]]; // y 轴
            let slice = [transform[2], transform[5], transform[8]]; // z 轴
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

impl  MHDHeader{
    pub fn from_bytes(data: &[u8], slope:f32, intercept: f32) -> Result<Self> {
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
        log::info!("Offset: {:?}", offset);
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
        
        let patient_position: String = match anatomical_orientation.as_str() {
            "IRP" => "HFS".to_string(),
            "ILP" => "HFP".to_string(),
            "SRP" => "FFS".to_string(),
            "SLP" => "FFP".to_string(),
            _ => "UNKNOWN".to_string(), // Default case for unhandled orientations
        };

        let data_offset_u64 = data_offset.map(|v| v as u64);

        // read voxel data
        let (nx, ny, nz) = (dim[0], dim[1], *dim.get(2).unwrap_or(&1));
        let voxel_count = nx * ny * nz;

        let start = data_offset.ok_or_else(|| anyhow!("Missing data_offset"))?;
        let raw = &data[start..];

        let mut voxels = Vec::with_capacity(voxel_count);

        // analyze raw data according to ElementType
        match element_type.as_str() {
            "MET_SHORT" | "MET_INT16" => {
                for chunk in raw.chunks_exact(2).take(voxel_count) {
                    let val = i16::from_le_bytes([chunk[0], chunk[1]]);
                    voxels.push(val);
                }
            }
            "MET_FLOAT" => {
                for chunk in raw.chunks_exact(4).take(voxel_count) {
                    let val = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    let val = (val * slope + intercept).round() as i16;
                    voxels.push(val);
                }
            }
            other => return Err(anyhow!("Unsupported ElementType: {}", other)),
        }

        Ok(MHDHeader {
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

    // ------------------------ read MHD ------------------------
    pub fn read_mhd_or_mha<P: AsRef<Path>>(path: P) -> Result<Self> {
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
        let patient_position: String = match anatomical_orientation.as_str() {
            "IRP" => "HFS".to_string(),
            "ILP" => "HFP".to_string(),
            "SRP" => "FFS".to_string(),
            "SLP" => "FFP".to_string(),
            _ => "UNKNOWN".to_string(), // Default case for unhandled orientations
        };
        Ok(MHDHeader {
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

    // ------------------------ resd RAW（MET_FLOAT） ------------------------
    pub fn add_data(&self,path:PathBuf) -> Result<Vec<f32>> {
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
        let mut data = Vec::with_capacity(n);
        let mut rdr = BufReader::new(&buf[..]);
        for _ in 0..n {
            data.push(rdr.read_f32::<LittleEndian>()?);
        }
        
        Ok(data)
    }

    // ------------------------ 将图像数据转换为CTVolume -----------------
    pub fn generate_ct_volume_mha(&self) -> Result<CTVolume, String> {
        let col = self.dim[0]; // x
        let row = self.dim[1]; // y
        let depth = self.dim[2]; // z
        let data = &self.data;
        let mut voxel_data = data.clone();

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

use std::fs;
pub fn read_file_as_bytes(path: &str) -> Result<Vec<u8>, std::io::Error> {
    fs::read(path)
}

//------------------------------ test Code -------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array3;

    #[test]
    fn test_read_mha_or_mhd() -> Result<()> {
        let path = "C:/share/input/CT.mha";
        let header = MHDHeader::read_mhd_or_mha(path)?;
        println!("=== MHDHeader 解析结果 ===");
        println!("维度 (DimSize): {:?}", header.dim);
        println!("体素间距 (ElementSpacing): {:?}", header.spacing);
        println!("数据类型 (ElementType): {}", header.element_type);
        println!("数据文件 (ElementDataFile): {}", header.element_data_file);
        println!("原点偏移 (Offset): {:?}", header.offset);
        println!("方向矩阵 (TransformMatrix): {:?}", header.transform);
        println!("数据偏移 (data_offset，仅 .mha 有): {:?}", header.data_offset);

        let path_raw = PathBuf::from(path);
        let data = MHDHeader::add_data(&header,path_raw)?;
        let arr = Array3::from_shape_vec((header.dim[2], header.dim[1], header.dim[0]), data.clone())
            .map_err(|e| anyhow!("reshape to (nz,ny,nx) failed: {}", e))?;
        let data: Vec<f32> = arr.iter().cloned().collect();

        println!("前 10 个体素值: {:?}", &data[..10]);

        assert_eq!(data.len(), header.dim.iter().product::<usize>());
        Ok(())
    }

    #[test]
    fn test_forbytes()-> Result<(), std::io::Error> {
        let data = read_file_as_bytes("C:/share/input/CT.mha")?;
        let bytes_slice: &[u8] = &data;
        let slope = 1612.903;
        let intercept = -1016.129;

        let header = MHDHeader::from_bytes(bytes_slice, slope, intercept).unwrap();
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