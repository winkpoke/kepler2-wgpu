use std::fs;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use anyhow::Result;
use std::sync::Arc;
use crate::server::segment_engine::ModelManager;

/// Initialize ONNX-side state during startup. Returns a ready-to-use
/// `ModelManager` whose `models/` directory the caller wires into
/// `ServerState` via `state.onnx_models.lock().replace(...)`.
pub async fn initialize_ai_services() -> Result<Arc<ModelManager>> {
    let model_dir = std::env::var("KEPLER_MODEL_DIR").unwrap_or_else(|_| "models".to_string());
    tokio::fs::create_dir_all(&model_dir).await?;
    let manager = Arc::new(ModelManager::new(&model_dir).await?);
    manager.preload_all().await?;
    Ok(manager)
}

pub struct ModelDownloader {
    base_url: String,
    models_dir: String,
}

impl ModelDownloader {
    pub fn new(models_dir: &str, base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            models_dir: models_dir.to_string(),
        }
    }

    pub async fn download_model(&self, model_name: &str) -> Result<()> {
        let url = format!("{}/{}.onnx", self.base_url, model_name);
        let local_path = format!("{}/{}.onnx", self.models_dir, model_name);
        log::info!("Downloading model {} from {}", model_name, url);

        // 创建HTTP客户端
        let client = reqwest::Client::new();

        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to download model: {}", response.status()));
        }

        // reqwest::Response doesn't implement AsyncRead directly; use the
        // convenient `bytes()` aggregator to pull the full body.
        let buffer = response.bytes().await?;

        let mut file = File::create(&local_path).await?;
        file.write_all(&buffer).await?;

        log::info!("Model {} downloaded successfully", model_name);

        Ok(())
    }

    pub fn list_models(&self) -> Result<Vec<String>> {
        let mut models = vec![];
        
        if Path::new(&self.models_dir).exists() {
            for entry in fs::read_dir(&self.models_dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.extension().and_then(|s| s.to_str()) == Some("onnx") {
                    if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                        models.push(filename.to_string());
                    }
                }
            }
        }
        
        Ok(models)
    }

    pub fn cleanup_old_models(&self, max_models: usize) -> Result<()> {
        let models = self.list_models()?;
        
        if models.len() <= max_models {
            return Ok(());
        }
        
        // 保留最新的N个模型，删除其他
        let mut model_files: Vec<_> = models.into_iter().map(|name| {
            format!("{}/{}.onnx", self.models_dir, name)
        }).collect();
        
        // 按修改时间排序
        model_files.sort_by(|a, b| {
            let meta_a = fs::metadata(a).unwrap();
            let meta_b = fs::metadata(b).unwrap();
            meta_b.modified().unwrap().cmp(&meta_a.modified().unwrap())
        });
        
        // 删除旧模型
        for model_path in model_files[max_models..].iter() {
            fs::remove_file(model_path)?;
        }
        
        Ok(())
    }
}