use std::rc::Rc;
use std::sync::Arc;
use std::{fs, io, time::Instant};
use std::path::{Path, PathBuf};
use log::{debug, error, info, warn};

// use wgpu::util::DeviceExt;
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};
use once_cell::sync::Lazy;
use std::cell::{LazyCell, OnceCell, RefCell};
#[cfg(target_arch = "wasm32")]
use async_lock::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::Mutex;


use crate::view::{CoronalView, GridLayout, OneCellLayout, Layout, MPRView, ObliqueView, Renderable, SagittalView, TransverseView};
use crate::ct_volume::*;
use crate::dicom::*;
use crate::render_content::RenderContent;





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

// #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct State {
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) config: wgpu::SurfaceConfiguration,
    pub(crate) size: winit::dpi::PhysicalSize<u32>,
    // The window must be declared after the surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    window: Arc<Window>,
    pub(crate) layout: Layout<OneCellLayout>,
    // pub(crate) layout: Layout<GridLayout>,
}

const HU_OFFSET: f32 = 1100.0;

impl State {
    pub async fn new(window: Arc<Window>, vol: &CTVolume) -> State {
        let mut state = State::initialize(window).await;
        state.load_data_from_ct_volume(vol);
        state
    }

    pub async fn initialize(window: Arc<Window>) -> State {
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

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

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
            .unwrap();

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
        let config = wgpu::SurfaceConfiguration {
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
        println!("format: {:?}", surface_format);

        error!("size: {}, {}", size.width, size.height);
        println!("print size: {}, {}", size.width, size.height);
        if size.width > 0 && size.height > 0 {
            surface.configure(&device, &config);
        }
        println!("supported texture formats: {:?}", surface_caps.formats);
        println!("format: {:?}", config.format);

        // #[cfg(target_arch = "wasm32")]
        // let repo = {
        //     let files = dicom::fileio::create_files_from_arrays(FILES);
        //     let repo = dicom::fileio::parse_dcm_files_wasm(files).await.unwrap();
        //     repo
        // };

        
        let layout = Layout::new((800, 800), OneCellLayout { rows: 1, cols: 1, spacing: 0 });

        let state = Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            layout,
        };
        // Self::set_instance(state).await;

        state
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn load_data(&mut self) {
        let repo = {
            // Start the timer
            let start_time = Instant::now();

            let file_names = list_files_in_directory("C:\\share\\imrt").unwrap();
            let repo = fileio::parse_dcm_directories(vec![
                "C:\\share\\imrt",
                "C:\\share\\head_mold",
            ])
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
        self.load_data_from_repo(&repo, "1.2.392.200036.9116.2.5.1.144.3437232930.1426478676.964561");
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    #[allow(unused_variables)]
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    pub fn update(&mut self) {
        self.layout.update(&self.queue);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let frame_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
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
        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }

    pub fn load_data_from_ct_volume(&mut self, vol: &CTVolume) {
        let voxel_data: Vec<u16> = vol.voxel_data.iter().map(|x| (*x + HU_OFFSET as i16) as u16).collect();
        let voxel_data: Vec<u8> = bytemuck::cast_slice(&voxel_data).to_vec();
        let texture = 
            RenderContent::from_bytes(
                &self.device,
                &self.queue,
                &voxel_data,
                "CT Volume",
                vol.dimensions.0 as u32,
                vol.dimensions.1 as u32,
                vol.dimensions.2 as u32,
            ).unwrap();
    
        self.layout.remove_all();

        let transverse_view = TransverseView::new(&self.device, &texture, &vol, 1.0, [0.0, 0.0, 0.0]);
        // let sagittal_view = SagittalView::new(&self.device, &texture, &vol, 1.0, [0.0, 0.0, 0.0], (900, 0), (300, 300));
        // let coronal_view = CoronalView::new(&self.device, &texture, &vol, 1.0, [0.0, 0.0, 0.0], (900, 300), (300, 300));
        // let oblique_view = ObliqueView::new(&self.device, &texture, &vol, 1.5, [150.0, 0.0, 0.0], (900, 600), (300, 300));
    
        self.layout.add_view(Box::new(transverse_view));
        // self.layout.add_view(Box::new(sagittal_view));
        // self.layout.add_view(Box::new(coronal_view));
        // self.layout.add_view(Box::new(oblique_view));
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
            mpr_view.set_window_level(window_level + HU_OFFSET);
            log::info!("TransverseView set_window_level: {}", window_level);
        }
    }

    pub fn set_window_width(&mut self, index: usize, window_width: f32) {
        let view = self.layout.views.get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_mpr() {
            mpr_view.set_window_width(window_width);
            log::info!("TransverseView set_window_width: {}", window_width);
        }
    }

    pub fn set_slice(&mut self, index: usize, slice: f32) {
        let view = self.layout.views.get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_mpr() {
            mpr_view.set_slice(slice);
            log::info!("TransverseView set_slice: {}", slice);
        }
    }

    pub fn set_scale(&mut self, index: usize, scale: f32) {
        let view = self.layout.views.get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_mpr() {
            mpr_view.set_scale(scale);
            log::info!("TransverseView set_scale: {}", scale);
        }
    }
    
    pub fn set_translate(&mut self, index: usize, translate: [f32;3]) {
        let view = self.layout.views.get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_mpr() {
            log::info!("TransverseView translate: {:#?}", translate);
            mpr_view.set_translate(translate);
        }
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
