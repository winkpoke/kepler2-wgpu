#![allow(dead_code)]

use log::{trace, info, warn};
use std::path::PathBuf;
use std::sync::Arc;
use std::{fs, io};
use crate::core::timing::{Instant, DurationExt};
use crate::rendering::view;
use crate::rendering::core::pipeline::PipelineManager;

// use wgpu::util::DeviceExt;
#[cfg(target_arch = "wasm32")]
use async_lock::Mutex;

use winit::{
    event::*,
    window::Window,
};

use crate::data::ct_volume::*;
use crate::data::dicom::*;
use crate::rendering::view::render_content::RenderContent;
use crate::rendering::view::*;
use crate::core::error::KeplerError;
use crate::rendering::mesh::mesh_texture_pool::MeshTexturePool;

fn list_files_in_directory(dir: &str) -> io::Result<Vec<PathBuf>> {
    let mut file_paths = Vec::new();

    // Open the directory and iterate over its contents
    for entry in fs::read_dir(dir)? {
        let entry = entry?; // unwrap the result of read_dir
        let path = entry.path();

        // Check if the entry is a file (not a directory)
        if path.is_file() {
            file_paths.push(path); // Add the full path to the list
        }
    }

    Ok(file_paths)
}

// static STATE: Lazy<Arc<Mutex<Option<State>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));

// thread_local! {
//     static STATE: OnceCell<Rc<RefCell<State>>> = OnceCell::new();
// }

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug)]
pub struct Graphics {
    pub(crate) window: Arc<Window>,
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) surface_config: wgpu::SurfaceConfiguration,
    pub(crate) adapter: wgpu::Adapter,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
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
            .find(|f| f.is_srgb())
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

pub struct AppModel {
    pub(crate) vol: Option<CTVolume>,
    pub(crate) app: Arc<App>,
}

pub struct AppView {
    pub(crate) graphics: Graphics,
    pub(crate) layout: Layout<GridLayout>,
    pub(crate) app: Arc<App>,
}

pub struct App {
    pub(crate) view: Arc<AppView>,
    pub(crate) doc: Arc<AppModel>,
}

// #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct State {
    pub(crate) graphics: Graphics,
    // pub(crate) layout: Layout<OneCellLayout>,
    pub(crate) layout: Layout<GridLayout>,
    pub(crate) enable_float_volume_texture: bool,
    pub(crate) toggle_enabled: bool,
    pub(crate) last_volume: Option<CTVolume>,
    pub(crate) enable_mesh: bool,
    pub(crate) texture_pool: MeshTexturePool,
    /// Function-level comment: Snapshot of slot-2 MPR state to restore when mesh mode is disabled.
    pub(crate) mpr_state_slot2: Option<MPRViewState>,
    /// Function-level comment: Cached BasicMeshContext wrapped in Arc for efficient reuse across toggles.
    pub(crate) mesh_ctx: Option<Arc<crate::rendering::mesh::basic_mesh_context::BasicMeshContext>>,
    /// Function-level comment: PassExecutor manages separate render passes for 3D mesh and 2D slice content.
    pub(crate) pass_executor: crate::rendering::core::PassExecutor,
}

const HU_OFFSET: f32 = 1100.0;

/// Captures key MPR view parameters for restoring after mesh toggles.
#[derive(Clone, Debug)]
pub struct MPRViewState {
    /// Function-level comment: Current window level used by the fragment shader (uniform value).
    pub window_level: f32,
    /// Function-level comment: Current window width used by the fragment shader (uniform value).
    pub window_width: f32,
    /// Function-level comment: Current slice position in millimeters along the view normal.
    pub slice_mm: f32,
    /// Function-level comment: Current screen-space scale factor.
    pub scale: f32,
    /// Function-level comment: Current view/model-space translation vector.
    pub translate: [f32; 3],
    /// Function-level comment: Current screen-space translation (pan) vector.
    pub translate_in_screen_coord: [f32; 3],
}

impl State {
    pub async fn new(window: Arc<Window>) -> Result<State, KeplerError> {
        State::initialize(window).await
    }

    pub async fn initialize(window: Arc<Window>) -> Result<State, KeplerError> {
        let graphics = Graphics::new(window.clone()).await?;
        // println!("supported texture formats: {:?}", surface_caps.formats);
        // println!("format: {:?}", config.format);

        let layout = Layout::new(
            (graphics.surface_config.width, graphics.surface_config.height),
            GridLayout {
                rows: 2,
                cols: 2,
                spacing: 2,
            },
        );

        // Choose default format based on device capability: prefer R16Float when supported, else RG8
        let default_float = Self::device_supports_r16float(&graphics.adapter);
        log::info!(
            "R16Float filterable sampling supported: {}. Defaulting to {}",
            default_float,
            if default_float { "R16Float" } else { "Rg8Unorm" }
        );

        crate::rendering::core::pipeline::set_swapchain_format(graphics.surface_config.format);

        let mut texture_pool = MeshTexturePool::new();

        {
            // Create initial depth texture and view for mesh rendering.
            // Function-level comment: This block initializes a depth attachment matching the current surface size.
            // Guard against zero-sized canvas on WASM to avoid WebGPU validation errors.
            let depth_format = crate::rendering::core::pipeline::get_mesh_depth_format();
            let width = graphics.surface_config.width;
            let height = graphics.surface_config.height;
            if width > 0 && height > 0 {
                let size = wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                };
                let desc = wgpu::TextureDescriptor {
                    label: Some("Mesh Depth Texture"),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: depth_format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                };
                let depth_tex = graphics.device.create_texture(&desc);
                let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());
                texture_pool.set_depth(depth_tex, depth_view);
            } else {
                warn!(
                    "Skipping initial mesh depth texture creation: surface size is {}x{} (expected >0). Will create after first resize.",
                    width, height
                );
            }
        }

        let surface_format = graphics.surface_config.format;
        Ok(Self {
            graphics,
            layout,
            enable_float_volume_texture: default_float,
            toggle_enabled: true,
            last_volume: None,
            enable_mesh: false,
            texture_pool: texture_pool,
            mpr_state_slot2: None,
            mesh_ctx: None,
            pass_executor: crate::rendering::core::PassExecutor::new(surface_format),
        })
    }

    pub fn swap_graphics(&mut self, new_graphics: Graphics) {
        self.graphics = new_graphics;
        crate::rendering::core::pipeline::set_swapchain_format(self.graphics.surface_config.format);
        
        // Function-level comment: Clear mesh resources bound to old device to prevent stale references.
        self.clear_mesh_context_cache();
        self.texture_pool.clear_depth_view();
        
        // self.resize(winit::dpi::PhysicalSize {
        //     width: self.graphics.surface_config.width,
        //     height: self.graphics.surface_config.height,
        // });
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Loads local DICOM data and forwards PipelineManager for pipeline retrieval.
    /// Native-only helper used during development/testing.
    pub async fn load_data(&mut self, manager: &mut PipelineManager) {
        let repo = {
            // Start the timer
            let start_time = Instant::now();

            let file_names = list_files_in_directory("C:\\share\\imrt").unwrap();
            let repo =
                fileio::parse_dcm_directories(vec!["C:\\share\\imrt", "C:\\share\\head_mold"])
                    .await
                    .unwrap();
            println!("DicomRepo:\n{}", repo.to_string());
            println!("Patients:\n{:?}", repo.get_all_patients());
            // Stop the timer
            let elapsed_time = start_time.elapsed();

            // Print the repository and performance details
            // println!("Parsed repository: {:?}", repo);
            println!(
                "Parsing completed in {:.1} ms.",
                elapsed_time.as_millis_f32()
            );
            repo
        };
        self.load_data_from_repo(
            manager,
            &repo,
            "1.2.392.200036.9116.2.5.1.144.3437232930.1426478676.964561",
        );
    }

    pub fn window(&self) -> &Window {
        &self.graphics.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        println!("Resizing to: {}, {}", new_size.width, new_size.height);
        if new_size.width > 0 && new_size.height > 0 {
            // self.size = new_size;
            self.graphics.surface_config.width = new_size.width;
            self.graphics.surface_config.height = new_size.height;

            self.layout.resize((new_size.width, new_size.height));

            #[cfg(target_arch = "wasm32")]
            {
                // sets the style width and height of the window canvas
                let _ = self.graphics.window.request_inner_size(new_size); 
            }
            self.graphics.surface.configure(&self.graphics.device, &self.graphics.surface_config);
            
            // Update PassExecutor with new surface format
            self.pass_executor.update_surface_format(self.graphics.surface_config.format);

            // Recreate depth texture to match new surface size
            let depth_format = crate::rendering::core::pipeline::get_mesh_depth_format();
            let size = wgpu::Extent3d {
                width: self.graphics.surface_config.width,
                height: self.graphics.surface_config.height,
                depth_or_array_layers: 1,
            };
            let desc = wgpu::TextureDescriptor {
                label: Some("Mesh Depth Texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: depth_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            };
            let depth_tex = self.graphics.device.create_texture(&desc);
            let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());
            self.texture_pool.set_depth(depth_tex, depth_view);
        }
    }

    #[allow(unused_variables)]
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    pub fn update(&mut self) {
        self.layout.update(&self.graphics.queue);
    }

    /// Function-level comment: Check if the layout contains any MIP views for MIP pass execution.
    fn has_mip_content(&self) -> bool {
        self.layout.views.iter().any(|view| {
            view.as_any().downcast_ref::<crate::rendering::mip::MipView>().is_some()
        })
    }

    /// Function-level comment: Renders the frame using separate render passes for 3D mesh and 2D slice content.
    /// This architecture provides better performance and cleaner separation of concerns.
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.graphics.surface.get_current_texture()?;
        let frame_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Create command encoder for render passes
        let mut encoder = self.graphics.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Determine mesh settings
        let mesh_enabled = self.enable_mesh;

        // Function-level comment: Check if MIP content is available in the layout
        let mip_enabled = true; // MIP re-enabled after fixing LoadOp::Clear issue
        let has_mip_content = self.has_mip_content();

        // Create texture pool for this frame
        let texture_pool = &mut self.texture_pool;

        // Function-level comment: Check if mesh content is available and reset error state if needed
        let has_mesh_content = self.layout.views.len() > 2 && 
            self.layout.views[2].as_any().downcast_ref::<crate::rendering::view::MeshView>().is_some();
        
        // Debug logging for pass execution conditions
        trace!("Pass conditions - mesh_enabled: {}, has_mesh_content: {}, mip_enabled: {}, has_mip_content: {}, views_len: {}", 
               mesh_enabled, has_mesh_content, mip_enabled, has_mip_content, self.layout.views.len());
        
        if self.layout.views.len() > 2 {
            let view_type = if self.layout.views[2].as_any().downcast_ref::<view::MeshView>().is_some() {
                "MeshView"
            } else {
                "Other"
            };
            trace!("View at index 2 type: {}", view_type);
        }
        
        // Reset mesh pass error state if mesh is enabled and content is available
        if mesh_enabled && has_mesh_content && !self.pass_executor.is_healthy() {
            log::info!("Resetting mesh pass error state - mesh content available");
            self.pass_executor.reset_error_state();
        }

        // Execute frame using PassExecutor with separate render passes
        // We need to split the borrowing to avoid conflicts
        let device = &self.graphics.device;
        let layout = &mut self.layout;
        let pass_executor = &mut self.pass_executor;
        
        pass_executor.execute_frame(
            &mut encoder,
            &frame_view,
            texture_pool,
            device,
            self.graphics.surface_config.width,
            self.graphics.surface_config.height,
            mesh_enabled,
            has_mesh_content, // has_mesh_content - enable mesh pass when mesh content is available
            mip_enabled,
            has_mip_content, // has_mip_content - enable MIP pass when MIP views are present
            |pass_context| {
                match pass_context.pass_id {
                    crate::rendering::core::PassId::MeshPass => {
                        // Function-level comment: Render 3D mesh content by accessing MeshView from layout slot 2
                        if mesh_enabled && layout.views.len() > 2 {
                            // Access MeshView from slot 2 and attempt to downcast to mutable reference
                            let mesh_view = layout.views.get_mut(2)
                                .and_then(|view| view.as_any_mut().downcast_mut::<crate::rendering::view::MeshView>());
                            if let Some(mesh_view) = mesh_view {
                                // Call the MeshView render method with the pass context
                                mesh_view.render(pass_context.pass).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                            }
                        }
                        Ok(())
                    }
                    crate::rendering::core::PassId::MipPass => {
                        // Function-level comment: Render MIP content by finding and rendering MIP views in the layout
                        for view in layout.views.iter_mut() {
                            // Check if this view is a MipView and render it
                            if let Some(mip_view) = view.as_any_mut().downcast_mut::<crate::rendering::mip::MipView>() {
                                mip_view.render(pass_context.pass).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                            }
                        }
                        Ok(())
                    }
                    crate::rendering::core::PassId::SlicePass => {
                        // Function-level comment: Render 2D slice content (MPR views only, skip MeshView)
                        // Iterate through views and only render MPR views, not MeshView
                        for (_, view) in layout.views.iter_mut().enumerate() {
                            // Check if this is a MeshView and skip it during slice pass
                            if view.as_any().downcast_ref::<crate::rendering::view::MeshView>().is_some() {
                                continue;
                            }
                            // Render MPR views only
                            view.render(pass_context.pass).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                        }
                        Ok(())
                    }
                }
            },
        ).map_err(|e| {
            log::error!("PassExecutor error: {}", e);
            wgpu::SurfaceError::Lost
        })?;

        // Submit the command buffer
        self.graphics.queue.submit(std::iter::once(encoder.finish()));

        frame.present();
        Ok(())
    }

    pub fn load_data_from_ct_volume(&mut self, manager: &mut PipelineManager, vol: &CTVolume) {
        self.last_volume = Some(vol.clone());
        let texture = if self.enable_float_volume_texture {
            info!("Using R16Float volume texture path");
            // Convert voxel i16 values to half-float bytes
            let bytes: Vec<u8> = {
                let voxels_f16_bits: Vec<u16> = vol
                    .voxel_data
                    .iter()
                    .map(|&x| half::f16::from_f32(x as f32).to_bits())
                    .collect();
                bytemuck::cast_slice(&voxels_f16_bits).to_vec()
            };
            Arc::new(RenderContent::from_bytes_r16f(
                &self.graphics.device,
                &self.graphics.queue,
                &bytes,
                "CT Volume",
                vol.dimensions.0 as u32,
                vol.dimensions.1 as u32,
                vol.dimensions.2 as u32,
            ).unwrap())
        } else {
            info!("Using Rg8Unorm volume texture path");
            let voxel_data: Vec<u16> = vol
                .voxel_data
                .iter()
                .map(|x| (*x + HU_OFFSET as i16) as u16)
                .collect();
            let voxel_data: Vec<u8> = bytemuck::cast_slice(&voxel_data).to_vec();
            Arc::new(RenderContent::from_bytes(
                &self.graphics.device,
                &self.graphics.queue,
                &voxel_data,
                "CT Volume",
                vol.dimensions.0 as u32,
                vol.dimensions.1 as u32,
                vol.dimensions.2 as u32,
            ).unwrap())
        };

        self.layout.remove_all();

        if self.enable_mesh {
            // Add MPR views to slots 0 and 1 (Transverse and Coronal)
            for orientation in [ALL_ORIENTATIONS[0], ALL_ORIENTATIONS[1]].iter() {
                let render_context = Arc::new(crate::rendering::view::mpr::mpr_render_context::MprRenderContext::new(
                    manager,
                    &self.graphics.device,
                ));
                let view = MprView::new(
                    render_context,
                    &self.graphics.device,
                    texture.clone(),
                    &vol,
                    *orientation,
                    1.0,
                    [0.0, 0.0, 0.0],
                    (0, 0),
                    (0, 0),
                );
                self.layout.add_view(Box::new(view));
            }
            
            // Add Mesh view to slot 2 (third position - replacing Sagittal)
            let mesh_view = self.create_mesh_view(manager, (0, 0), (0, 0));
            self.layout.add_view(Box::new(mesh_view));
            
            // Add MIP view to slot 3 (fourth position - replacing Oblique)
            let mip_wgpu_impl = crate::rendering::MipViewWgpuImpl::new(
                texture.clone(),
                &self.graphics.device,
                self.graphics.surface_config.format,
            );
            let mip_view = crate::rendering::mip::MipView::new(Arc::new(mip_wgpu_impl));
            self.layout.add_view(Box::new(mip_view));
        } else {
            // Mesh disabled: add all four MPR views (including oblique)
            for orientation in ALL_ORIENTATIONS.iter() {
                let render_context = Arc::new(crate::rendering::view::mpr::mpr_render_context::MprRenderContext::new(
                    manager,
                    &self.graphics.device,
                ));
                let view = MprView::new(
                    render_context,
                    &self.graphics.device,
                    texture.clone(),
                    &vol,
                    *orientation,
                    1.0,
                    [0.0, 0.0, 0.0],
                    (0, 0),
                    (0, 0),
                );
                self.layout.add_view(Box::new(view));
            }
        }
    }

    pub fn load_data_from_repo(&mut self, manager: &mut PipelineManager, repo: &DicomRepo, image_series_number: &str) {
        let vol = repo.generate_ct_volume(image_series_number).unwrap();
        self.load_data_from_ct_volume(manager, &vol);
    }

    /// Function-level comment: Returns whether mesh mode is currently enabled.
    pub fn mesh_mode_enabled(&self) -> bool {
        self.enable_mesh
    }

    /// Function-level comment: Clear cached mesh context to force recreation and prevent buffer reference issues.
    /// This is useful when switching graphics contexts or when buffer errors occur.
    pub fn clear_mesh_context_cache(&mut self) {
        if self.mesh_ctx.is_some() {
            log::debug!("Clearing cached mesh context to prevent buffer reference issues");
            self.mesh_ctx = None;
        }
    }

    // /// Function-level comment: Enable or disable mesh mode at runtime by swapping the view at slot 2.
    // /// When enabling, replaces slot 2 with a MeshView and snapshots the previous MPR state.
    // /// When disabling, recreates a GenericMPRView for slot 2 and restores the cached MPR state if available.
    // pub fn set_mesh_mode_enabled(&mut self, manager: &mut crate::rendering::core::pipeline::PipelineManager, enabled: bool) {
    
    /// Function-level comment: Enable or disable mesh mode at runtime by rebuilding the layout appropriately.
    pub fn set_mesh_mode_enabled(&mut self, manager: &mut PipelineManager, enabled: bool) {
        if self.enable_mesh == enabled { 
            return; 
        }
        self.enable_mesh = enabled;

        if self.last_volume.is_none() {
            log::info!("Mesh mode set to {} without loaded volume; will apply on next data load.", enabled);
            return;
        }

        // Rebuild the entire layout to ensure proper view configuration
        if let Some(vol) = &self.last_volume.clone() {
            self.load_data_from_ct_volume(manager, vol);
            log::info!("Layout rebuilt for mesh mode: {}", enabled);
        }
    }

    /// Function-level comment: Validate that the specified view slot exists in the layout.
    fn validate_view_slot(&self, index: usize) -> bool {
        if self.layout.views.len() <= index {
            log::warn!("Expected at least {} views for slot {}; found {}. Toggle will not modify layout.", 
                      index + 1, index, self.layout.views.len());
            false
        } else {
            true
        }
    }

    /// Function-level comment: Calculate position and size for a view at the specified index.
    fn calculate_view_position_and_size(&self, index: usize) -> ((i32, i32), (u32, u32)) {
        let total_views = self.layout.views.len() as u32;
        let parent_dim = (self.graphics.surface_config.width, self.graphics.surface_config.height);
        self.layout.strategy.calculate_position_and_size(index as u32, total_views, parent_dim)
    }

    /// Function-level comment: Enable mesh mode by creating depth texture, saving MPR state, and creating MeshView.
    fn enable_mesh_mode(&mut self, manager: &mut PipelineManager, 
                       index: usize, pos: (i32, i32), size: (u32, u32)) {
        if !self.ensure_depth_texture() {
            return;
        }

        self.save_mpr_state(index);
        let mesh_view = self.create_mesh_view(manager, pos, size);
        self.layout.views[index] = Box::new(mesh_view);
        log::info!("MeshView placed at slot {} with position {:?} and size {:?}.", index, pos, size);
    }

    /// Function-level comment: Disable mesh mode by creating GenericMPRView and restoring MPR state.
    fn disable_mesh_mode(&mut self, manager: &mut PipelineManager, 
                        index: usize, pos: (i32, i32), size: (u32, u32)) {
        let mut view = self.create_mpr_view_for_slot(manager, index);
        self.restore_mpr_state(&mut view);
        
        view.move_to(pos);
        view.resize(size);
        self.layout.views[index] = Box::new(view);
        log::info!("GenericMPRView placed at slot {} with position {:?} and size {:?}.", index, pos, size);
        
        // Clear mesh context cache when disabling mesh mode to prevent buffer reference issues
        self.clear_mesh_context_cache();
    }

    /// Function-level comment: Ensure depth texture exists for mesh rendering, creating it if necessary.
    fn ensure_depth_texture(&mut self) -> bool {
        if self.texture_pool.depth_view().is_some() {
            return true;
        }

        let depth_format = crate::rendering::core::pipeline::get_mesh_depth_format();
        let width = self.graphics.surface_config.width;
        let height = self.graphics.surface_config.height;
        
        if width == 0 || height == 0 {
            log::warn!("Cannot create depth texture: surface size is {}x{} (expected >0).", width, height);
            return false;
        }

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("Mesh Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: depth_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };
        let depth_tex = self.graphics.device.create_texture(&desc);
        let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());
        self.texture_pool.set_depth(depth_tex, depth_view);
        log::info!("Created depth texture for mesh mode: {}x{} format {:?}", width, height, depth_format);
        true
    }

    /// Function-level comment: Save the current MPR state from the specified view slot.
    fn save_mpr_state(&mut self, index: usize) {
        if let Some(view) = self.layout.views.get_mut(index) {
            if let Some(mpr) = view.as_mpr() {
                let snap = MPRViewState {
                    window_level: mpr.get_window_level(),
                    window_width: mpr.get_window_width(),
                    slice_mm: mpr.get_slice_mm(),
                    scale: mpr.get_scale(),
                    translate: mpr.get_translate(),
                    translate_in_screen_coord: mpr.get_translate_in_screen_coord(),
                };
                self.mpr_state_slot2 = Some(snap);
                log::info!("Saved MPR snapshot for slot {} prior to enabling mesh.", index);
            } else {
                self.mpr_state_slot2 = None;
            }
        }
    }

    /// Function-level comment: Restore MPR state to the specified view if a snapshot exists.
    fn restore_mpr_state(&mut self, view: &mut crate::rendering::view::MprView) {
        if let Some(snap) = &self.mpr_state_slot2 {
            view.set_window_level(snap.window_level);
            view.set_window_width(snap.window_width);
            view.set_slice_mm(snap.slice_mm);
            view.set_scale(snap.scale);
            view.set_translate(snap.translate);
            view.set_translate_in_screen_coord(snap.translate_in_screen_coord);
            log::info!("Restored MPR snapshot for slot 2 after disabling mesh.");
        }
    }

    /// Function-level comment: Create a MeshView with cached or new BasicMeshContext.
    fn create_mesh_view(&mut self, manager: &mut PipelineManager, 
                       pos: (i32, i32), size: (u32, u32)) -> crate::rendering::view::MeshView {
        use std::sync::Arc;
        use crate::rendering::mesh::{mesh::Mesh, basic_mesh_context::BasicMeshContext};
        use crate::rendering::view::{MeshView, View as _};

        let mut mesh_view = MeshView::new();
        mesh_view.set_rotation_enabled(true);
        log::info!("Mesh rotation enabled");
        
        // Create or reuse cached Arc<BasicMeshContext> for efficient toggling
        let ctx_arc = if let Some(cached_ctx) = &self.mesh_ctx {
            // Reuse cached context
            cached_ctx.clone()
        } else {
            let mesh = Mesh::uniform_color_cube();
            let ctx = BasicMeshContext::new(
                manager,
                &self.graphics.device,
                &self.graphics.queue,
                &mesh,
                true, // Enable depth testing for proper 3D rendering
            );
            let ctx_arc = Arc::new(ctx);
            self.mesh_ctx = Some(ctx_arc.clone());
            ctx_arc
        };
        
        mesh_view.attach_context(ctx_arc);
        mesh_view.move_to(pos);
        mesh_view.resize(size);
        mesh_view
    }

    /// Function-level comment: Create a GenericMPRView for the specified slot with appropriate orientation.
    fn create_mpr_view_for_slot(&self, manager: &mut PipelineManager, 
                               index: usize) -> crate::rendering::view::MprView {
        use crate::rendering::view::{MprView, ALL_ORIENTATIONS};

        let vol = self.last_volume.as_ref().unwrap();
        let texture = self.create_volume_texture(vol);
        let orientation = ALL_ORIENTATIONS[index]; // Use index to determine orientation
        
        let render_context = Arc::new(crate::rendering::view::mpr::mpr_render_context::MprRenderContext::new(
            manager,
            &self.graphics.device,
        ));
        MprView::new(
            render_context,
            &self.graphics.device,
            texture,
            vol,
            orientation,
            1.0,
            [0.0, 0.0, 0.0],
            (0, 0),
            (0, 0),
        )
    }

    /// Function-level comment: Create volume texture based on current texture format settings.
    fn create_volume_texture(&self, vol: &crate::data::ct_volume::CTVolume) -> std::sync::Arc<crate::rendering::view::render_content::RenderContent> {
        use std::sync::Arc;
        use crate::rendering::view::render_content::RenderContent;

        if self.enable_float_volume_texture {
            log::info!("Using R16Float volume texture path (toggle)");
            let bytes: Vec<u8> = {
                let voxels_f16_bits: Vec<u16> = vol
                    .voxel_data
                    .iter()
                    .map(|&x| half::f16::from_f32(x as f32).to_bits())
                    .collect();
                bytemuck::cast_slice(&voxels_f16_bits).to_vec()
            };
            Arc::new(RenderContent::from_bytes_r16f(
                &self.graphics.device,
                &self.graphics.queue,
                &bytes,
                "CT Volume",
                vol.dimensions.0 as u32,
                vol.dimensions.1 as u32,
                vol.dimensions.2 as u32,
            ).unwrap())
        } else {
            log::info!("Using Rg8Unorm volume texture path (toggle)");
            let voxel_data: Vec<u16> = vol
                .voxel_data
                .iter()
                .map(|x| (*x + HU_OFFSET as i16) as u16)
                .collect();
            let voxel_data: Vec<u8> = bytemuck::cast_slice(&voxel_data).to_vec();
            Arc::new(RenderContent::from_bytes(
                &self.graphics.device,
                &self.graphics.queue,
                &voxel_data,
                "CT Volume",
                vol.dimensions.0 as u32,
                vol.dimensions.1 as u32,
                vol.dimensions.2 as u32,
            ).unwrap())
        }
    }

    // pub fn set_slice_speed(&mut self, index: usize, speed: f32) {
    //     let view = self.layout.views.get_mut(index).unwrap();
    //     if let Some(transverse_view) = view.as_any_mut().downcast_mut::<TransverseView>() {
    //         transverse_view.set_slice_speed(speed);
    //     }
    // }

    pub fn set_window_level(&mut self, index: usize, window_level: f32) {
        let view = self.layout.views.get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_mpr() {
            if self.enable_float_volume_texture {
                // Float path uses native HU values
                mpr_view.set_window_level(window_level);
            } else {
                // Packed RG8 path uses offset
                mpr_view.set_window_level(window_level + HU_OFFSET);
            }
            log::info!("View {} set_window_level: {}", index, window_level);
        }
    }

    pub fn set_window_width(&mut self, index: usize, window_width: f32) {
        let view = self.layout.views.get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_mpr() {
            mpr_view.set_window_width(window_width);
            log::info!("View {} set_window_width: {}", index, window_width);
        }
    }

    pub fn set_slice_mm(&mut self, index: usize, z: f32) {
        let view = self.layout.views.get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_mpr() {
            mpr_view.set_slice_mm(z);
            log::info!("View {} set_slice: {}", index, z);
        }
    }

    pub fn set_scale(&mut self, index: usize, scale: f32) {
        let view = self.layout.views.get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_mpr() {
            mpr_view.set_scale(scale);
            log::info!("View {} set_scale: {}", index, scale);
        }
    }

    pub fn set_translate(&mut self, index: usize, translate: [f32; 3]) {
        let view = self.layout.views.get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_mpr() {
            log::info!("View {} translate: {:#?}", index, translate);
            mpr_view.set_translate(translate);
        }
    }

    pub fn set_translate_in_screen_coord(&mut self, index: usize, translate: [f32; 3]) {
        let view = self.layout.views.get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_mpr() {
            log::info!("View {} move to: {:#?}", index, translate);
            mpr_view.set_translate_in_screen_coord(translate);
        }
    }

    pub fn set_pan(&mut self, index: usize, x: f32, y: f32 ) {
        let view = self.layout.views.get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_mpr() {
            log::info!("View {} move to: {:#?}", index, (x, y));
            mpr_view.set_pan(x, y);
        }
    }

    pub fn set_pan_mm(&mut self, index: usize, x_mm: f32, y_mm: f32 ) {
        let view = self.layout.views.get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_mpr() {
            log::info!("View {} move to mm: {:#?}", index, (x_mm, y_mm));
            mpr_view.set_pan(x_mm, y_mm);
        }
    }

    // Check if device supports R16Float with filtering and sampling as a texture binding
    fn device_supports_r16float(adapter: &wgpu::Adapter) -> bool {
        let features = adapter.get_texture_format_features(wgpu::TextureFormat::R16Float);
        let filterable = features
            .flags
            .contains(wgpu::TextureFormatFeatureFlags::FILTERABLE);
        let can_sample = features
            .allowed_usages
            .contains(wgpu::TextureUsages::TEXTURE_BINDING);
        filterable && can_sample
    }

    /// Function-level comment: Enable or disable Y-axis rotation for the mesh view in slot 2.
    /// This method provides external control over mesh rotation animation.
    pub fn set_mesh_rotation_enabled(&mut self, enabled: bool) {
        if self.layout.views.len() > 2 {
            if let Some(mesh_view) = self.layout.views[2].as_any_mut().downcast_mut::<crate::rendering::view::MeshView>() {
                mesh_view.set_rotation_enabled(enabled);
                log::info!("Mesh rotation {} via State control", if enabled { "enabled" } else { "disabled" });
            } else {
                log::warn!("Cannot control mesh rotation: slot 2 does not contain a MeshView");
            }
        } else {
            log::warn!("Cannot control mesh rotation: insufficient views (need at least 3, found {})", self.layout.views.len());
        }
    }

    /// Function-level comment: Set the rotation speed for the mesh view in slot 2.
    /// Speed is specified in radians per second. Use set_mesh_rotation_speed_degrees for degree-based input.
    pub fn set_mesh_rotation_speed(&mut self, speed_rad_per_sec: f32) {
        if self.layout.views.len() > 2 {
            if let Some(mesh_view) = self.layout.views[2].as_any_mut().downcast_mut::<crate::rendering::view::MeshView>() {
                mesh_view.set_rotation_speed(speed_rad_per_sec);
                log::info!("Mesh rotation speed set to {:.3} rad/s ({:.1}°/s) via State control", 
                           speed_rad_per_sec, speed_rad_per_sec.to_degrees());
            } else {
                log::warn!("Cannot set mesh rotation speed: slot 2 does not contain a MeshView");
            }
        } else {
            log::warn!("Cannot set mesh rotation speed: insufficient views (need at least 3, found {})", self.layout.views.len());
        }
    }

    /// Function-level comment: Set the rotation speed for the mesh view using degrees per second.
    /// This is a convenience method that converts degrees to radians internally.
    pub fn set_mesh_rotation_speed_degrees(&mut self, degrees_per_sec: f32) {
        self.set_mesh_rotation_speed(degrees_per_sec.to_radians());
    }

    /// Function-level comment: Reset the mesh rotation angle to zero.
    /// Useful for returning the mesh to its initial orientation.
    pub fn reset_mesh_rotation(&mut self) {
        if self.layout.views.len() > 2 {
            if let Some(mesh_view) = self.layout.views[2].as_any_mut().downcast_mut::<crate::rendering::view::MeshView>() {
                mesh_view.reset_rotation();
                log::info!("Mesh rotation angle reset via State control");
            } else {
                log::warn!("Cannot reset mesh rotation: slot 2 does not contain a MeshView");
            }
        } else {
            log::warn!("Cannot reset mesh rotation: insufficient views (need at least 3, found {})", self.layout.views.len());
        }
    }

    /// Function-level comment: Check if mesh rotation is currently enabled.
    /// Returns false if slot 2 doesn't contain a MeshView.
    pub fn is_mesh_rotation_enabled(&self) -> bool {
        if self.layout.views.len() > 2 {
            if let Some(mesh_view) = self.layout.views[2].as_any().downcast_ref::<crate::rendering::view::MeshView>() {
                mesh_view.is_rotation_enabled()
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Function-level comment: Get the current mesh rotation speed in radians per second.
    /// Returns 0.0 if slot 2 doesn't contain a MeshView.
    pub fn get_mesh_rotation_speed(&self) -> f32 {
        if self.layout.views.len() > 2 {
            if let Some(mesh_view) = self.layout.views[2].as_any().downcast_ref::<crate::rendering::view::MeshView>() {
                mesh_view.get_rotation_speed()
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Toggle the volume texture format (R16Float vs Rg8Unorm) and reload the CT volume using the given PipelineManager.
    /// Ensures hardware support when enabling float textures and reinitializes views.
    pub fn toggle_float_volume_texture(&mut self, manager: &mut PipelineManager) {
        if !self.toggle_enabled {
            log::warn!("Toggle feature is disabled; ignoring.");
            return;
        }
        // If enabling float path, ensure hardware support
        if !self.enable_float_volume_texture {
            if !Self::device_supports_r16float(&self.graphics.adapter) {
                log::warn!(
                    "Hardware doesn't support R16Float filtered sampling; staying on RG8."
                );
                return;
            }
        }
        self.enable_float_volume_texture = !self.enable_float_volume_texture;
        log::info!(
            "Toggled enable_float_volume_texture to {}",
            self.enable_float_volume_texture
        );
        if let Some(vol) = self.last_volume.clone() {
            // Clone to avoid borrowing self immutably while mutably reloading
            self.load_data_from_ct_volume(manager, &vol);
        } else {
            log::warn!("No cached CTVolume to reload after toggle.");
        }
    }

    pub fn disable_volume_format_toggle(&mut self) {
        self.toggle_enabled = false;
        log::info!(
            "Volume format toggle feature disabled. Default format in use: {}",
            if self.enable_float_volume_texture { "R16Float" } else { "Rg8Unorm" }
        );
    }
}

// ---------------------------------------- WASM ---------------------------------------------

#[cfg(target_arch = "wasm32")]
use js_sys::Array;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[cfg(target_arch = "wasm32")]
pub fn load_data_from_repo_wasm(/*repo: &DicomRepo,*/ image_series_number: &str) {
    warn!(".....................");
    warn!("Image Series Number: {image_series_number}");
    // let state = State::get_instance().await;
    // state.borrow_mut().load_data_from_repo(repo, image_series_number);
}
