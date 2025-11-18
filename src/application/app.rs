#![allow(dead_code)]

use log::{trace, info, warn};

// ---------------------------------------- WASM ---------------------------------------------
use std::path::PathBuf;
use std::{fs, io};
use std::sync::Arc;
use crate::mesh::mesh::Mesh;
use crate::rendering::{view, Graphics, GraphicsContext};

// use wgpu::util::DeviceExt;
#[cfg(target_arch = "wasm32")]
use async_lock::Mutex;

use winit::{
    event::*,
    window::Window,
};

use crate::data::{AppModel, ct_volume::*};
use crate::data::dicom::*;
use crate::rendering::view::render_content::RenderContent;
use crate::rendering::view::*;
use crate::core::{error::KeplerError, WindowLevel};
use crate::rendering::mesh::mesh_texture_pool::MeshTexturePool;


// static STATE: Lazy<Arc<Mutex<Option<State>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));

// thread_local! {
//     static STATE: OnceCell<Rc<RefCell<State>>> = OnceCell::new();
// }

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::application::appview::AppView;

// #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct App {
    /// Graphics context that encapsulates both hardware abstraction and rendering pipeline orchestration
    pub(crate) graphics_context: GraphicsContext,
    pub(crate) app_view: AppView,
    pub(crate) enable_float_volume_texture: bool,

    pub(crate) enable_mesh: bool,

    pub(crate) app_model: AppModel,
}

impl App {
    const HU_OFFSET: f32 = 1100.0;
    
    pub async fn new(window: Arc<Window>) -> Result<App, KeplerError> {
        App::initialize(window).await
    }

    pub async fn initialize(window: Arc<Window>) -> Result<App, KeplerError> {
        let graphics = Graphics::new(window.clone()).await?;
        // println!("supported texture formats: {:?}", surface_caps.formats);
        // println!("format: {:?}", config.format);

        let layout = DynamicLayout::new(
            (graphics.surface_config.width, graphics.surface_config.height),
            Box::new(GridLayout {
                rows: 2,
                cols: 2,
                spacing: 2,}
            ),
        );

        // Choose default format based on device capability: prefer R16Float when supported, else RG8
        let default_float = Self::device_supports_r16float(&graphics.adapter);
        log::info!(
            "R16Float filterable sampling supported: {}. Defaulting to {}",
            default_float,
            if default_float { "R16Float" } else { "Rg8Unorm" }
        );

        crate::rendering::core::pipeline::set_swapchain_format(graphics.surface_config.format);
        

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
            enable_float_volume_texture: default_float,

            enable_mesh: false,

            app_view: AppView::new(layout, factory),
            app_model: AppModel::new(),
        })
    }

    pub fn swap_graphics(&mut self, new_graphics: Graphics) {
        // Function-level comment: Swap the underlying Graphics (window/surface/device/queue) safely.
        // This replaces the graphics inside GraphicsContext and updates dependent state:
        // - PassExecutor is recreated with the new surface format to keep pipelines targeting the correct format.
        // - Global swapchain format is updated for modules that query it.
        // - Mesh depth view is cleared to avoid stale references across device changes.

        // Recreate GraphicsContext from new Graphics to ensure PassExecutor targets the new surface format
        let new_gc = crate::rendering::core::graphics::GraphicsContext::from_graphics(new_graphics);
        // Update global swapchain format for pipeline helpers
        crate::rendering::core::pipeline::set_swapchain_format(new_gc.graphics.surface_config.format);
        // Replace the graphics_context
        self.graphics_context = new_gc;
        
        // Function-level comment: Clear mesh resources bound to old device to prevent stale references.
        // Texture pool is now created per-frame, so no persistent state to clear
        // Function-level comment: Reinitialize the DefaultViewFactory with the new device/queue to avoid cross-device resource mismatches on WASM.
        // This fixes a panic where a TextureView created on the new device was used to create a bind group on the old device.
        self.app_view.view_factory = crate::rendering::view::DefaultViewFactory::new(
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

    /// Function-level comment: Resize the application window and update graphics resources.
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        println!("Resizing to: {}, {}", new_size.width, new_size.height);
        if new_size.width > 0 && new_size.height > 0 {
            // self.size = new_size;
            self.surface_config_mut().width = new_size.width;
            self.surface_config_mut().height = new_size.height;

            self.app_view.layout.resize((new_size.width, new_size.height));

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
            // Texture pool is now created per-frame, so no persistent depth texture to update
        }
    }

    #[allow(unused_variables)]
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    pub fn update(&mut self) {
        self.app_view.layout.update(&self.graphics_context.graphics.queue);
    }

    /// Function-level comment: Check if the layout contains any MIP views for MIP pass execution.
    fn has_mip_content(&self) -> bool {
        self.app_view.layout.views().iter().any(|view| {
            view.as_any().downcast_ref::<view::MipView>().is_some()
        })
    }

    /// Function-level comment: Check if the layout contains any mesh views.
    fn has_mesh_view(&self) -> bool {
        self.app_view.layout.views().iter().any(|view| {
            view.as_any().downcast_ref::<view::MeshView>().is_some()
        })
    }

    /// Function-level comment: Check if the layout contains any MPR views.
    fn has_mpr_view(&self) -> bool {
        self.app_view.layout.views().iter().any(|view| {
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
        let has_mesh_view = self.enable_mesh;
        let has_mip_view = self.has_mip_content();
        let has_mpr_view = self.has_mpr_view();
        
        // Debug logging for pass execution conditions
        trace!("View-driven pass conditions - has_mesh_view: {}, has_mip_view: {}, has_mpr_view: {}, views_len: {}", 
               has_mesh_view, has_mip_view, has_mpr_view, self.app_view.layout.views().len());
        
        // Reset mesh pass error state if mesh view is present and pass executor is unhealthy
        // Do this before borrowing texture_pool to avoid borrowing conflicts
        if has_mesh_view && !self.pass_executor_is_healthy() {
            log::info!("Resetting mesh pass error state - mesh view present in layout");
            self.pass_executor_reset_error_state();
        }
        
        // Execute frame using PassExecutor with separate render passes
        // Extract all needed values and mutable references in one go to avoid borrowing conflicts
        // Create temporary texture pool per-frame; depth is ensured by pass executor
        let mut texture_pool = MeshTexturePool::new();
        let layout = &mut self.app_view.layout;
        
        let surface_width = self.graphics_context.graphics.surface_config.width;
        let surface_height = self.graphics_context.graphics.surface_config.height;
        let device = &self.graphics_context.graphics.device;
        let pass_executor = &mut self.graphics_context.pass_executor;
        
        pass_executor.execute_frame(
            &mut encoder,
            &frame_view,
            &mut texture_pool,
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

    pub fn load_data_from_ct_volume(&mut self, vol: &CTVolume)  -> Arc<RenderContent> {
        self.app_model.load_volume(vol.clone());
        let mut winlev;
        let texture = if self.enable_float_volume_texture {
            winlev = WindowLevel::new();
            if let Err(e) = winlev.apply_bone_preset() {
                log::warn!("apply_bone_preset (float path) failed: {}", e);
            }
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
            if let Err(e) = winlev.set_bias(Self::HU_OFFSET) {
                log::warn!("set_bias (packed RG8 path) failed: {}", e);
            }
            if let Err(e) = winlev.apply_bone_preset() {
                log::warn!("apply_bone_preset (packed RG8 path) failed: {}", e);
            }
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
    
        self.app_view.layout.remove_all();
        let view_factory = &self.app_view.view_factory;
        for orientation in [ALL_ORIENTATIONS[0], ALL_ORIENTATIONS[1], ALL_ORIENTATIONS[2], ALL_ORIENTATIONS[0]].iter() {
            let view = view_factory
                .create_mpr_view_with_content(
                    texture.clone(),
                    &vol,
                    *orientation,
                    (0, 0),
                    (0, 0),
                )
                .unwrap();
            self.app_view.layout.add_view(view);
        }

        // Return the shared render content for caller access
        texture
    }

    pub fn load_data_from_repo(&mut self, repo: &DicomRepo, image_series_number: &str) {
        let vol = repo.generate_ct_volume(image_series_number).unwrap();
        self.load_data_from_ct_volume(&vol);
    }

    /// Function-level comment: Returns whether mesh mode is currently enabled.
    pub fn mesh_mode_enabled(&self) -> bool {
        self.enable_mesh
    }

    /// Function-level comment: Enable or disable mesh mode at runtime by rebuilding the layout appropriately.
    pub fn set_mesh_mode_enabled(&mut self, enabled: bool, mip: Option<usize>, change_mpr: bool, index_1: usize, index_2: usize, index_3: usize, index_4: usize, downsample: usize) {
        if self.enable_mesh != enabled { 
            self.enable_mesh = enabled;
        }

        let mut change_index =false;
        if enabled || change_mpr {
            change_index = true;
        }
        if let Some(_) = mip {
            change_index = true;
        }

        if self.app_view.is_one_cell_layout() {
            self.app_view.set_grid_layout(2, 2, 2);
        }

        let vol_option = self.app_model.volume().ok().map(|vol| vol.clone());
    
        // Rebuild layout immediately if a volume is already loaded
        if let Some(vol) = vol_option {
            let texture = self.load_data_from_ct_volume(&vol.clone());
            if change_index {
                self.app_view.layout.remove_all();
                for orientation in [ALL_ORIENTATIONS[index_1], ALL_ORIENTATIONS[index_2], ALL_ORIENTATIONS[index_3], ALL_ORIENTATIONS[index_4]].iter() {
                    let view = self.app_view.view_factory
                        .create_mpr_view_with_content(
                            texture.clone(),
                            &vol,
                            *orientation,
                        (0, 0),
                        (0, 0),
                    )
                    .unwrap();
                self.app_view.layout.add_view(view);
                }

                if let Some(mip) = mip{
                    // Add MIP view to slot 3 using factory
                    let mip_view = self.app_view.view_factory
                        .create_mip_view_with_content(texture.clone(), (0, 0), (0, 0))
                        .unwrap();
                    self.app_view.layout.replace_view_at(mip, mip_view);
                }

                if enabled {
                    // Add Mesh view to slot 3 using factory
                    let mesh = Mesh::new(&vol, 100.0, Some(downsample), 0);
                    let mesh_view = self.app_view.view_factory
                        .create_mesh_view(&mesh, (0, 0), (0, 0))
                        .unwrap();
                    self.app_view.layout.replace_view_at(3, mesh_view);
                }
            }else {
                self.load_data_from_ct_volume(&vol.clone());
            }
            log::info!("Layout rebuilt for mode: {}", change_index);
        } else {
            log::info!("Mode set to {} without loaded volume; will apply on next data load.", change_index);
        }
    }

    /// Switch to a single-cell layout and display the requested view type (MPR/MIP/MESH).
    /// Mode: 0=MPR, 1=MIP, 2=MESH. For MPR, provide `orientation_index` to select orientation.
    /// For MESH, `iso_value`, `downsample`, and `vertex_precision` control isosurface extraction.
    pub fn set_one_cell_layout(
        &mut self,
        mode: usize,
        orientation_index: usize,
        downsample: usize,
    ) {
        let vol_option = self.app_model.volume().ok().map(|vol| vol.clone());
        if let Some(vol) = vol_option {
            let texture = self.load_data_from_ct_volume(&vol.clone());
            self.app_view.set_one_cell_layout();
            self.app_view.layout.remove_all();
            let view_factory = &self.app_view.view_factory;
            match mode {
                0 => {
                    let orientation = ALL_ORIENTATIONS[orientation_index];
                    let view = view_factory
                        .create_mpr_view_with_content(
                            texture.clone(),
                            &vol,
                            orientation,
                            (0, 0),
                            (0, 0),
                        )
                        .unwrap();
                    self.app_view.layout.add_view(view);
                }
                1 => {
                    let mip_view = view_factory
                        .create_mip_view_with_content(texture.clone(), (0, 0), (0, 0))
                        .unwrap();
                    self.app_view.layout.add_view(mip_view);
                }
                2 => {
                    let mesh = Mesh::new(&vol, 100.0, Some(downsample), 0);
                    let mesh_view = view_factory
                        .create_mesh_view(&mesh, (0, 0), (0, 0))
                        .unwrap();
                    self.app_view.layout.add_view(mesh_view);
                }
                _ => {
                    log::warn!("Unsupported mode {} for one-cell layout; defaulting to MPR.", mode);
                    let orientation = ALL_ORIENTATIONS[orientation_index];
                    let view = view_factory
                        .create_mpr_view_with_content(
                            texture.clone(),
                            &vol,
                            orientation,
                            (0, 0),
                            (0, 0),
                        )
                        .unwrap();
                    self.app_view.layout.add_view(view);
                }
            }

            log::info!(
                "Switched to one-cell layout: mode={}, orientation_index={}, one_cell={}",
                mode,
                orientation_index,
                self.app_view.is_one_cell_layout(),
            );
        } else {
            log::info!(
                "One-cell layout requested (mode={}) without loaded volume; will apply on next data load.",
                mode
            );
        }
    }
    
    /// Function-level comment: Calculate position and size for a view at the specified index.
    fn calculate_view_position_and_size(&self, index: usize) -> ((i32, i32), (u32, u32)) {
        let total_views = self.app_view.layout.views().len() as u32;
        let parent_dim = (self.surface_config().width, self.surface_config().height);
        self.app_view.layout.strategy().calculate_position_and_size(index as u32, total_views, parent_dim)
    }

    pub fn set_window_level(&mut self, index: usize, window_level: f32) {
        let view = self.app_view.layout.views_mut().get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
            if let Err(e) = mpr_view.set_window_level(window_level) {
                log::warn!("set_window_level {} failed on view {}: {}", 
                        if self.enable_float_volume_texture {"(float)"} else {"(packed RG8)"}, 
                        index, e);
            }
            log::info!("View {} set_window_level: {}", index, window_level);
        }
    }

    pub fn set_window_width(&mut self, index: usize, window_width: f32) {
        let view = self.app_view.layout.views_mut().get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
            if let Err(e) = mpr_view.set_window_width(window_width) {
                log::warn!("set_window_width failed on view {}: {}", index, e);
            }
            log::info!("View {} set_window_width: {}", index, window_width);
        }
    }

    pub fn get_window_level(&self, index: usize) -> [f32; 2] {
        let view = self.app_view.layout.views().get(index).unwrap();
        if let Some(mpr_view) = view.as_any().downcast_ref::<MprView>() {
            [mpr_view.get_window_level(), mpr_view.get_window_width()]
        } else {
            [f32::NAN, f32::NAN]
        }
    }

    pub fn set_slice_mm(&mut self, index: usize, z: f32) {
        let view = self.app_view.layout.views_mut().get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
            if let Err(e) = mpr_view.set_slice_mm(z) {
                log::warn!("set_slice_mm failed on view {}: {}", index, e);
            }
            log::info!("View {} set_slice: {}", index, z);
        }
    }

    pub fn set_scale(&mut self, index: usize, scale: f32) {
        let view = self.app_view.layout.views_mut().get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
            if let Err(e) = mpr_view.set_scale(scale) {
                log::warn!("set_scale failed on view {}: {}", index, e);
            }
            log::info!("View {} set_scale: {}", index, scale);
        }
    }

    pub fn set_translate_in_screen_coord(&mut self, index: usize, translate: [f32; 3]) {
        let view = self.app_view.layout.views_mut().get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
            log::info!("View {} move to: {:#?}", index, translate);
            // Handle potential error from translate operation to avoid unused Result warnings.
            if let Err(e) = mpr_view.set_translate_in_screen_coord(translate) {
                log::warn!("set_translate_in_screen_coord failed on view {}: {}", index, e);
            }
        }
    }

    pub fn set_pan(&mut self, index: usize, x: f32, y: f32 ) {
        let view = self.app_view.layout.views_mut().get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
            log::info!("View {} move to: {:#?}", index, (x, y));
            if let Err(e) = mpr_view.set_pan(x, y) {
                log::warn!("set_pan failed on view {}: {}", index, e);
            }
        }
    }

    pub fn set_pan_mm(&mut self, index: usize, x_mm: f32, y_mm: f32 ) {
        let view = self.app_view.layout.views_mut().get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
            log::info!("View {} move to mm: {:#?}", index, (x_mm, y_mm));
            if let Err(e) = mpr_view.set_pan_mm(x_mm, y_mm) {
                log::warn!("set_pan_mm failed on view {}: {}", index, e);
            }
        }
    }

    pub fn set_center_at_point_in_mm(&mut self, index: usize, x_mm: f32, y_mm: f32, z_mm: f32) {
        let view = self.app_view.layout.views_mut().get_mut(index).unwrap();
        if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
            log::info!("View {} set_center_at_point_in_mm: {:#?}", index, (x_mm, y_mm, z_mm));
            if let Err(e) = mpr_view.set_center_at_point_in_mm([x_mm, y_mm, z_mm]) {
                log::warn!("set_center_at_point_in_mm failed on view {}: {}", index, e);
            }
        }
    }

    /// Get screen coordinate in millimeters for the specified view
    pub fn get_screen_coord_in_mm(&self, index: usize, coord: [f32; 3]) -> [f32; 3] {
        if let Some(view) = self.app_view.layout.views().get(index) {
            if let Some(mpr_view) = view.as_any().downcast_ref::<MprView>() {
                return mpr_view.screen_coord_to_world(coord);
            }
        }
        // Return the original coordinate if view not found or not an MprView
        coord
    }

    /// Function-level comment: Handle view click for cross-sectional linking between MPR views.
    /// When a user clicks on an MPR view, this method converts the screen coordinates to world coordinates
    /// and updates the slice positions of other MPR views to show the corresponding cross-sections.
    pub fn handle_view_click(&mut self, clicked_view_index: usize, screen_x: f32, screen_y: f32, screen_z: f32) -> [f32; 4] {
        // Default failure return uses NaN to indicate invalid result to the caller
        let mut result = [f32::NAN, f32::NAN, f32::NAN, f32::NAN];
        
        // Convert screen coordinates to world coordinates for the clicked view
        let (world_coord, slice_mm) = {
            let clicked_view = self.app_view.layout.views().get(clicked_view_index).unwrap();
            if let Some(mpr_view) = clicked_view.as_any().downcast_ref::<MprView>() {
                let world_coord = mpr_view.screen_coord_to_world([screen_x, screen_y, screen_z]);
                let slice = mpr_view.get_slice_mm();
                log::info!("View {} clicked at screen: {:#?}, world: {:#?}, slice: {}", clicked_view_index, (screen_x, screen_y, screen_z), world_coord, slice);
                (world_coord, slice)
            }else {
                warn!("handle_view_click: view {} is not an MprView", clicked_view_index);
                ([f32::NAN, f32::NAN, f32::NAN], f32::NAN)
            }
        };

        // Update slice positions for all other MPR views
        for (index, view) in self.app_view.layout.views_mut().iter_mut().enumerate() {
            // Skip the clicked view itself
            if index == clicked_view_index {
                result[index] = slice_mm;
                continue; 
            }

            if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
                let shift = mpr_view.set_center_at_point_in_mm(world_coord).unwrap();

                let orientation = mpr_view.get_orientation();
                // Calculate slice position based on the orientation
                let slice_position = match orientation {
                    Orientation::Transverse => shift[2],      // Z axis for axial (transverse)
                    Orientation::Coronal => shift[1],         // Y axis for coronal
                    Orientation::Sagittal => shift[0],        // X axis for sagittal
                    Orientation::Oblique => {
                        // Oblique: fall back to Z-axis for slice; consider improving with normal projection
                        log::warn!("Oblique orientation: defaulting slice to Z-axis value for view {}", index);
                        shift[2]
                    }
                };
                result[index] = slice_position;
            }
        }
        
        log::info!("handle_view_click: result={:?}", result);
        result
    }

    /// Function-level comment: Convert world coordinates to screen coordinates for the specified view.
    /// This method is useful for mapping 3D world positions to 2D screen positions for rendering.
    pub fn world_coord_to_screen(&self, index: usize, world_coord: [f32; 3]) -> [f32; 3] {
        if let Some(view) = self.app_view.layout.views().get(index) {
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

    /// Function-level comment: Enable or disable Y-axis rotation for the mesh view.
    /// This method provides external control over mesh rotation animation.
    pub fn set_mesh_rotation_enabled(&mut self, enabled: bool) {
        if self.app_view.layout.views().len() > 0 {
            for view in self.app_view.layout.views_mut().iter_mut() {
                if let Some(mesh_view) = view.as_any_mut().downcast_mut::<MeshView>() {
                    mesh_view.set_rotation_enabled(enabled);
                    log::info!("Mesh rotation {} via State control", if enabled { "enabled" } else { "disabled" });
                    break;
                }
            }
        } else {
            log::warn!("Cannot control mesh rotation: no MeshView in layout");
        }
    }

    /// Function-level comment: Set the rotation speed for the mesh view.
    /// Speed is specified in radians per second. Use set_mesh_rotation_speed_degrees for degree-based input.
    pub fn set_mesh_rotation_speed(&mut self, speed_rad_per_sec: f32) {
        if self.app_view.layout.views().len() > 0 {
            for view in self.app_view.layout.views_mut().iter_mut() {
                if let Some(mesh_view) = view.as_any_mut().downcast_mut::<MeshView>() {
                    mesh_view.set_rotation_speed(speed_rad_per_sec);
                    log::info!("Mesh rotation speed set to {:.3} rad/s ({:.1}°/s) via State control", 
                            speed_rad_per_sec, speed_rad_per_sec.to_degrees());
                    break;
                }
            }
        } else {
            log::warn!("Cannot set mesh rotation speed: no MeshView in layout");
        }
    }

    /// Function-level comment: Reset the mesh rotation angle to zero.
    /// Useful for returning the mesh to its initial orientation.
    pub fn reset_mesh(&mut self) {
        if self.app_view.layout.views().len() > 0 {
            for view in self.app_view.layout.views_mut().iter_mut() {
                if let Some(mesh_view) = view.as_any_mut().downcast_mut::<MeshView>() {
                    mesh_view.reset_rotation();
                    mesh_view.reset_scale_factor();
                    mesh_view.reset_pan();
                    log::info!("Mesh reset via State control");
                    break;
                }
            }
        } else {
            log::warn!("Cannot reset mesh rotation: no MeshView in layout");
        }
    }

    /// Set uniform mesh scale factor for the first MeshView present.
    pub fn set_mesh_scale(&mut self, scale: f32) {
        if self.app_view.layout.views().len() > 0 {
            for view in self.app_view.layout.views_mut().iter_mut() {
                if let Some(mesh_view) = view.as_any_mut().downcast_mut::<MeshView>() {
                    mesh_view.set_scale_factor(scale);
                    log::info!("Mesh scale set to {:.3}", scale);
                    break;
                }
            }
        }else {
            log::warn!("Cannot set mesh scale: no MeshView in layout");
        }
    }

    /// Get current mesh scale factor; returns 0.0 if no MeshView present.
    pub fn get_mesh_scale(&self) -> f32 {
        for view in self.app_view.layout.views().iter() {
            if let Some(mesh_view) = view.as_any().downcast_ref::<crate::rendering::view::MeshView>() {
                return mesh_view.get_scale_factor();
            }
        }
        0.0
    }

    /// Function-level comment: Set the pan offset for the mesh view.
    /// dx, dy: Pan offsets in normalized device coordinates (-1 to 1 range).
    pub fn set_mesh_pan(&mut self, dx: f32, dy: f32) {
        if self.app_view.layout.views().len() > 0 {
            for view in self.app_view.layout.views_mut().iter_mut() {
                if let Some(mesh_view) = view.as_any_mut().downcast_mut::<MeshView>() {
                    mesh_view.set_pan(dx, dy);
                    break;
                }
            }
        }else {
            log::warn!("Cannot set mesh pan: no MeshView in layout");
        }
    }
    
    /// Set mesh rotation angle in degrees for the first MeshView.
    pub fn set_mesh_rotation_angle_degrees(&mut self, degrees_x: f32, degrees_y: f32, degrees_z: f32) {
        if self.app_view.layout.views().len() > 0 {
            for view in self.app_view.layout.views_mut().iter_mut() {
                if let Some(mesh_view) = view.as_any_mut().downcast_mut::<MeshView>() {
                    let degrees = [degrees_x, degrees_y, degrees_z];
                    mesh_view.set_rotation_angle_degrees(degrees);
                    break;
                }
            }
        }else {
            log::warn!("Cannot set mesh rotation angle: no MeshView in layout");
        }
    }

    /// Toggle the volume texture format (R16Float vs Rg8Unorm) and reload the CT volume.
    /// Ensures hardware support when enabling float textures and reinitializes views.
    pub fn toggle_float_volume_texture(&mut self) {
        // Toggle feature always enabled - removed toggle_enabled field
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
        let vol = {
            let vol = match self.app_model.volume() {
                Ok(v) => v,
                Err(e) => {
                    log::warn!("Failed to get volume: {}", e);
                    return;
                }   
            };
            vol.clone()
        };

        self.load_data_from_ct_volume(&vol);
        log::warn!("No cached CTVolume to reload after toggle.");
    }

    pub fn disable_volume_format_toggle(&mut self) {

        log::info!(
            "Volume format toggle feature disabled. Default format in use: {}",
            if self.enable_float_volume_texture { "R16Float" } else { "Rg8Unorm" }
        );
    }

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

    
    #[cfg(not(target_arch = "wasm32"))]
    /// Loads local DICOM data for pipeline creation.
    /// Native-only helper used during development/testing.
    pub async fn load_data(&mut self) {
        let repo = {
            // Start the timer

            use crate::{core::Instant, dicom::fileio};
            let start_time = Instant::now();
    
            let _file_names = Self::list_files_in_directory("C:\\share\\imrt").unwrap();
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
