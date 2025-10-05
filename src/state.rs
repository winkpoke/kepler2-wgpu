use log::{debug, error, info, warn};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::{fs, io, time::Instant};

// use wgpu::util::DeviceExt;
#[cfg(target_arch = "wasm32")]
use async_lock::Mutex;
use once_cell::sync::Lazy;
use std::cell::{LazyCell, OnceCell, RefCell};
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::Mutex;
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

use crate::ct_volume::*;
use crate::dicom::*;
use crate::render_content::RenderContent;
use crate::view::*;
use crate::error::KeplerError;

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
    pub async fn initialize(window: Arc<Window>) -> Result<Graphics, KeplerError> {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            // backends: wgpu::Backends::PRIMARY,
            backends: wgpu::Backends::DX12,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            // backends: wgpu::BROWSER_WEBGPU,
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
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
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
                // Some(&std::path::Path::new("trace")), // Trace path
                None,
            )
            .await
            .map_err(|e| KeplerError::Graphics(format!("Failed to create device: {}", e)))?;

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an Srgb surface texture. Using a different
        // one will result all the colors comming out darker. If you want to support non
        // Srgb surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            // format: surface_format,
            format: wgpu::TextureFormat::Rgba8Unorm,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
        };

        if size.width > 0 && size.height > 0 {
            surface.configure(&device, &surface_config);
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
    #[cfg(feature = "mesh")]
    pub(crate) enable_mesh: bool,
}

const HU_OFFSET: f32 = 1100.0;

impl State {
    pub async fn new(window: Arc<Window>) -> Result<State, KeplerError> {
        State::initialize(window).await
    }

    pub async fn initialize(window: Arc<Window>) -> Result<State, KeplerError> {
        let graphics = Graphics::new(window.clone()).await?;
        // println!("supported texture formats: {:?}", surface_caps.formats);
        // println!("format: {:?}", config.format);

        let layout = Layout::new(
            (800, 800),
            GridLayout {
                rows: 2,
                cols: 2,
                spacing: 0,
            },
        );

        // Choose default format based on device capability: prefer R16Float when supported, else RG8
        let default_float = Self::device_supports_r16float(&graphics.adapter);
        log::info!(
            "R16Float filterable sampling supported: {}. Defaulting to {}",
            default_float,
            if default_float { "R16Float" } else { "Rg8Unorm" }
        );

        Ok(Self {
            graphics,
            layout,
            enable_float_volume_texture: default_float,
            toggle_enabled: true,
            last_volume: None,
            #[cfg(feature = "mesh")]
            enable_mesh: false,
        })
    }

    pub fn swap_graphics(&mut self, new_graphics: Graphics) {
        self.graphics = new_graphics;
        // self.resize(winit::dpi::PhysicalSize {
        //     width: self.graphics.surface_config.width,
        //     height: self.graphics.surface_config.height,
        // });
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn load_data(&mut self) {
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
        }
    }

    #[allow(unused_variables)]
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    pub fn update(&mut self) {
        self.layout.update(&self.graphics.queue);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.graphics.surface.get_current_texture()?;
        let frame_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .graphics.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.5,
                            g: 0.5,
                            b: 0.5,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            self.layout.render(&mut render_pass)?;
        }
        self.graphics.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }

    pub fn load_data_from_ct_volume(&mut self, vol: &CTVolume) {
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

        for orietation in ALL_ORIENTATIONS.iter() {
            let (pos, size) = self.layout.strategy.calculate_position_and_size(
                self.layout.views.len() as u32,
                (self.layout.views.len() + 1) as u32,
                (self.graphics.surface_config.width, self.graphics.surface_config.height),
            );
            info!("Adding view at position: {:?}, size: {:?}", pos, size);
            let view = GenericMPRView::new(
                &self.graphics.device,
                texture.clone(),
                &vol,
                *orietation,
                1.0,
                [0.0, 0.0, 0.0],
                pos,
                size,
            );
            self.layout.add_view(Box::new(view));
        }

        #[cfg(feature = "mesh")]
        if self.enable_mesh {
            use crate::view::MeshView;
            use crate::mesh::{mesh::Mesh, mesh_render_context::MeshRenderContext};
            let mut mesh_view = MeshView::new();
            let mesh = Mesh::unit_cube();
            let ctx = MeshRenderContext::new(&self.graphics.device, &self.graphics.queue, &mesh);
            mesh_view.attach_context(ctx);
            self.layout.add_view(Box::new(mesh_view));
        }
    }

    pub fn load_data_from_repo(&mut self, repo: &DicomRepo, image_series_number: &str) {
        let vol = repo.generate_ct_volume(image_series_number).unwrap();
        self.load_data_from_ct_volume(&vol);
    }

    pub fn set_slice_speed(&mut self, index: usize, speed: f32) {
        let view = self.layout.views.get_mut(index).unwrap();
        if let Some(transverse_view) = view.as_any_mut().downcast_mut::<TransverseView>() {
            transverse_view.set_slice_speed(speed);
        }
    }

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

    pub fn toggle_float_volume_texture(&mut self) {
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
            self.load_data_from_ct_volume(&vol);
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
