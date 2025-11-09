#![allow(dead_code)]

use log::{trace, info, warn};
use std::path::PathBuf;
use std::sync::Arc;
use std::{fs, io};
use crate::core::timing::Instant;
use crate::rendering::{view, Graphics, GraphicsContext};

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
use crate::core::{error::KeplerError, WindowLevel};
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

use crate::application::app_model::AppModel;

pub struct AppView {
    pub(crate) layout: DynamicLayout,
    pub(crate) app: Arc<App>,
}

pub struct App {
    pub(crate) view: Arc<AppView>,
    pub(crate) doc: Arc<AppModel>,
}

// #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct State {
    /// Graphics context that encapsulates both hardware abstraction and rendering pipeline orchestration
    pub(crate) graphics_context: GraphicsContext,
    // pub(crate) layout: Layout<OneCellLayout>,
    pub(crate) layout: DynamicLayout,
    pub(crate) enable_float_volume_texture: bool,
    pub(crate) toggle_enabled: bool,
    pub(crate) last_volume: Option<CTVolume>,
    pub(crate) enable_mesh: bool,
    pub(crate) texture_pool: MeshTexturePool,
    pub(crate) view_factory: DefaultViewFactory,
}

impl State {
    const HU_OFFSET: f32 = 1100.0;
    
    pub async fn new(window: Arc<Window>) -> Result<State, KeplerError> {
        State::initialize(window).await
    }

    pub async fn initialize(window: Arc<Window>) -> Result<State, KeplerError> {
        let graphics = Graphics::new(window.clone()).await?;
        // println!("supported texture formats: {:?}", surface_caps.formats);
        // println!("format: {:?}", config.format);

        let layout = DynamicLayout::new(
            (graphics.surface_config.width, graphics.surface_config.height),
            Box::new(GridLayout {
                rows: 2,
                cols: 2,
                spacing: 2,
            }),
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

        // Create GraphicsContext which encapsulates both graphics and pass_executor
        let graphics_context = GraphicsContext::from_graphics(graphics);

        // Initialize DefaultViewFactory with current GPU resources and configuration
        let factory = crate::rendering::view::DefaultViewFactory::new(
            Arc::clone(&graphics_context.graphics.device),
            Arc::clone(&graphics_context.graphics.queue),
            graphics_context.graphics.surface_config.format,
            default_float,
        );
        
        Ok(Self {
            graphics_context,
            layout,
            enable_float_volume_texture: default_float,
            toggle_enabled: true,
            last_volume: None,
            enable_mesh: false,
            texture_pool: texture_pool,
            view_factory: factory,
        })
    }

    pub fn swap_graphics(&mut self, new_graphics: Graphics) {
        let new_gc = GraphicsContext::from_graphics(new_graphics);
        crate::rendering::core::pipeline::set_swapchain_format(self.surface_config().format);
        self.graphics_context = new_gc;
        
        // Function-level comment: Clear mesh resources bound to old device to prevent stale references.
        self.texture_pool.clear_depth_view();
        // Function-level comment: Reinitialize the DefaultViewFactory with the new device/queue to avoid cross-device resource mismatches on WASM.
        // This fixes a panic where a TextureView created on the new device was used to create a bind group on the old device.
        self.view_factory = crate::rendering::view::DefaultViewFactory::new(
            std::sync::Arc::clone(&self.graphics_context.graphics.device),
            std::sync::Arc::clone(&self.graphics_context.graphics.queue),
            self.graphics_context.graphics.surface_config.format,
            self.enable_float_volume_texture,
        );
        log::info!("ViewFactory reinitialized after graphics swap.");
        
        // self.resize(winit::dpi::PhysicalSize {
        //     width: self.surface_config().width,
        //     height: self.surface_config().height,
        // });
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Loads local DICOM data for pipeline creation.
    /// Native-only helper used during development/testing.
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

    /// Get a reference to the window
    pub fn window(&self) -> &Window {
        &self.graphics_context.graphics().window
    }

    // Delegation methods for accessing Graphics through GraphicsContext
    // Function-level comment: These methods provide access to graphics resources through the GraphicsContext
    
    /// Get a reference to the graphics device
    pub fn device(&self) -> &wgpu::Device {
        &self.graphics_context.graphics().device
    }

    /// Get a reference to the graphics queue
    pub fn queue(&self) -> &wgpu::Queue {
        &self.graphics_context.graphics().queue
    }

    /// Get a reference to the surface
    pub fn surface(&self) -> &wgpu::Surface<'static> {
        &self.graphics_context.graphics().surface
    }

    /// Get a reference to the surface configuration
    pub fn surface_config(&self) -> &wgpu::SurfaceConfiguration {
        &self.graphics_context.graphics().surface_config
    }

    /// Get a mutable reference to the surface configuration
    pub fn surface_config_mut(&mut self) -> &mut wgpu::SurfaceConfiguration {
        &mut self.graphics_context.graphics_mut().surface_config
    }

    /// Get a reference to the adapter
    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.graphics_context.graphics().adapter
    }

    /// Get a mutable reference to the PassExecutor
    pub fn pass_executor_mut(&mut self) -> &mut crate::rendering::core::PassExecutor {
        self.graphics_context.pass_executor_mut()
    }

    /// Function-level comment: Check if PassExecutor is healthy.
    pub fn pass_executor_is_healthy(&self) -> bool {
        self.graphics_context.pass_executor.is_healthy()
    }

    /// Function-level comment: Reset PassExecutor error state.
    pub fn pass_executor_reset_error_state(&mut self) {
        self.graphics_context.pass_executor.reset_error_state();
    }

    /// Function-level comment: Update PassExecutor surface format.
    pub fn pass_executor_update_surface_format(&mut self, format: wgpu::TextureFormat) {
        self.graphics_context.pass_executor.update_surface_format(format);
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        println!("Resizing to: {}, {}", new_size.width, new_size.height);
        if new_size.width > 0 && new_size.height > 0 {
            // self.size = new_size;
            self.surface_config_mut().width = new_size.width;
            self.surface_config_mut().height = new_size.height;

            self.layout.resize((new_size.width, new_size.height));

            #[cfg(target_arch = "wasm32")]
            {
                // sets the style width and height of the window canvas
                let _ = self.window().request_inner_size(new_size); 
            }
            self.surface().configure(self.device(), self.surface_config());
            
            // Update PassExecutor with new surface format
            let surface_format = self.surface_config().format;
            self.pass_executor_update_surface_format(surface_format);

            // Recreate depth texture to match new surface size
            let depth_format = crate::rendering::core::pipeline::get_mesh_depth_format();
            let size = wgpu::Extent3d {
                width: self.surface_config().width,
                height: self.surface_config().height,
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
            let depth_tex = self.device().create_texture(&desc);
            let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());
            self.texture_pool.set_depth(depth_tex, depth_view);
        }
    }

    #[allow(unused_variables)]
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    pub fn update(&mut self) {
        self.layout.update(&self.graphics_context.graphics.queue);
    }

    /// Function-level comment: Check if the layout contains any MIP views for MIP pass execution.
    fn has_mip_content(&self) -> bool {
        self.layout.views().iter().any(|view| {
            view.as_any().downcast_ref::<view::MipView>().is_some()
        })
    }

    /// Function-level comment: Check if the layout contains any mesh views.
    fn has_mesh_view(&self) -> bool {
        self.layout.views().iter().any(|view| {
            view.as_any().downcast_ref::<view::MeshView>().is_some()
        })
    }

    /// Function-level comment: Check if the layout contains any MPR views.
    fn has_mpr_view(&self) -> bool {
        self.layout.views().iter().any(|view| {
            view.as_any().downcast_ref::<view::MprView>().is_some()
        })
    }

    /// Function-level comment: Renders the frame using separate render passes for 3D mesh and 2D slice content.
    /// This architecture provides better performance and cleaner separation of concerns.
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface().get_current_texture()?;
        let frame_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Create command encoder for render passes
        let mut encoder = self.device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Function-level comment: Determine which rendering passes to enable based on view types present in layout
        let has_mesh_view = self.has_mesh_view();
        let has_mip_view = self.has_mip_content(); // Keep existing method name for MIP
        let has_mpr_view = self.has_mpr_view();
        
        // Debug logging for pass execution conditions
        trace!("View-driven pass conditions - has_mesh_view: {}, has_mip_view: {}, has_mpr_view: {}, views_len: {}", 
               has_mesh_view, has_mip_view, has_mpr_view, self.layout.views().len());
        
        // Reset mesh pass error state if mesh view is present and pass executor is unhealthy
        // Do this before borrowing texture_pool to avoid borrowing conflicts
        if has_mesh_view && !self.pass_executor_is_healthy() {
            log::info!("Resetting mesh pass error state - mesh view present in layout");
            self.pass_executor_reset_error_state();
        }
        
        // Execute frame using PassExecutor with separate render passes
        // Extract all needed values and mutable references in one go to avoid borrowing conflicts
        let texture_pool = &mut self.texture_pool;
        let layout = &mut self.layout;
        
        let surface_width = self.graphics_context.graphics.surface_config.width;
        let surface_height = self.graphics_context.graphics.surface_config.height;
        let device = &self.graphics_context.graphics.device;
        let pass_executor = &mut self.graphics_context.pass_executor;
        
        pass_executor.execute_frame(
            &mut encoder,
            &frame_view,
            texture_pool,
            device,
            surface_width,
            surface_height,
            has_mesh_view,    // Whether there is a mesh view present in the layout
            has_mip_view,     // Whether there is a MIP view present in the layout
            has_mpr_view,     // Whether there is an MPR view present in the layout
            |pass_context| {
                match pass_context.pass_id {
                    crate::rendering::core::PassId::MeshPass => {
                        // Function-level comment: Render 3D mesh content by finding MeshView in the layout
                        for view in layout.views_mut().iter_mut() {
                            if let Some(mesh_view) = view.as_any_mut().downcast_mut::<MeshView>() {
                                // Call the MeshView render method with the pass context
                                mesh_view.render(pass_context.pass).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                                break; // Only render the first mesh view found
                            }
                        }
                        Ok(())
                    }
                    crate::rendering::core::PassId::MipPass => {
                        // Function-level comment: Render MIP content by finding and rendering MIP views in the layout
                        for view in layout.views_mut().iter_mut() {
                            // Check if this view is a MipView and render it
                            if let Some(mip_view) = view.as_any_mut().downcast_mut::<MipView>() {
                                mip_view.render(pass_context.pass).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                            }
                        }
                        Ok(())
                    }
                    crate::rendering::core::PassId::SlicePass => {
                        // Function-level comment: Render 2D slice content (MPR views only, skip MeshView)
                        // Iterate through views and only render MPR views, not MeshView
                        for (_, view) in layout.views_mut().iter_mut().enumerate() {
                            // Check if this is a MeshView and skip it during slice pass
                            if view.as_any().downcast_ref::<MeshView>().is_some() {
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
        self.queue().submit(std::iter::once(encoder.finish()));

        frame.present();
        Ok(())
    }

    pub fn load_data_from_ct_volume(&mut self, vol: &CTVolume) {
        self.last_volume = Some(vol.clone());
        let mut winlev;
        let texture = if self.enable_float_volume_texture {
            winlev = WindowLevel::new();
            winlev.apply_bone_preset();
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
                self.device(),
                self.queue(),
                &bytes,
                "CT Volume",
                vol.dimensions.0 as u32,
                vol.dimensions.1 as u32,
                vol.dimensions.2 as u32,
            ).unwrap())
        } else {
            winlev = WindowLevel::new();
            winlev.set_bias(Self::HU_OFFSET);
            winlev.apply_bone_preset();
            info!("Using Rg8Unorm volume texture path");
            let voxel_data: Vec<u16> = vol
                .voxel_data
                .iter()
                .map(|x| (*x + Self::HU_OFFSET as i16) as u16)
                .collect();
            let voxel_data: Vec<u8> = bytemuck::cast_slice(&voxel_data).to_vec();
            Arc::new(RenderContent::from_bytes(
                self.device(),
                self.queue(),
                &voxel_data,
                "CT Volume",
                vol.dimensions.0 as u32,
                vol.dimensions.1 as u32,
                vol.dimensions.2 as u32,
            ).unwrap())
        };
    
        self.layout.remove_all();
    
        if self.enable_mesh {
            // Add MPR views to slots 0 and 1 (Transverse and Coronal) using factory
            for orientation in [ALL_ORIENTATIONS[0], ALL_ORIENTATIONS[1]].iter() {
                let view = self.view_factory
                    .create_mpr_view_with_content(
                        texture.clone(),
                        &vol,
                        *orientation,
                        (0, 0),
                        (0, 0),
                    )
                    .unwrap();
                self.layout.add_view(view);
            }

            // Add Mesh view to slot 2 (third position - replacing Sagittal) using factory
            let mesh = crate::rendering::mesh::mesh::Mesh::spine_vertebra();
            let mesh_view = self.view_factory
                .create_mesh_view(&mesh, (0, 0), (0, 0))
                .unwrap();
            self.layout.add_view(mesh_view);

            // Add MIP view to slot 3 (fourth position - replacing Oblique) using factory
            let mip_view = self.view_factory
                .create_mip_view_with_content(texture.clone(), (0, 0), (0, 0))
                .unwrap();
            self.layout.add_view(mip_view);
        } else {
            // Mesh disabled: add all four MPR views (including oblique) using factory
            for orientation in ALL_ORIENTATIONS.iter() {
                let view = self.view_factory
                    .create_mpr_view_with_content(
                        texture.clone(),
                        &vol,
                        *orientation,
                        (0, 0),
                        (0, 0),
                    )
                    .unwrap();
                self.layout.add_view(view);
            }
        }
    }

    pub fn load_data_from_repo(&mut self, repo: &DicomRepo, image_series_number: &str) {
        let vol = repo.generate_ct_volume(image_series_number).unwrap();
        self.load_data_from_ct_volume(&vol);
    }

    /// Function-level comment: Returns whether mesh mode is currently enabled.
    pub fn mesh_mode_enabled(&self) -> bool {
        self.enable_mesh
    }

    /// Function-level comment: Clear cached mesh context to force recreation and prevent buffer reference issues.


    /// Function-level comment: Enable or disable mesh mode at runtime by rebuilding the layout appropriately.
    pub fn set_mesh_mode_enabled(&mut self, enabled: bool) {
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
            self.load_data_from_ct_volume(vol);
            log::info!("Layout rebuilt for mesh mode: {}", enabled);
        }
    }

    /// Function-level comment: Calculate position and size for a view at the specified index.
    fn calculate_view_position_and_size(&self, index: usize) -> ((i32, i32), (u32, u32)) {
        let total_views = self.layout.views().len() as u32;
        let parent_dim = (self.surface_config().width, self.surface_config().height);
        self.layout.strategy().calculate_position_and_size(index as u32, total_views, parent_dim)
    }

    pub fn set_window_level(&mut self, index: usize, window_level: f32) {
        let view = self.layout.views_mut().get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
            if self.enable_float_volume_texture {
                // Float path uses native HU values
                mpr_view.set_window_level(window_level);
            } else {
                // Packed RG8 path uses offset
                mpr_view.set_window_level(window_level + Self::HU_OFFSET);
            }
            log::info!("View {} set_window_level: {}", index, window_level);
        }
    }

    pub fn set_window_width(&mut self, index: usize, window_width: f32) {
        let view = self.layout.views_mut().get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
            mpr_view.set_window_width(window_width);
            log::info!("View {} set_window_width: {}", index, window_width);
        }
    }

    pub fn set_slice_mm(&mut self, index: usize, z: f32) {
        let view = self.layout.views_mut().get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
            mpr_view.set_slice_mm(z);
            log::info!("View {} set_slice: {}", index, z);
        }
    }

    pub fn set_scale(&mut self, index: usize, scale: f32) {
        let view = self.layout.views_mut().get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
            mpr_view.set_scale(scale);
            log::info!("View {} set_scale: {}", index, scale);
        }
    }

    pub fn set_translate_in_screen_coord(&mut self, index: usize, translate: [f32; 3]) {
        let view = self.layout.views_mut().get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
            log::info!("View {} move to: {:#?}", index, translate);
            mpr_view.set_translate_in_screen_coord(translate);
        }
    }

    pub fn set_pan(&mut self, index: usize, x: f32, y: f32 ) {
        let view = self.layout.views_mut().get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
            log::info!("View {} move to: {:#?}", index, (x, y));
            mpr_view.set_pan(x, y);
        }
    }

    pub fn set_pan_mm(&mut self, index: usize, x_mm: f32, y_mm: f32 ) {
        let view = self.layout.views_mut().get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
            log::info!("View {} move to mm: {:#?}", index, (x_mm, y_mm));
            mpr_view.set_pan_mm(x_mm, y_mm);
        }
    }

    pub fn set_center_at_point_in_mm(&mut self, index: usize, x_mm: f32, y_mm: f32, z_mm: f32) {
        let view = self.layout.views_mut().get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
            log::info!("View {} set_center_at_point_in_mm: {:#?}", index, (x_mm, y_mm, z_mm));
            mpr_view.set_center_at_point_in_mm([x_mm, y_mm, z_mm]);
        }
    }

    /// Get screen coordinate in millimeters for the specified view
    pub fn get_screen_coord_in_mm(&self, index: usize, coord: [f32; 3]) -> [f32; 3] {
        if let Some(view) = self.layout.views().get(index) {
            if let Some(mpr_view) = view.as_any().downcast_ref::<MprView>() {
                return mpr_view.screen_coord_to_world(coord);
            }
        }
        // Return the original coordinate if view not found or not an MprView
        coord
    }

    pub fn world_coord_to_screen(&self, index: usize, world_coord: [f32; 3]) -> [f32; 3] {
        if let Some(view) = self.layout.views().get(index) {
            if let Some(mpr_view) = view.as_any().downcast_ref::<MprView>() {
                return mpr_view.world_coord_to_screen(world_coord);
            }
        }
        // Return the original coordinate if view not found or not an MprView
        world_coord
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
        if self.layout.views().len() > 2 {
            if let Some(mesh_view) = self.layout.views_mut()[2].as_any_mut().downcast_mut::<MeshView>() {
                mesh_view.set_rotation_enabled(enabled);
                log::info!("Mesh rotation {} via State control", if enabled { "enabled" } else { "disabled" });
            } else {
                log::warn!("Cannot control mesh rotation: slot 2 does not contain a MeshView");
            }
        } else {
            log::warn!("Cannot control mesh rotation: insufficient views (need at least 3, found {})", self.layout.views().len());
        }
    }

    /// Function-level comment: Set the rotation speed for the mesh view in slot 2.
    /// Speed is specified in radians per second. Use set_mesh_rotation_speed_degrees for degree-based input.
    pub fn set_mesh_rotation_speed(&mut self, speed_rad_per_sec: f32) {
        if self.layout.views().len() > 2 {
            if let Some(mesh_view) = self.layout.views_mut()[2].as_any_mut().downcast_mut::<MeshView>() {
                mesh_view.set_rotation_speed(speed_rad_per_sec);
                log::info!("Mesh rotation speed set to {:.3} rad/s ({:.1}°/s) via State control", 
                           speed_rad_per_sec, speed_rad_per_sec.to_degrees());
            } else {
                log::warn!("Cannot set mesh rotation speed: slot 2 does not contain a MeshView");
            }
        } else {
            log::warn!("Cannot set mesh rotation speed: insufficient views (need at least 3, found {})", self.layout.views().len());
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
        if self.layout.views().len() > 2 {
            if let Some(mesh_view) = self.layout.views_mut()[2].as_any_mut().downcast_mut::<MeshView>() {
                mesh_view.reset_rotation();
                log::info!("Mesh rotation angle reset via State control");
            } else {
                log::warn!("Cannot reset mesh rotation: slot 2 does not contain a MeshView");
            }
        } else {
            log::warn!("Cannot reset mesh rotation: insufficient views (need at least 3, found {})", self.layout.views().len());
        }
    }

    /// Function-level comment: Check if mesh rotation is currently enabled.
    /// Returns false if slot 2 doesn't contain a MeshView.
    pub fn is_mesh_rotation_enabled(&self) -> bool {
        if self.layout.views().len() > 2 {
            if let Some(mesh_view) = self.layout.views()[2].as_any().downcast_ref::<crate::rendering::view::MeshView>() {
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
        if self.layout.views().len() > 2 {
            if let Some(mesh_view) = self.layout.views()[2].as_any().downcast_ref::<crate::rendering::view::MeshView>() {
                mesh_view.get_rotation_speed()
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Toggle the volume texture format (R16Float vs Rg8Unorm) and reload the CT volume.
    /// Ensures hardware support when enabling float textures and reinitializes views.
    pub fn toggle_float_volume_texture(&mut self) {
        if !self.toggle_enabled {
            log::warn!("Toggle feature is disabled; ignoring.");
            return;
        }
        // If enabling float path, ensure hardware support
        if !self.enable_float_volume_texture {
            if !Self::device_supports_r16float(self.adapter()) {
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
