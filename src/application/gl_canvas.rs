use winit::event_loop::EventLoopProxy;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::{data::ct_volume::CTVolume, rendering::core::Graphics};

#[cfg(target_arch = "wasm32")]
use futures::channel::oneshot;

#[derive(Debug)]
pub enum UserEvent {
    SetWindowLevel(usize, f32),
    SetWindowWidth(usize, f32),
    SetSliceMM(usize, f32),
    SetScale(usize, f32),
    SetTranslateInScreenCoord(usize, f32, f32, f32),
    SetPan(usize, f32, f32),   // pan in screen space
    SetPanMM(usize, f32, f32), // pan in mm
    LoadDataFromCTVolume(CTVolume),
    Resize(u32, u32), // width, height
    Quit,
    SetWindowByDivId(String, CTVolume),
    GraphicsReady(Graphics, CTVolume),
    ClearLayout,
    /// Manually trigger shader reload by invalidating pipelines; pipelines will be lazily rebuilt on next render.
    ReloadShaders,
    /// Manually trigger pipeline cache invalidation without any other action.
    InvalidatePipelines,
    SetRenderMode(
        usize,
        bool,
        bool,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        Option<usize>,
        Option<usize>,
        f32,
        f32,
        Option<usize>,
        usize,
    ),
    SetMipMode(usize, u32),
    SetOneCellLayout(usize, usize),
    #[cfg(target_arch = "wasm32")]
    GetObliqueNormal(usize, oneshot::Sender<[f32; 3]>),
    #[cfg(target_arch = "wasm32")]
    GetScreenCoordInMM(usize, [f32; 3], oneshot::Sender<[f32; 3]>),
    #[cfg(target_arch = "wasm32")]
    GetWindowLevel(usize, oneshot::Sender<[f32; 2]>),
    #[cfg(target_arch = "wasm32")]
    GetPan(usize, oneshot::Sender<[f32; 3]>),
    #[cfg(target_arch = "wasm32")]
    WorldCoordToScreen(usize, [f32; 3], oneshot::Sender<[f32; 3]>),
    SetSlabThickness(usize, f32),
    SetMipRotationAngleDeg(usize, f32, f32, f32),
    ViewClick(usize, f32, f32, f32), // view_index, screen_x, screen_y, screen_z
    SetObliqueNormal(usize,[f32;3],f32),
    SetObliqueRotation(usize, Option<f32>, Option<f32>, Option<f32>),
    #[cfg(target_arch = "wasm32")]
    /// View click with reply; returns [x_mm, y_mm, slice_mm, reserved]
    ViewClickGet(usize, f32, f32, f32, oneshot::Sender<[f32; 4]>),
    // Mesh control events
    SetMeshRotationEnabled(usize, bool),
    SetMeshOpacity(usize, f32),
    SetMeshPan(usize, f32, f32),
    ResetMesh(usize),
    SetMeshScale(usize, f32),
    SetMeshRotationAngleDeg(usize, f32, f32),
    SetMeshRotationDegrees(usize, f32, f32, f32),
    SetMeshRotation(usize, [f32; 16]),
    #[cfg(target_arch = "wasm32")]
    GetMeshRotation(usize, oneshot::Sender<[f32; 16]>),
}

#[macro_export]
macro_rules! impl_user_event_senders_for_glcanvas {
    (
        $( $fn_name:ident => $variant:ident( $( $arg:ident : $arg_ty:ty ),* ) ),* $(,)?
    ) => {
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
        impl GLCanvas {
            $(
                /// Sends a `UserEvent::$variant` targeted to a specific window index.
                pub fn $fn_name(&self, index: usize, $( $arg: $arg_ty ),* ) {
                    if let Err(e) = self.proxy.send_event(UserEvent::$variant(index, $( $arg ),*)) {
                        log::error!(
                            "Failed to send {} event for window {}: {:?}",
                            stringify!($variant),
                            index,
                            e
                        );
                    } else {
                        log::info!(
                            "Sent {} event for window {}: {:?}",
                            stringify!($variant),
                            index,
                            ( $( $arg ),* )
                        );
                    }
                }
            )*
        }
    };
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl GLCanvas {
    pub fn load_data_from_ct_volume(&self, volume: &CTVolume) {
        if let Err(e) = self
            .proxy
            .send_event(UserEvent::LoadDataFromCTVolume(volume.clone()))
        {
            log::error!("Failed to send LoadDataFromCTVolume event {:?}", e);
        } else {
            log::info!("Sent LoadDataFromCTVolume event");
        }
    }

    pub fn resize(&self, width: u32, height: u32) {
        if let Err(e) = self.proxy.send_event(UserEvent::Resize(width, height)) {
            log::error!("Failed to send Resize event: {:?}", e);
        } else {
            log::info!("Sent Resize event: width={}, height={}", width, height);
        }
    }

    pub fn quit(&self) {
        if let Err(e) = self.proxy.send_event(UserEvent::Quit) {
            log::error!("Failed to send Quit event: {:?}", e);
        } else {
            log::info!("Sent Quit event");
        }
    }

    pub fn set_window_by_div_id(&self, div_id: String, volume: &CTVolume) {
        if let Err(e) = self
            .proxy
            .send_event(UserEvent::SetWindowByDivId(div_id.clone(), volume.clone()))
        {
            log::error!(
                "Failed to send SetWindowByDivId event for div_id {}: {:?}",
                div_id,
                e
            );
        } else {
            log::info!("Sent SetWindowByDivId event for div_id {}", div_id);
        }
    }
    pub fn clear_layout(&self) {
        if let Err(e) = self.proxy.send_event(UserEvent::ClearLayout) {
            log::error!("Failed to send ClearLayout event: {:?}", e);
        } else {
            log::info!("Sent ClearLayout event");
        }
    }

    /// Sends a ReloadShaders event which will invalidate pipelines; they will be recreated as needed.
    pub fn reload_shaders(&self) {
        if let Err(e) = self.proxy.send_event(UserEvent::ReloadShaders) {
            log::error!("Failed to send ReloadShaders event: {:?}", e);
        } else {
            log::info!("Sent ReloadShaders event");
        }
    }

    /// Sends an InvalidatePipelines event without any shader changes.
    pub fn invalidate_pipelines(&self) {
        if let Err(e) = self.proxy.send_event(UserEvent::InvalidatePipelines) {
            log::error!("Failed to send InvalidatePipelines event: {:?}", e);
        } else {
            log::info!("Sent InvalidatePipelines event");
        }
    }

    pub fn set_render_mode(
        &self,
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
        if let Err(e) = self.proxy.send_event(UserEvent::SetRenderMode(
            mode,
            save_mesh,
            crop,
            sx,
            sy,
            sz,
            lx,
            ly,
            lz,
            mesh_index,
            index,
            iso_min,
            iso_max,
            mip_index,
            orientation_index,
        )) {
            log::error!("Failed to send SetRenderMode event: {:?}", e);
        } else {
            log::info!("Sent SetRenderMode event: mode={}, save_mesh={}, crop={}, sx={}, sy={}, sz={}, lx={}, ly={}, lz={}, mesh_index={:?}, index={:?}, iso_min={}, iso_max={}, orientation_index={}", mode, save_mesh, crop, sx, sy, sz, lx, ly, lz, mesh_index, index, iso_min, iso_max, orientation_index);
        }
    }

    pub fn set_one_cell_layout(&self, mode: usize, orientation_index: usize) {
        if let Err(e) = self
            .proxy
            .send_event(UserEvent::SetOneCellLayout(mode, orientation_index))
        {
            log::error!("Failed to send SetOneCellLayout event: {:?}", e);
        } else {
            log::info!(
                "Sent SetOneCellLayout event: mode={}, orientation_index={}",
                mode,
                orientation_index
            );
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn get_oblique_normal(&self, index: usize) -> Result<Box<[f32]>, String> {
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self.proxy.send_event(UserEvent::GetObliqueNormal(index, tx)) {
            log::error!(
                "Failed to send GetObliqueNormal event for window {}: {:?}",
                index,
                e
            );
            return Err(format!("Failed to send event: {:?}", e));
        }

        log::info!("Sent GetObliqueNormal event for window {}", index);

        match rx.await {
            Ok(result) => Ok(result.into()),
            Err(e) => Err(format!("Failed to receive result: {:?}", e)),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn get_screen_coord_in_mm(
        &self,
        index: usize,
        x: f32,
        y: f32,
        z: f32,
    ) -> Result<Box<[f32]>, String> {
        log::info!(
            "get_screen_coord_in_mm: index={}, x={}, y={}, z={}",
            index,
            x,
            y,
            z
        );
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self
            .proxy
            .send_event(UserEvent::GetScreenCoordInMM(index, [x, y, z], tx))
        {
            log::error!(
                "Failed to send GetScreenCoordInMM event for window {}: {:?}",
                index,
                e
            );
            return Err(format!("Failed to send event: {:?}", e));
        }

        log::info!(
            "Sent GetScreenCoordInMM event for window {}: {:?}",
            index,
            [x, y, z]
        );

        match rx.await {
            Ok(result) => {
                log::info!(
                    "Received GetScreenCoordInMM result for window {}: {:?}",
                    index,
                    result
                );
                Ok(result.into())
            }
            Err(e) => {
                log::error!(
                    "Failed to receive GetScreenCoordInMM result for window {}: {:?}",
                    index,
                    e
                );
                Err(format!("Failed to receive result: {:?}", e))
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn get_window_level(&self, index: usize) -> Result<Box<[f32]>, String> {
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self.proxy.send_event(UserEvent::GetWindowLevel(index, tx)) {
            log::error!(
                "Failed to send GetWindowLevel event for window {}: {:?}",
                index,
                e
            );
            return Err(format!("Failed to send event: {:?}", e));
        }

        log::info!("Sent GetWindowLevel event for window {}", index);

        match rx.await {
            Ok(result) => Ok(result.into()),
            Err(e) => Err(format!("Failed to receive result: {:?}", e)),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn get_pan(&self, index: usize) -> Result<Box<[f32]>, String> {
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self.proxy.send_event(UserEvent::GetPan(index, tx)) {
            log::error!("Failed to send GetPan event for window {}: {:?}", index, e);
            return Err(format!("Failed to send event: {:?}", e));
        }

        log::info!("Sent GetPan event for window {}", index);

        match rx.await {
            Ok(result) => Ok(result.into()),
            Err(e) => Err(format!("Failed to receive result: {:?}", e)),
        }
    }

    #[cfg(target_arch = "wasm32")]
    /// Dispatch a view click and asynchronously return computed world/slice data.
    pub async fn handle_view_click_get(
        &self,
        index: usize,
        x: f32,
        y: f32,
        z: f32,
    ) -> Result<Box<[f32]>, String> {
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self
            .proxy
            .send_event(UserEvent::ViewClickGet(index, x, y, z, tx))
        {
            log::error!(
                "Failed to send ViewClickGet event for window {}: {:?}",
                index,
                e
            );
            return Err(format!("Failed to send event: {:?}", e));
        }
        match rx.await {
            Ok(result) => Ok(result.into()),
            Err(e) => Err(format!("Failed to receive result: {:?}", e)),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn world_coord_to_screen(
        &self,
        index: usize,
        x: f32,
        y: f32,
        z: f32,
    ) -> Result<Box<[f32]>, String> {
        log::info!(
            "world_coord_to_screen: index={}, x={}, y={}, z={}",
            index,
            x,
            y,
            z
        );
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self
            .proxy
            .send_event(UserEvent::WorldCoordToScreen(index, [x, y, z], tx))
        {
            log::error!(
                "Failed to send WorldCoordToScreen event for window {}: {:?}",
                index,
                e
            );
            return Err(format!("Failed to send event: {:?}", e));
        }

        log::info!(
            "Sent WorldCoordToScreen event for window {}: {:?}",
            index,
            [x, y, z]
        );

        match rx.await {
            Ok(result) => {
                log::info!(
                    "Received WorldCoordToScreen result for window {}: {:?}",
                    index,
                    result
                );
                Ok(result.into())
            }
            Err(e) => {
                log::error!(
                    "Failed to receive WorldCoordToScreen result for window {}: {:?}",
                    index,
                    e
                );
                Err(format!("Failed to receive result: {:?}", e))
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn get_mesh_rotation(&self, index: usize) -> Result<Box<[f32]>, String> {
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self.proxy.send_event(UserEvent::GetMeshRotation(index, tx)) {
            log::error!(
                "Failed to send GetMeshRotation event for window {}: {:?}",
                index,
                e
            );
            return Err(format!("Failed to send event: {:?}", e));
        }

        log::info!("Sent GetMeshRotation event for window {}", index);

        match rx.await {
            Ok(result) => Ok(result.into()),
            Err(e) => Err(format!("Failed to receive result: {:?}", e)),
        }
    }

    pub fn set_mesh_rotation(&self, index: usize, rotation: Vec<f32>) {
        if rotation.len() != 16 {
            log::error!(
                "set_mesh_rotation expected 16 floats, got {}",
                rotation.len()
            );
            return;
        }
        let mut arr = [0.0; 16];
        arr.copy_from_slice(&rotation);
        if let Err(e) = self
            .proxy
            .send_event(UserEvent::SetMeshRotation(index, arr))
        {
            log::error!("Failed to send SetMeshRotation event: {:?}", e);
        } else {
            log::info!("Sent SetMeshRotation event for window {}", index);
        }
    }

    pub fn set_oblique_normal(&self, index: usize, normal: Vec<f32>, in_plane_radians: f32) {
        if normal.len() != 3 {
            log::error!(
                "set_mesh_rotation expected 16 floats, got {}",
                normal.len()
            );
            return;
        }
        let mut arr = [0.0; 3];
        arr.copy_from_slice(&normal);
        if let Err(e) = self
            .proxy
            .send_event(UserEvent::SetObliqueNormal(index, arr, in_plane_radians))
        {
            log::error!("Failed to send SetObliqueNormal event: {:?}", e);
        } else {
            log::info!("Sent SetObliqueNormal event for window {}", index);
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct GLCanvas {
    pub(crate) proxy: EventLoopProxy<UserEvent>,
}

impl_user_event_senders_for_glcanvas! {
    set_window_level => SetWindowLevel(window_level: f32),
    set_window_width => SetWindowWidth(window_width: f32),
    set_slice_mm => SetSliceMM(slice: f32),
    set_scale => SetScale(scale: f32),
    set_translate_in_screen_coord => SetTranslateInScreenCoord(x: f32, y: f32, z: f32),
    set_pan => SetPan(dx: f32, dy: f32),
    set_pan_mm => SetPanMM(dx_mm: f32, dy_mm: f32),
    handle_view_click => ViewClick(screen_x: f32, screen_y: f32, screen_z: f32),
    set_oblique_rotation_radians => SetObliqueRotation(horizontal_radians: Option<f32>, vertical_radians: Option<f32>, in_plane_radians: Option<f32>),
    // Mip controls
    set_mip_mode => SetMipMode(mode: u32),
    set_slab_thickness => SetSlabThickness(thickness: f32),
    set_mip_rotation_angle_degrees => SetMipRotationAngleDeg(roll_deg: f32, yaw_deg: f32, pitch_deg: f32),
    // Mesh controls
    set_mesh_rotation_enabled => SetMeshRotationEnabled(enabled: bool),
    set_mesh_opacity => SetMeshOpacity(alpha: f32),
    set_mesh_pan => SetMeshPan(dx: f32, dy: f32),
    reset_mesh => ResetMesh(),
    set_mesh_scale => SetMeshScale(scale: f32),
    set_mesh_rotation_angle_degrees => SetMeshRotationAngleDeg(degrees_x: f32, degrees_y: f32),
    set_mesh_rotation_degrees => SetMeshRotationDegrees(roll_deg: f32, yaw_deg: f32, pitch_deg: f32),
}
