use std::{fs::File, 
    io::{Read, Seek, SeekFrom}, 
    path::PathBuf, 
    fmt, 
    collections::HashMap
};
use crate::coord::{Base, Matrix4x4};
use crate::ct_volume::CTVolume;
use anyhow::{anyhow, Context, Result};

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
pub struct MhxHeader {
    pub(crate) dim: Vec<usize>,
    pub(crate) spacing: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct MhxData {
    pub(crate) data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum MhxDataLazy {
    Mha { file: PathBuf, data_offset: u64 },
    Mhd { data_file: PathBuf },
}

#[derive(Debug, Clone)]
pub struct MhxHandler {
    pub(crate) header: MhxHeader,
    pub(crate) data: MhxDataLazy,
}

#[derive(Debug, Clone)]
pub struct Mhx {
    pub(crate) header: MhxHeader,
    pub(crate) data: MhxData,
}

impl MhxHeader {
    pub fn read_mhx_header(path: &PathBuf) -> std::io::Result<MhxHandler> {
        // Parse text header, find data file or offset.
        // Example placeholder:
        let header = MhxHeader { dim: vec![128,128,64], spacing: vec![1.0,1.0,1.0] };
        let data = MhxDataLazy::Mha { file: path.clone(), data_offset: 1024 };
        Ok(MhxHandler { header, data })
    }
}

impl MhxDataLazy {
    pub fn read_data(&self, header: &MhxHeader) -> std::io::Result<MhxData> {
        match self {
            MhxDataLazy::Mha { file, data_offset } => {
                let mut f = File::open(file)?;
                f.seek(SeekFrom::Start(*data_offset))?;
                let mut buf = Vec::new();
                f.read_to_end(&mut buf)?;
                Ok(MhxData { data: buf })
            }
            MhxDataLazy::Mhd { data_file } => {
                let mut f = File::open(data_file)?;
                let mut buf = Vec::new();
                f.read_to_end(&mut buf)?;
                Ok(MhxData { data: buf })
            }
        }
    }
}

impl MhxHandler {
    pub fn read_data(self) -> std::io::Result<Mhx> {
        let data = self.data.read_data(&self.header)?;
        Ok(Mhx { header: self.header, data })
    }
}


fn func() -> std::io::Result<()> {
    let path = PathBuf::from("image.mha");
    let handler = MhxHeader::read_mhx_header(&path)?;
    let mhx = handler.read_data()?; // header + raw voxels
    println!("{:?}", mhx.header.dim);
    Ok(())
}