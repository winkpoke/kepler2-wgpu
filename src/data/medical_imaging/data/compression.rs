use std::path::PathBuf;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum CompressionType {
    GZip,
    ZLib,
    Raw,
}

// dicom sink trait
pub trait DicomSink {
    fn save_slice(&mut self, filename: String, data: Vec<u8>) -> Result<()>;
}

pub struct FsSink {
    pub(crate) out_dir: PathBuf,
}

impl DicomSink for FsSink {
    fn save_slice(&mut self, filename: String, data: Vec<u8>) -> Result<()> {
        let out_path = self.out_dir.join(filename);
        std::fs::write(&out_path, data)?;
        Ok(())
    }
}

pub struct MemSink {
    pub(crate) files: Vec<(String, Vec<u8>)>,
}

impl MemSink {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }
}

impl DicomSink for MemSink {
    fn save_slice(&mut self, filename: String, data: Vec<u8>) -> Result<()> {
        self.files.push((filename, data));
        Ok(())
    }
}