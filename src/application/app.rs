#![allow(dead_code)]

use log::{info, trace, warn};

// ---------------------------------------- WASM ---------------------------------------------
use crate::rendering::{view, Graphics, GraphicsContext};
use std::path::PathBuf;
use std::sync::Arc;
use std::{fs, io};

// use wgpu::util::DeviceExt;
#[cfg(target_arch = "wasm32")]
use async_lock::Mutex;

use winit::{event::*, window::Window};

use crate::core::{error::KeplerError, WindowLevel};
use crate::data::dicom::*;
use crate::data::volume_encoding::VolumeEncoding;
use crate::data::{ct_volume::*, AppModel};
use crate::rendering::view::mesh::mesh_texture_pool::MeshTexturePool;
use crate::rendering::view::render_content::RenderContent;
use crate::rendering::view::*;
use glam::Mat4;
use std::f32::consts::PI;

// static STATE: Lazy<Arc<Mutex<Option<State>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));

// thread_local! {
//     static STATE: OnceCell<Rc<RefCell<State>>> = OnceCell::new();
// }

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::application::appview::AppView;

// #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
/// Main application logic and state management
pub struct App {
    /// Graphics context that encapsulates both hardware abstraction and rendering pipeline orchestration
    pub(crate) graphics_context: GraphicsContext,
    pub(crate) app_view: AppView,
    pub(crate) app_model: AppModel,
    pub(crate) cached_mesh: Option<crate::mesh::mesh::Mesh>,
    pub(crate) saved_states: [usize; 4],
}

impl App {
    pub async fn new(window: Arc<Window>) -> Result<App, KeplerError> {
        App::initialize(window).await
    }

    pub async fn initialize(window: Arc<Window>) -> Result<App, KeplerError> {
        let graphics = Graphics::new(window.clone()).await?;

        let layout = DynamicLayout::new(
            (
                graphics.surface_config.width,
                graphics.surface_config.height,
            ),
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
            if default_float {
                "R16Float"
            } else {
                "Rg8Unorm"
            }
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
            app_view: AppView::new(layout, factory),
            app_model: AppModel::new(default_float),
            cached_mesh: None,
            saved_states: [0; 4],
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
        crate::rendering::core::pipeline::set_swapchain_format(
            new_gc.graphics.surface_config.format,
        );
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
            self.app_model.enable_float_volume_texture,
        );
        log::info!("ViewFactory reinitialized after graphics swap.");
    }

    /// Get a reference to the window
    pub fn window(&self) -> &Window {
        &self.graphics_context.graphics().window
    }

    // Delegation methods for accessing Graphics through GraphicsContext
    // Function-level comment: These methods provide access to graphics resources through the GraphicsContext
    /// Access the underlying Graphics object
    pub fn graphics(&self) -> &Graphics {
        self.graphics_context.graphics()
    }

    /// Access the underlying Graphics object mutably
    pub fn graphics_mut(&mut self) -> &mut Graphics {
        self.graphics_context.graphics_mut()
    }

    /// Function-level comment: Resize the application window and update graphics resources.
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        // get max texture dimension
        let max_dim = self.graphics().device.limits().max_texture_dimension_2d as u32;

        let safe_width = new_size.width.min(max_dim);
        let safe_height = new_size.height.min(max_dim);
        if (safe_width != new_size.width) || (safe_height != new_size.height) {
            log::warn!(
                "please resize the window to ({}, {}) or smaller",
                safe_width, safe_height
            );
        }
        log::info!("Resizing to: {}, {}", safe_width, safe_height);

        if safe_width > 0 && safe_height > 0 {
            // self.size = new_size;
            self.graphics_mut().surface_config.width = safe_width;
            self.graphics_mut().surface_config.height = safe_height;

            self.app_view.layout.resize((safe_width, safe_height));

            #[cfg(target_arch = "wasm32")]
            {
                // sets the style width and height of the window canvas
                let _ = self
                    .window()
                    .request_inner_size(winit::dpi::PhysicalSize::new(safe_width, safe_height));
            }
            self.graphics()
                .surface
                .configure(&self.graphics().device, &self.graphics().surface_config);

            // Update PassExecutor with new surface format
            let surface_format = self.graphics().surface_config.format;
            self.graphics_context
                .pass_executor
                .update_surface_format(surface_format);

            // Recreate depth texture to match new surface size
            let depth_format = crate::rendering::core::pipeline::get_mesh_depth_format();
            let size = wgpu::Extent3d {
                width: self.graphics().surface_config.width,
                height: self.graphics().surface_config.height,
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
            let depth_tex = self.graphics().device.create_texture(&desc);
            let _ = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());
            // Texture pool is now created per-frame, so no persistent depth texture to update
        }
    }

    #[allow(unused_variables)]
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    pub fn update(&mut self) {
        self.app_view
            .layout
            .update(&self.graphics_context.graphics.queue);
    }

    /// Function-level comment: Check if the layout contains any MIP views for MIP pass execution.
    fn has_mip_content(&self) -> bool {
        self.app_view
            .layout
            .views()
            .iter()
            .any(|view| view.as_any().downcast_ref::<view::MipView>().is_some())
    }

    /// Function-level comment: Check if the layout contains any mesh views.
    fn has_mesh_view(&self) -> bool {
        self.app_view
            .layout
            .views()
            .iter()
            .any(|view| view.as_any().downcast_ref::<view::MeshView>().is_some())
    }

    /// Function-level comment: Check if the layout contains any MPR views.
    fn has_mpr_view(&self) -> bool {
        self.app_view
            .layout
            .views()
            .iter()
            .any(|view| view.as_any().downcast_ref::<view::MprView>().is_some())
    }

    /// Function-level comment: Renders the frame using separate render passes for 3D mesh and 2D slice content.
    /// This architecture provides better performance and cleaner separation of concerns.
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.graphics().surface.get_current_texture()?;
        let frame_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Create command encoder for render passes
        let mut encoder =
            self.graphics()
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        // Function-level comment: Determine which rendering passes to enable based on view types present in layout
        let has_mesh_view = self.has_mesh_view();
        let has_mip_view = self.has_mip_content();
        let has_mpr_view = self.has_mpr_view();

        // Debug logging for pass execution conditions
        trace!("View-driven pass conditions - has_mesh_view: {}, has_mip_view: {}, has_mpr_view: {}, views_len: {}", 
               has_mesh_view, has_mip_view, has_mpr_view, self.app_view.layout.views().len());

        // Reset mesh pass error state if mesh view is present and pass executor is unhealthy
        // Do this before borrowing texture_pool to avoid borrowing conflicts
        if has_mesh_view && !self.graphics_context.pass_executor.is_healthy() {
            log::info!("Resetting mesh pass error state - mesh view present in layout");
            self.graphics_context.pass_executor.reset_error_state();
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

        pass_executor
            .execute_frame(
                &mut encoder,
                &frame_view,
                &mut texture_pool,
                device,
                surface_width,
                surface_height,
                has_mesh_view, // Whether there is a mesh view present in the layout
                has_mip_view,  // Whether there is a MIP view present in the layout
                has_mpr_view,  // Whether there is an MPR view present in the layout
                |pass_context| {
                    match pass_context.pass_id {
                        crate::rendering::core::PassId::MeshPass => {
                            // Function-level comment: Render 3D mesh content by finding MeshView in the layout
                            for view in layout.views_mut().iter_mut() {
                                if let Some(mesh_view) =
                                    view.as_any_mut().downcast_mut::<MeshView>()
                                {
                                    // Call the MeshView render method with the pass context
                                    mesh_view
                                        .render(pass_context.pass)
                                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                                    break; // Only render the first mesh view found
                                }
                            }
                            Ok(())
                        }
                        crate::rendering::core::PassId::MipPass => {
                            // Function-level comment: Render MIP content by finding and rendering MIP views in the layout
                            for view in layout.views_mut().iter_mut() {
                                // Check if this view is a MipView and render it
                                if let Some(mip_view) = view.as_any_mut().downcast_mut::<MipView>()
                                {
                                    mip_view
                                        .render(pass_context.pass)
                                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
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
                                view.render(pass_context.pass)
                                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                            }
                            Ok(())
                        }
                    }
                },
            )
            .map_err(|e| {
                log::error!("PassExecutor error: {}", e);
                wgpu::SurfaceError::Lost
            })?;

        // Submit the command buffer
        self.graphics()
            .queue
            .submit(std::iter::once(encoder.finish()));

        frame.present();
        Ok(())
    }

    /// Internal helper to load volume and create RenderContent without modifying layout
    fn load_render_content(&mut self, vol: &CTVolume) -> Result<Arc<RenderContent>, KeplerError> {
        let _ = self.app_model.load_volume(vol.clone());
        let mut winlev;

        // Delegate data preparation to AppModel
        let (bytes, encoding) = self.app_model.get_volume_render_data()?;

        match encoding {
            VolumeEncoding::HuFloat => {
                winlev = WindowLevel::new();
                if let Err(e) = winlev.apply_bone_preset() {
                    log::warn!("apply_bone_preset (float path) failed: {}", e);
                }
                info!("Using R16Float volume texture path");

                Ok(Arc::new(RenderContent::from_bytes_r16f(
                    &self.graphics().device,
                    &self.graphics().queue,
                    &bytes,
                    "CT Volume",
                    vol.dimensions.0 as u32,
                    vol.dimensions.1 as u32,
                    vol.dimensions.2 as u32,
                    encoding,
                )?))
            }
            VolumeEncoding::HuPackedRg8 { offset } => {
                winlev = WindowLevel::new();
                if let Err(e) = winlev.set_bias(offset) {
                    log::warn!("set_bias (packed RG8 path) failed: {}", e);
                }
                if let Err(e) = winlev.apply_bone_preset() {
                    log::warn!("apply_bone_preset (packed RG8 path) failed: {}", e);
                }
                info!("Using Rg8Unorm volume texture path");

                Ok(Arc::new(RenderContent::from_bytes(
                    &self.graphics().device,
                    &self.graphics().queue,
                    &bytes,
                    "CT Volume",
                    vol.dimensions.0 as u32,
                    vol.dimensions.1 as u32,
                    vol.dimensions.2 as u32,
                    encoding,
                )?))
            }
        }
    }

    pub fn load_data_from_ct_volume(
        &mut self,
        vol: &CTVolume,
    ) -> Result<Arc<RenderContent>, KeplerError> {
        let texture = self.load_render_content(vol)?;
        let _ = self
            .app_view
            .reset_to_default_mpr_layout(texture.clone(), vol)
            .map_err(|e| KeplerError::Graphics(e.to_string()));
        self.saved_states = [0, 1, 2, 0];
        Ok(texture)
    }

    pub fn load_data_from_repo(&mut self, repo: &DicomRepo, image_series_number: &str) {
        let vol = repo.generate_ct_volume(image_series_number).unwrap();
        if let Err(e) = self.load_data_from_ct_volume(&vol) {
            log::error!("Failed to load data from repo: {}", e);
        }
    }

    /// Render mode setter for MPR, MIP, and Mesh.
    ///
    /// Function-level comment:
    /// This function replaces both `set_mesh_mode` and `set_mpr_mip_mode`, allowing unified control
    /// of rendering modes (MPR, MIP, Mesh). It automatically saves and restores view states
    /// when switching between single-cell and multi-cell layouts.
    ///
    /// Parameters:
    /// - mode: 0 = MPR, 1 = MIP, 2 = Mesh
    /// - save_mesh: if true, reuse cached mesh if available
    /// - crop: whether to crop the ROI with given world bounds
    /// - sx..lz: world bounds
    /// - one_cell: whether to switch to single-view layout
    /// - mesh_index: the target cell index for mesh view
    /// - iso_min, iso_max: ISO range for mesh extraction
    /// - mip: optional parameter for MIP config
    /// - orientation_index: orientation for MPR
    pub fn set_render_mode(
        &mut self,
        mode: usize,
        save_mesh: bool,
        crop: bool,
        sx: f32,
        sy: f32,
        sz: f32,
        lx: f32,
        ly: f32,
        lz: f32,
        mesh_index: Option<usize>,
        index: Option<usize>,
        iso_min: f32,
        iso_max: f32,
        mip_index: Option<usize>,
        orientation_index: usize,
    ) {
        // Save current view states before layout switch
        // self.app_view.save_view_states();

        // Prepare cropping region if requested
        let world_min = crop.then_some([sx, sy, sz]);
        let world_max = crop.then_some([lx, ly, lz]);

        // Layout management
        if mode == 2 {
            self.app_view.set_one_cell_layout();
            self.app_view.layout.remove_all();
        } else if self.app_view.is_one_cell_layout() {
            self.app_view.set_grid_layout(2, 2, 2);
        }
        
        // Load current volume
        if let Some(vol) = self.app_model.volume().ok().map(|v| v.clone()) {
            if self.saved_states.is_empty() {
                self.load_data_from_ct_volume(&vol).unwrap();
            }
            
            // Load render texture
            let texture = match self.load_render_content(&vol) {
                Ok(t) => t,
                Err(e) => {
                    log::error!("Failed to load render texture: {}", e);
                    return;
                }
            };

            if let Some(_) = mesh_index {
                self.app_model.enable_mesh = true;

                // Build or reuse cached mesh
                if !save_mesh || self.cached_mesh.is_none() {
                    let mut mesh = crate::rendering::view::mesh::mesh::Mesh::new(
                        &vol, iso_min, iso_max, world_min, world_max,
                    );

                    // Safety: Ensure non-empty mesh
                    if mesh.vertices.is_empty() {
                        log::warn!(
                            "Generated mesh is empty (ISO: {}-{}). Injecting dummy triangle.",
                            iso_min,
                            iso_max
                        );
                        let dummy = crate::rendering::view::mesh::mesh::MeshVertex {
                            position: [0.0, 0.0, 0.0],
                            normal: [0.0, 0.0, 1.0],
                            color: [0.0, 0.0, 0.0],
                        };
                        mesh.vertices.extend([dummy; 3]);
                        mesh.indices.extend([0, 1, 2]);
                    }
                    self.cached_mesh = Some(mesh);
                }
            }

            if let Some(idx) = index {
                self.saved_states[idx] = orientation_index;
            }

            // Switch rendering mode
            match mode {
                // === MPR ===
                0 => {
                    log::info!("Switching to MPR mode (orientation: {})", orientation_index);
                    let _ = self.app_view.set_layout_mode_single(
                        texture.clone(),
                        &vol,
                        0, // mode=0 for MPR
                        orientation_index,
                    );

                    // self.app_view.restore_view_states();
                }

                // === MIP ===
                1 => {
                    log::info!("Switching to MIP mode");
                    let _ = self.app_view.set_layout_mode_single(
                        texture.clone(),
                        &vol,
                        1, // mode=1 for MIP
                        orientation_index,
                    );

                    // self.app_view.restore_view_states();
                }

                // === Mesh ===
                2 => {
                    log::info!("Switching to Mesh mode");
                    // Create mesh view
                    let mesh_view = self
                        .app_view
                        .view_factory
                        .create_mesh_view_with_content(
                            texture,
                            self.cached_mesh.as_ref().expect("cached_mesh must exist"),
                            (0, 0),
                            (0, 0),
                        )
                        .expect("Failed to create mesh view");

                    self.app_view.layout.add_view(mesh_view);
                    // self.app_view.restore_view_states();
                }
                _ => {
                    let _ = self.app_view.configure_mesh_layout(
                        texture.clone(),
                        &vol,
                        self.saved_states,
                        mip_index,
                        mesh_index,
                        self.cached_mesh.clone(),
                    );

                    // self.app_view.restore_view_states();
                }
            }
        } else {
            log::info!(
                "MPR/MIP layout requested without loaded volume; will apply on next data load."
            );
        }
    }

    pub fn set_window_level(&mut self, index: usize, window_level: f32) {
        if let Err(e) = self.app_view.set_window_level(index, window_level) {
            log::warn!(
                "set_window_level {} failed on view {}: {}",
                if self.app_model.enable_float_volume_texture {
                    "(float)"
                } else {
                    "(packed RG8)"
                },
                index,
                e
            );
        } else {
            log::info!("View {} set_window_level: {}", index, window_level);
        }
    }

    pub fn set_window_width(&mut self, index: usize, window_width: f32) {
        if let Err(e) = self.app_view.set_window_width(index, window_width) {
            log::warn!("set_window_width failed on view {}: {}", index, e);
        } else {
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
        if let Err(e) = self.app_view.set_slice_mm(index, z) {
            log::warn!("set_slice_mm failed on view {}: {}", index, e);
        } else {
            log::info!("View {} set_slice: {}", index, z);
        }
    }

    pub fn set_scale(&mut self, index: usize, scale: f32) {
        if let Err(e) = self.app_view.set_scale(index, scale) {
            log::warn!("set_scale failed on view {}: {}", index, e);
        } else {
            log::info!("View {} set_scale: {}", index, scale);
        }
    }

    pub fn set_translate_in_screen_coord(&mut self, index: usize, translate: [f32; 3]) {
        if let Err(e) = self
            .app_view
            .set_translate_in_screen_coord(index, translate)
        {
            log::warn!(
                "set_translate_in_screen_coord failed on view {}: {}",
                index,
                e
            );
        } else {
            log::info!("View {} move to: {:#?}", index, translate);
        }
    }

    pub fn set_pan(&mut self, index: usize, x: f32, y: f32) {
        if let Err(e) = self.app_view.set_pan(index, x, y) {
            log::warn!("set_pan failed on view {}: {}", index, e);
        } else {
            log::info!("View {} pan to: {:#?}", index, (x, y));
        }
    }

    pub fn set_pan_mm(&mut self, index: usize, x_mm: f32, y_mm: f32) {
        if let Err(e) = self.app_view.set_pan_mm(index, x_mm, y_mm) {
            log::warn!("set_pan_mm failed on view {}: {}", index, e);
        } else {
            log::info!("View {} move to mm: {:#?}", index, (x_mm, y_mm));
        }
    }

    pub fn set_center_at_point_in_mm(&mut self, index: usize, x_mm: f32, y_mm: f32, z_mm: f32) {
        if let Err(e) = self
            .app_view
            .set_center_at_point_in_mm(index, [x_mm, y_mm, z_mm])
        {
            log::warn!("set_center_at_point_in_mm failed on view {}: {}", index, e);
        } else {
            log::info!(
                "View {} set_center_at_point_in_mm: {:#?}",
                index,
                (x_mm, y_mm, z_mm)
            );
        }
    }

    pub fn set_slab_thickness(&mut self, index: usize, thickness: f32) {
        if let Err(e) = self.app_view.set_slab_thickness(index, thickness) {
            log::warn!("set_slab_thickness failed on view {}: {}", index, e);
        } else {
            log::info!("View {} set_slab_thickness: {}", index, thickness);
        }
    }

    pub fn set_mip_mode(&mut self, index: usize, mip_mode: u32) {
        if let Err(e) = self.app_view.set_mip_mode(index, mip_mode) {
            log::warn!("set_mip_mode failed on view {}: {}", index, e);
        } else {
            log::info!("View {} set_mip_mode: {}", index, mip_mode);
        }
    }

    pub fn set_mip_rotation_angle_degrees(
        &mut self,
        index: usize,
        roll_deg: f32,
        yaw_deg: f32,
        pitch_deg: f32,
    ) {
        if let Err(e) = self
            .app_view
            .set_mip_rotation_angle_degrees(index, roll_deg, yaw_deg, pitch_deg)
        {
            log::warn!(
                "set_mip_rotation_angle_degrees failed on view {}: {}",
                index,
                e
            );
        } else {
            log::info!(
                "View {} set_mip_rotation_angle_degrees: roll_deg={}, yaw_deg={}, pitch_deg={}",
                index,
                roll_deg,
                yaw_deg,
                pitch_deg
            );
        }
    }

    pub fn get_oblique_normal(&self, index: usize)->[f32; 3]{
        let view = self.app_view.layout.views().get(index).unwrap();
        if let Some(mpr_view) = view.as_any().downcast_ref::<MprView>() {
            let n = mpr_view.get_oblique_normal();
            [n[0], n[1], n[2]]
        } else {
            [f32::NAN, f32::NAN, f32::NAN]
        }
    }

    pub fn set_oblique_normal(
        &mut self, 
        index: usize, 
        normal: [f32; 3], 
        in_plane_radians: f32
    ) {
        if let Some(view) = self.app_view.layout.views_mut().get_mut(index) {
            if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
                if let Err(e) = mpr_view.set_oblique_normal(normal, in_plane_radians) {
                    log::warn!("set_oblique_normal failed on view {}: {}", index, e);
                } else {
                    log::info!(
                        "View {} set_oblique_normal: normal={:?}, in_plane={}",
                        index,
                        normal,
                        in_plane_radians
                    );
                }
            }
        }
    }

    pub fn set_oblique_rotation_radians(
        &mut self,
        index: usize,
        horizontal_radians: f32,
        vertical_radians: f32,
        in_plane_radians: f32,
    ) {
        if let Some(view) = self.app_view.layout.views_mut().get_mut(index) {
            if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
                if let Err(e) = mpr_view.set_oblique_rotation_radians(
                    horizontal_radians,
                    vertical_radians,
                    in_plane_radians,
                ) {
                    log::warn!(
                        "set_oblique_rotation_radians failed on view {}: {}",
                        index,
                        e
                    );
                } else {
                    log::info!(
                        "View {} set_oblique_rotation_radians: horizontal={:?}, vertical={:?}, in_plane={:?}",
                        index,
                        horizontal_radians,
                        vertical_radians,
                        in_plane_radians
                    );
                }
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

    pub fn get_translate_in_screen_coord(&self, index: usize) -> [f32; 3] {
        let view = self.app_view.layout.views().get(index).unwrap();
        if let Some(mpr_view) = view.as_any().downcast_ref::<MprView>() {
            mpr_view.get_translate_in_screen_coord()
        } else {
            [f32::NAN, f32::NAN, f32::NAN]
        }
    }

    /// Function-level comment: Handle view click for cross-sectional linking between MPR views.
    /// When a user clicks on an MPR view, this method converts the screen coordinates to world coordinates
    /// and updates the slice positions of other MPR views to show the corresponding cross-sections.
    pub fn handle_view_click(
        &mut self,
        clicked_view_index: usize,
        screen_x: f32,
        screen_y: f32,
        screen_z: f32,
    ) -> [f32; 4] {
        // Default failure return uses NaN to indicate invalid result to the caller
        let mut result = [f32::NAN, f32::NAN, f32::NAN, f32::NAN];

        // Convert screen coordinates to world coordinates for the clicked view
        let (world_coord, slice_mm) = {
            let clicked_view = self
                .app_view
                .layout
                .views()
                .get(clicked_view_index)
                .unwrap();
            if let Some(mpr_view) = clicked_view.as_any().downcast_ref::<MprView>() {
                let world_coord = mpr_view.screen_coord_to_world([screen_x, screen_y, screen_z]);
                let slice = mpr_view.get_slice_mm();
                log::info!(
                    "View {} clicked at screen: {:#?}, world: {:#?}, slice: {}",
                    clicked_view_index,
                    (screen_x, screen_y, screen_z),
                    world_coord,
                    slice
                );
                (world_coord, slice)
            } else {
                warn!(
                    "handle_view_click: view {} is not an MprView",
                    clicked_view_index
                );
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
                    Orientation::Transverse => shift[2], // Z axis for axial (transverse)
                    Orientation::Coronal => shift[1],    // Y axis for coronal
                    Orientation::Sagittal => shift[0],   // X axis for sagittal
                    Orientation::Oblique => {
                        let n = mpr_view.get_oblique_normal();
                        n.x * shift[0] + n.y * shift[1] + n.z * shift[2]
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
                    log::info!(
                        "Mesh rotation {} via State control",
                        if enabled { "enabled" } else { "disabled" }
                    );
                    break;
                }
            }
        } else {
            log::warn!("Cannot control mesh rotation: no MeshView in layout");
        }
    }

    /// Helper method to apply an operation to the first available MeshView.
    fn apply_to_mesh_view<F>(&mut self, f: F)
    where
        F: FnOnce(&mut MeshView),
    {
        if let Some(view) = self
            .app_view
            .layout
            .views_mut()
            .iter_mut()
            .find_map(|v| v.as_any_mut().downcast_mut::<MeshView>())
        {
            f(view);
        } else {
            log::warn!("No MeshView found in layout");
        }
    }

    /// Set rotation speed (radians/sec) for the first MeshView.
    pub fn set_mesh_rotation_speed(&mut self, speed_rad_per_sec: f32) {
        self.apply_to_mesh_view(|mesh_view| {
            mesh_view.set_rotation_speed(speed_rad_per_sec);
            log::info!(
                "Mesh rotation speed set to {:.3} rad/s ({:.1}°/s) via State control",
                speed_rad_per_sec,
                speed_rad_per_sec.to_degrees()
            );
        });
    }

    /// Function-level comment: Reset the mesh rotation angle to zero.
    /// Useful for returning the mesh to its initial orientation.
    pub fn reset_mesh(&mut self) {
        self.apply_to_mesh_view(|mesh_view| {
            mesh_view.reset_rotation();
            mesh_view.reset_scale_factor();
            mesh_view.reset_pan();
            mesh_view.reset_opacity();
            log::info!("Mesh reset via State control");
        });
    }

    /// Set uniform mesh scale factor for the first MeshView present.
    pub fn set_mesh_scale(&mut self, scale: f32) {
        self.apply_to_mesh_view(|mesh_view| {
            mesh_view.set_scale_factor(scale);
            log::info!("Mesh scale set to {:.3}", scale);
        });
    }

    /// Function-level comment: Set the pan offset for the mesh view.
    /// dx, dy: Pan offsets in normalized device coordinates (-1 to 1 range).
    pub fn set_mesh_pan(&mut self, dx: f32, dy: f32) {
        self.apply_to_mesh_view(|mesh_view| {
            mesh_view.set_pan(dx, dy);
            log::info!("Mesh pan set to ({:.3}, {:.3})", dx, dy);
        });
    }

    pub fn set_mesh_opacity(&mut self, alpha: f32) {
        self.apply_to_mesh_view(|mesh_view| {
            mesh_view.set_opacity(alpha);
            log::info!("Mesh opacity set to {:.3}", alpha);
        });
    }

    /// Set mesh rotation angle in degrees for the first MeshView.
    pub fn set_mesh_rotation_angle_degrees(&mut self, degrees_x: f32, degrees_y: f32) {
        self.apply_to_mesh_view(|mesh_view| {
            mesh_view.set_rotation_angle_degrees(degrees_x, degrees_y);
        });
    }

    /// Apply a rotation delta to the first MeshView using mouse movement (pixels).
    pub fn set_mesh_rotation_degrees(&mut self, roll_deg: f32, yaw_deg: f32, pitch_deg: f32) {
        self.apply_to_mesh_view(|mesh_view| {
            mesh_view.set_rotation_degrees(roll_deg, yaw_deg, pitch_deg);
        });
    }

    pub fn get_mesh_rotation(&self) -> [f32; 16] {
        for view in self.app_view.layout.views().iter() {
            if let Some(mesh_view) = view
                .as_any()
                .downcast_ref::<crate::rendering::view::MeshView>()
            {
                return mesh_view.get_rotation().to_cols_array();
            }
        }
        Mat4::from_rotation_x(PI).to_cols_array()
    }

    pub fn set_mesh_rotation(&mut self, rotation: [f32; 16]) {
        self.apply_to_mesh_view(|mesh_view| {
            mesh_view.set_rotation(Mat4::from_cols_array(&rotation));
        });
    }

    /// Toggle the volume texture format (R16Float vs Rg8Unorm) and reload the CT volume.
    /// Ensures hardware support when enabling float textures and reinitializes views.
    pub fn toggle_float_volume_texture(&mut self) {
        // Toggle feature always enabled - removed toggle_enabled field
        // If enabling float path, ensure hardware support
        if !self.app_model.enable_float_volume_texture {
            if !Self::device_supports_r16float(&self.graphics().adapter) {
                log::warn!("Hardware doesn't support R16Float filtered sampling; staying on RG8.");
                return;
            }
        }
        self.app_model.enable_float_volume_texture = !self.app_model.enable_float_volume_texture;
        log::info!(
            "Toggled enable_float_volume_texture to {}",
            self.app_model.enable_float_volume_texture
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

        if let Err(e) = self.load_data_from_ct_volume(&vol) {
            log::error!("Failed to reload volume texture: {}", e);
        }
    }

    pub fn disable_volume_format_toggle(&mut self) {
        log::info!(
            "Volume format toggle feature disabled. Default format in use: {}",
            if self.app_model.enable_float_volume_texture {
                "R16Float"
            } else {
                "Rg8Unorm"
            }
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
