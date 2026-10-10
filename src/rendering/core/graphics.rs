use log::{info, warn};
use std::sync::Arc;
use winit::window::Window;

use crate::KeplerError;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug)]
pub struct Graphics {
    pub(crate) window: Arc<Window>,
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) surface_config: wgpu::SurfaceConfiguration,
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    pub(crate) faults: super::fault::FaultNotifier,
    capabilities: super::capabilities::GpuCapabilities,
    // Function-level comment: Wrap device and queue in Arc for cheap cloning across subsystems
    pub(crate) device: Arc<wgpu::Device>,
    pub(crate) queue: Arc<wgpu::Queue>,
}

#[cfg(target_arch = "wasm32")]
async fn preflight_browser_backend(
    backend: super::browser_backend::BrowserBackend,
) -> Result<(), String> {
    let backends = match backend {
        super::browser_backend::BrowserBackend::BrowserWebGpu => wgpu::Backends::BROWSER_WEBGPU,
        super::browser_backend::BrowserBackend::Gl => wgpu::Backends::GL,
    };
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let canvas = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| format!("{backend:?} preflight: browser document is unavailable"))?
        .create_element("canvas")
        .map_err(|error| {
            format!("{backend:?} preflight: detached canvas creation failed: {error:?}")
        })?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| format!("{backend:?} preflight: detached element is not a canvas"))?;
    canvas.set_width(1);
    canvas.set_height(1);

    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
        .map_err(|error| format!("{backend:?} preflight surface creation failed: {error}"))?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        })
        .await
        .map_err(|error| format!("{backend:?} preflight adapter request failed: {error}"))?;
    let adapter_info = adapter.get_info();
    let expected_backend = match backend {
        super::browser_backend::BrowserBackend::BrowserWebGpu => wgpu::Backend::BrowserWebGpu,
        super::browser_backend::BrowserBackend::Gl => wgpu::Backend::Gl,
    };
    if adapter_info.backend != expected_backend {
        return Err(format!(
            "{backend:?} preflight selected unexpected adapter backend {:?}",
            adapter_info.backend
        ));
    }

    let surface_capabilities = surface.get_capabilities(&adapter);
    if surface_capabilities.formats.is_empty()
        || surface_capabilities.present_modes.is_empty()
        || surface_capabilities.alpha_modes.is_empty()
    {
        return Err(format!(
            "{backend:?} preflight surface has no usable format, present mode, or alpha mode"
        ));
    }
    let required_limits = super::capabilities::browser_limits(&adapter.limits())
        .map_err(|error| format!("{backend:?} preflight limit negotiation failed: {error}"))?;
    let (device, _queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("Kepler browser GPU preflight"),
            required_features: wgpu::Features::empty(),
            required_limits,
            memory_hints: Default::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            trace: wgpu::Trace::Off,
        })
        .await
        .map_err(|error| format!("{backend:?} preflight device request failed: {error}"))?;
    super::capabilities::GpuCapabilities::from_device(&adapter, &device)
        .map_err(|error| format!("{backend:?} preflight volume capabilities failed: {error}"))?;
    Ok(())
}

/// Graphics context that encapsulates both hardware abstraction and rendering pipeline orchestration
pub struct GraphicsContext {
    /// Core graphics hardware abstraction
    pub(crate) graphics: Graphics,
    /// Rendering pipeline orchestrator
    pub(crate) pass_executor: crate::rendering::core::PassExecutor,
    /// Depth attachments are valid for this device and survive individual frames.
    pub(crate) mesh_texture_pool: crate::rendering::view::mesh::mesh_texture_pool::MeshTexturePool,
}

impl GraphicsContext {
    /// Function-level comment: Create a new GraphicsContext with initialized Graphics and PassExecutor.
    pub async fn new(window: Arc<Window>) -> Result<GraphicsContext, KeplerError> {
        let graphics = Graphics::initialize(window).await?;
        let pass_executor =
            crate::rendering::core::PassExecutor::new(graphics.surface_config.format);

        Ok(GraphicsContext {
            graphics,
            pass_executor,
            mesh_texture_pool: Default::default(),
        })
    }

    /// Function-level comment: Create a new GraphicsContext from an existing Graphics instance.
    pub fn from_graphics(graphics: Graphics) -> GraphicsContext {
        let pass_executor =
            crate::rendering::core::PassExecutor::new(graphics.surface_config.format);

        GraphicsContext {
            graphics,
            pass_executor,
            mesh_texture_pool: Default::default(),
        }
    }

    /// Color attachment format of this context's configured surface.
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.graphics.surface_config.format
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
        self.pass_executor_mut()
            .update_surface_format(config.format);
        *self.surface_config_mut() = config;
    }
}

impl Drop for Graphics {
    fn drop(&mut self) {
        // Old devices may still be retained by views; they must not wake a new owner.
        self.faults.clear_wake();
    }
}

impl Graphics {
    #[cfg(target_arch = "wasm32")]
    async fn preflight_browser_selection(
    ) -> Result<super::browser_backend::BrowserBackend, KeplerError> {
        use super::browser_backend::{
            browser_backend_preference_from_query, select_browser_candidate, BrowserBackend,
        };

        let preference = browser_backend_preference_from_query();
        // wgpu's adapter probe checks actual adapter creation, not just navigator.gpu presence.
        let webgpu_detected = preference == super::browser_backend::BrowserBackendPreference::Auto
            && wgpu::util::is_browser_webgpu_supported().await;
        let selection = select_browser_candidate(
            preference,
            webgpu_detected,
            || async { preflight_browser_backend(BrowserBackend::BrowserWebGpu).await },
            || async { preflight_browser_backend(BrowserBackend::Gl).await },
        )
        .await
        .map_err(|error| {
            KeplerError::Graphics(format!(
                "Browser GPU pre-canvas negotiation failed: {error}"
            ))
        })?;
        if let Some(reason) = selection.webgpu_failure.as_deref() {
            warn!("WebGPU unavailable during pre-canvas negotiation; trying WebGL2: {reason}");
        }
        info!("Browser GPU preflight selected {:?}", selection.backend);
        Ok(selection.backend)
    }

    /// Capabilities negotiated for this graphics device.
    pub(crate) fn capabilities(&self) -> &super::capabilities::GpuCapabilities {
        &self.capabilities
    }

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
            let env_backend = std::env::var("KEPLER_WGPU_BACKEND")
                .ok()
                .or_else(|| std::env::var("WGPU_BACKEND").ok());
            match env_backend.as_deref() {
                Some("dx12") => {
                    info!("Backend override via env: DX12");
                    wgpu::Backends::DX12
                }
                Some("vulkan") | Some("vk") => {
                    info!("Backend override via env: VULKAN");
                    wgpu::Backends::VULKAN
                }
                Some("metal") => {
                    info!("Backend override via env: METAL");
                    wgpu::Backends::METAL
                }
                Some("gl") => {
                    info!("Backend override via env: GL");
                    wgpu::Backends::GL
                }
                Some("primary") | Some("auto") | None => {
                    info!("Backend selection: PRIMARY");
                    wgpu::Backends::PRIMARY
                }
                Some(other) => {
                    warn!("Unknown backend {} in env, defaulting to PRIMARY", other);
                    wgpu::Backends::PRIMARY
                }
            }
        };
        #[cfg(target_arch = "wasm32")]
        let selected_browser_backend = Self::preflight_browser_selection().await?;
        #[cfg(target_arch = "wasm32")]
        let selected_backends = match selected_browser_backend {
            super::browser_backend::BrowserBackend::BrowserWebGpu => wgpu::Backends::BROWSER_WEBGPU,
            super::browser_backend::BrowserBackend::Gl => wgpu::Backends::GL,
        };
        #[cfg(target_arch = "wasm32")]
        let post_binding_error = |stage, detail: &str| {
            KeplerError::Graphics(super::browser_backend::post_binding_error(
                selected_browser_backend,
                stage,
                detail,
            ))
        };

        #[cfg(not(target_arch = "wasm32"))]
        let instance_flags: wgpu::InstanceFlags =
            match std::env::var("KEPLER_WGPU_VALIDATION").ok().as_deref() {
                Some("1") | Some("true") | Some("on") => {
                    info!("Instance validation enabled via env KEPLER_WGPU_VALIDATION");
                    wgpu::InstanceFlags::VALIDATION
                }
                _ => wgpu::InstanceFlags::empty(),
            };
        #[cfg(target_arch = "wasm32")]
        let instance_flags: wgpu::InstanceFlags = wgpu::InstanceFlags::empty();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: selected_backends,
            flags: instance_flags,
            // Native GLES needs the window's display before surface creation.
            // Retain the window so that display also survives surface recovery.
            #[cfg(not(target_arch = "wasm32"))]
            display: Some(Box::new(window.clone())),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        #[cfg(not(target_arch = "wasm32"))]
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| KeplerError::Graphics(format!("Failed to create surface: {}", e)))?;
        #[cfg(target_arch = "wasm32")]
        let surface = instance.create_surface(window.clone()).map_err(|error| {
            post_binding_error("real-canvas surface creation", &error.to_string())
        })?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|error| {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    KeplerError::Graphics(format!("Failed to find suitable adapter: {error}"))
                }
                #[cfg(target_arch = "wasm32")]
                {
                    post_binding_error("real-canvas adapter request", &error.to_string())
                }
            })?;

        #[cfg(target_arch = "wasm32")]
        let required_limits =
            super::capabilities::browser_limits(&adapter.limits()).map_err(|error| {
                post_binding_error("real-canvas device limit validation", &error.to_string())
            })?;
        #[cfg(not(target_arch = "wasm32"))]
        let required_limits = super::capabilities::native_limits(&adapter.limits());

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits,
                memory_hints: Default::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|error| {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    KeplerError::Graphics(format!("Failed to create device: {error}"))
                }
                #[cfg(target_arch = "wasm32")]
                {
                    post_binding_error("real-canvas device request", &error.to_string())
                }
            })?;

        let faults = super::fault::FaultNotifier::default();
        let backend = adapter.get_info().backend;
        let uncaptured = faults.clone();
        device.on_uncaptured_error(Arc::new(move |error| {
            use crate::core::error::{SurfaceFailure, SurfaceFailureKind};
            let kind = match &error {
                wgpu::Error::OutOfMemory { .. } => SurfaceFailureKind::OutOfMemory,
                wgpu::Error::Validation { .. } => SurfaceFailureKind::Validation,
                wgpu::Error::Internal { .. } => SurfaceFailureKind::Other,
            };
            uncaptured.report(SurfaceFailure::new(
                kind,
                "uncaptured GPU operation",
                format!("{backend:?}: {error}"),
            ));
        }));
        let lost = faults.clone();
        device.set_device_lost_callback(move |reason, message| {
            if reason != wgpu::DeviceLostReason::Destroyed {
                lost.report(crate::core::error::SurfaceFailure::new(
                    crate::core::error::SurfaceFailureKind::DeviceLost,
                    "device lost callback",
                    format!("{backend:?} ({reason:?}): {message}"),
                ));
            }
        });

        #[cfg(not(target_arch = "wasm32"))]
        let capabilities = super::capabilities::GpuCapabilities::from_device(&adapter, &device)?;
        #[cfg(target_arch = "wasm32")]
        let capabilities = super::capabilities::GpuCapabilities::from_device(&adapter, &device)
            .map_err(|error| {
                post_binding_error(
                    "real-canvas volume capability validation",
                    &error.to_string(),
                )
            })?;
        let adapter_info = adapter.get_info();
        // Native: read env to summarize requested backend and validation; WASM: default values
        #[cfg(not(target_arch = "wasm32"))]
        let backend_env = std::env::var("KEPLER_WGPU_BACKEND")
            .ok()
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
        let (backend_str, validation) = (format!("{:?}", adapter_info.backend), false);

        info!(
            "Adapter: {} ({:?}), vendor: {}, device: {}",
            adapter_info.name, adapter_info.backend, adapter_info.vendor, adapter_info.device
        );
        info!("Final backend chosen by wgpu: {:?}", adapter_info.backend);
        info!(
            "Startup GPU summary: requested_backend={} validation={} final_backend={:?} adapter=\"{}\" vendor={} device={}",
            backend_str,
            if validation { "enabled" } else { "disabled" },
            adapter_info.backend,
            adapter_info.name,
            adapter_info.vendor,
            adapter_info.device
        );
        info!(
            "GPU volume capabilities: backend={:?} r16float_filterable={} rg8_filterable={} max_3d_texture_dimension={} default_encoding={:?}",
            capabilities.backend(),
            capabilities.r16float_filterable(),
            capabilities.rg8_filterable(),
            capabilities.max_3d_texture_dimension(),
            capabilities.volume_encoding()
        );

        let surface_caps = surface.get_capabilities(&adapter);
        // tests
        info!("Surface supports {} formats:", surface_caps.formats.len());
        for fmt in surface_caps.formats.iter() {
            let is_srgb = fmt.is_srgb();
            info!("  - {:?}: sRGB={}", fmt, is_srgb);
        }
        // Prefer non-sRGB (linear) format for medical grayscale accuracy.
        // sRGB applies gamma on present, which can alter perceived contrast of DICOM WL.
        // Choose BGRA/RGBA Unorm if available; otherwise any non-sRGB; else fallback.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|format| !format.is_srgb())
            .or_else(|| surface_caps.formats.first().copied())
            .ok_or_else(|| {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    KeplerError::Graphics("Surface reports no supported color formats".to_string())
                }
                #[cfg(target_arch = "wasm32")]
                {
                    post_binding_error(
                        "real-canvas surface capabilities",
                        "no color formats were returned",
                    )
                }
            })?;
        let present_mode = surface_caps.present_modes.first().copied().ok_or_else(|| {
            #[cfg(not(target_arch = "wasm32"))]
            {
                KeplerError::Graphics("Surface reports no supported present modes".to_string())
            }
            #[cfg(target_arch = "wasm32")]
            {
                post_binding_error(
                    "real-canvas surface capabilities",
                    "no present modes were returned",
                )
            }
        })?;
        let alpha_mode = surface_caps.alpha_modes.first().copied().ok_or_else(|| {
            #[cfg(not(target_arch = "wasm32"))]
            {
                KeplerError::Graphics("Surface reports no supported alpha modes".to_string())
            }
            #[cfg(target_arch = "wasm32")]
            {
                post_binding_error(
                    "real-canvas surface capabilities",
                    "no alpha modes were returned",
                )
            }
        })?;
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode,
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
        };
        info!(
            "Surface format chosen: {:?} (sRGB: {}), present_mode: {:?}, alpha_mode: {:?}",
            surface_config.format,
            surface_config.format.is_srgb(),
            surface_config.present_mode,
            surface_config.alpha_mode
        );

        if size.width > 0 && size.height > 0 {
            let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
            surface.configure(&device, &surface_config);
            if let Some(error) = scope.pop().await {
                #[cfg(target_arch = "wasm32")]
                return Err(post_binding_error(
                    "real-canvas surface configuration",
                    &error.to_string(),
                ));
                #[cfg(not(target_arch = "wasm32"))]
                return Err(crate::core::error::SurfaceFailure::new(
                    crate::core::error::SurfaceFailureKind::Validation,
                    "initial surface configuration",
                    format!("{backend:?}: {error}"),
                )
                .into());
            }
            faults.check()?;
        }

        // Wrap device and queue in Arc to enable sharing without cloning wgpu handles
        let device = Arc::new(device);
        let queue = Arc::new(queue);

        Ok(Self {
            surface,
            surface_config,
            instance,
            adapter,
            faults,
            capabilities,
            device,
            queue,
            window,
        })
    }

    /// Reuse the selected device and backend; never renegotiate a bound browser canvas.
    pub(crate) fn recover_surface(
        &mut self,
        recreate: bool,
    ) -> crate::core::error::KeplerResult<()> {
        use crate::core::error::{SurfaceFailure, SurfaceFailureKind};
        if self.surface_config.width == 0 || self.surface_config.height == 0 {
            return Ok(());
        }
        if recreate {
            let surface = self
                .instance
                .create_surface(self.window.clone())
                .map_err(|error| {
                    SurfaceFailure::new(
                        SurfaceFailureKind::Lost,
                        "recreate selected-backend surface",
                        format!("{:?}: {error}", self.capabilities.backend()),
                    )
                })?;
            let caps = surface.get_capabilities(&self.adapter);
            let format = caps
                .formats
                .iter()
                .copied()
                .find(|format| !format.is_srgb())
                .or_else(|| caps.formats.first().copied())
                .ok_or_else(|| {
                    SurfaceFailure::new(
                        SurfaceFailureKind::Lost,
                        "recreated surface capabilities",
                        format!("{:?}: no color formats", self.capabilities.backend()),
                    )
                })?;
            self.surface_config.format = format;
            self.surface_config.present_mode = *caps.present_modes.first().ok_or_else(|| {
                SurfaceFailure::new(
                    SurfaceFailureKind::Lost,
                    "recreated surface capabilities",
                    "no present modes",
                )
            })?;
            self.surface_config.alpha_mode = *caps.alpha_modes.first().ok_or_else(|| {
                SurfaceFailure::new(
                    SurfaceFailureKind::Lost,
                    "recreated surface capabilities",
                    "no alpha modes",
                )
            })?;
            self.surface = surface;
        }
        self.faults.check()?;
        self.surface.configure(&self.device, &self.surface_config);
        self.faults.check().map_err(|error| {
            SurfaceFailure::new(
                SurfaceFailureKind::Validation,
                "configure surface recovery",
                format!("{:?}: {error}", self.capabilities.backend()),
            )
            .into()
        })
    }

    pub async fn new(window: Arc<Window>) -> Result<Graphics, KeplerError> {
        Self::initialize(window).await
    }
}
