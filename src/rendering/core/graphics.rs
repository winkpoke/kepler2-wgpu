use std::sync::Arc;
use winit::window::Window;
use log::{info, warn};

use crate::KeplerError;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;


#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug)]
pub struct Graphics {
    pub(crate) window: Arc<Window>,
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) surface_config: wgpu::SurfaceConfiguration,
    pub(crate) adapter: wgpu::Adapter,
    // Function-level comment: Wrap device and queue in Arc for cheap cloning across subsystems
    pub(crate) device: Arc<wgpu::Device>,
    pub(crate) queue: Arc<wgpu::Queue>,
}

/// Graphics context that encapsulates both hardware abstraction and rendering pipeline orchestration
pub struct GraphicsContext {
    /// Core graphics hardware abstraction
    pub(crate) graphics: Graphics,
    /// Rendering pipeline orchestrator
    pub(crate) pass_executor: crate::rendering::core::PassExecutor,
}

impl GraphicsContext {
    /// Function-level comment: Create a new GraphicsContext with initialized Graphics and PassExecutor.
    pub async fn new(window: Arc<Window>) -> Result<GraphicsContext, KeplerError> {
        let graphics = Graphics::initialize(window).await?;
        let pass_executor = crate::rendering::core::PassExecutor::new(graphics.surface_config.format);
        
        Ok(GraphicsContext {
            graphics,
            pass_executor,
        })
    }

    /// Function-level comment: Create a new GraphicsContext from an existing Graphics instance.
    pub fn from_graphics(graphics: Graphics) -> GraphicsContext {
        let pass_executor = crate::rendering::core::PassExecutor::new(graphics.surface_config.format);
        
        GraphicsContext {
            graphics,
            pass_executor,
        }
    }
    
    /// Function-level comment: Get a reference to the underlying Graphics struct.
    pub fn graphics(&self) -> &Graphics {
        &self.graphics
    }
    
    /// Function-level comment: Get a mutable reference to the underlying Graphics struct.
    pub fn graphics_mut(&mut self) -> &mut Graphics {
        &mut self.graphics
    }
    
    /// Function-level comment: Get a reference to the PassExecutor.
    pub fn pass_executor(&self) -> &crate::rendering::core::PassExecutor {
        &self.pass_executor
    }
    
    /// Function-level comment: Get a mutable reference to the PassExecutor.
    pub fn pass_executor_mut(&mut self) -> &mut crate::rendering::core::PassExecutor {
        &mut self.pass_executor
    }
    
    /// Function-level comment: Get mutable reference to surface configuration.
    pub fn surface_config_mut(&mut self) -> &mut wgpu::SurfaceConfiguration {
        &mut self.graphics.surface_config
    }
    
    /// Function-level comment: Update surface configuration and notify PassExecutor of format changes.
    pub fn update_surface_config(&mut self, config: wgpu::SurfaceConfiguration) {
        self.pass_executor_mut().update_surface_format(config.format);
        *self.surface_config_mut() = config;
    }
}

impl Graphics {
    // Function-level comment: Initialize Graphics with environment-driven backend selection and optional validation.
    // Behavior:
    // - Native: default to Backends::PRIMARY; allow override via KEPLER_WGPU_BACKEND or WGPU_BACKEND (dx12, vulkan, metal, gl, primary/auto).
    // - WASM: use Backends::GL for broad compatibility.
    // - Validation: enable via KEPLER_WGPU_VALIDATION=1/true/on; otherwise disabled to avoid noisy Vulkan loader warnings.
    // - Negotiates surface format preferring sRGB; logs adapter info and chosen format for transparency.
    // This improves portability, reduces overlay/validation noise on Windows, and keeps color correctness across OS/GPU.
    pub async fn initialize(window: Arc<Window>) -> Result<Graphics, KeplerError> {
        let size = window.inner_size();

        // The instance is a handle to our GPU with runtime-selectable backend and optional validation
        #[cfg(not(target_arch = "wasm32"))]
        let selected_backends: wgpu::Backends = {
            let env_backend = std::env::var("KEPLER_WGPU_BACKEND").ok()
                .or_else(|| std::env::var("WGPU_BACKEND").ok());
            match env_backend.as_deref() {
                Some("dx12") => { info!("Backend override via env: DX12"); wgpu::Backends::DX12 }
                Some("vulkan") | Some("vk") => { info!("Backend override via env: VULKAN"); wgpu::Backends::VULKAN }
                Some("metal") => { info!("Backend override via env: METAL"); wgpu::Backends::METAL }
                Some("gl") => { info!("Backend override via env: GL"); wgpu::Backends::GL }
                Some("primary") | Some("auto") | None => { info!("Backend selection: PRIMARY"); wgpu::Backends::PRIMARY }
                Some(other) => { warn!("Unknown backend {} in env, defaulting to PRIMARY", other); wgpu::Backends::PRIMARY }
            }
        };
        #[cfg(target_arch = "wasm32")]
        let selected_backends: wgpu::Backends = wgpu::Backends::GL;

        #[cfg(not(target_arch = "wasm32"))]
        let instance_flags: wgpu::InstanceFlags = match std::env::var("KEPLER_WGPU_VALIDATION").ok().as_deref() {
            Some("1") | Some("true") | Some("on") => {
                info!("Instance validation enabled via env KEPLER_WGPU_VALIDATION");
                wgpu::InstanceFlags::VALIDATION
            },
            _ => wgpu::InstanceFlags::empty(),
        };
        #[cfg(target_arch = "wasm32")]
        let instance_flags: wgpu::InstanceFlags = wgpu::InstanceFlags::empty();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: selected_backends,
            flags: instance_flags,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())
            .map_err(|e| KeplerError::Graphics(format!("Failed to create surface: {}", e)))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| KeplerError::Graphics("Failed to find suitable adapter".to_string()))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits {
                            max_texture_dimension_3d: 1024,
                            ..wgpu::Limits::downlevel_webgl2_defaults()
                        }
                    } else {
                        wgpu::Limits::default()
                    },
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| KeplerError::Graphics(format!("Failed to create device: {}", e)))?;

        let adapter_info = adapter.get_info();
        // Native: read env to summarize requested backend and validation; WASM: default values
        #[cfg(not(target_arch = "wasm32"))]
        let backend_env = std::env::var("KEPLER_WGPU_BACKEND").ok()
            .or_else(|| std::env::var("WGPU_BACKEND").ok());
        #[cfg(not(target_arch = "wasm32"))]
        let backend_str = match backend_env.as_deref() {
            Some("dx12") => "dx12",
            Some("vulkan") | Some("vk") => "vulkan",
            Some("metal") => "metal",
            Some("gl") => "gl",
            Some("primary") | Some("auto") | None => "primary",
            Some(other) => other,
        };
        #[cfg(not(target_arch = "wasm32"))]
        let validation = matches!(
            std::env::var("KEPLER_WGPU_VALIDATION").ok().as_deref(),
            Some("1") | Some("true") | Some("on")
        );
        #[cfg(target_arch = "wasm32")]
        let (backend_str, validation) = ("gl", false);

        info!("Adapter: {} ({:?}), vendor: {}, device: {}", adapter_info.name, adapter_info.backend, adapter_info.vendor, adapter_info.device);
        info!("Final backend chosen by wgpu: {:?}", adapter_info.backend);
        info!(
            "Startup GPU summary: backend_env={} validation={} final_backend={:?} adapter=\"{}\" vendor={} device={}",
            backend_str,
            if validation { "enabled" } else { "disabled" },
            adapter_info.backend,
            adapter_info.name,
            adapter_info.vendor,
            adapter_info.device
        );

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
        };
        info!("Surface format chosen: {:?} (sRGB: {}), present_mode: {:?}, alpha_mode: {:?}", surface_config.format, surface_config.format.is_srgb(), surface_config.present_mode, surface_config.alpha_mode);

        if size.width > 0 && size.height > 0 {
            surface.configure(&device, &surface_config);
            crate::rendering::core::pipeline::set_swapchain_format(surface_config.format);
        }

        // Wrap device and queue in Arc to enable sharing without cloning wgpu handles
        let device = Arc::new(device);
        let queue = Arc::new(queue);

        Ok(Self {
            surface,
            surface_config,
            adapter,
            device,
            queue,
            window,
        })
    }

    pub async fn new(window: Arc<Window>) -> Result<Graphics, KeplerError> {
        Self::initialize(window).await
    }
}