//! Prepare the release viewer layout completely before publishing it.
use super::{app_model::AppModel, appview::AppView};
use crate::core::error::{KeplerError, KeplerResult};
use crate::data::ct_volume::CTVolume;
use crate::rendering::view::{
    DefaultViewFactory, GridLayout, LargeLeft3RightLayout, LayoutStrategy, OneCellLayout,
    Orientation, RenderContent, View, ViewFactory, ALL_ORIENTATIONS,
};
use std::sync::Arc;

pub(super) struct TransitionRequest {
    pub mode: usize,
    pub mesh_index: Option<usize>,
    pub orientation_slot: Option<usize>,
    pub mip_index: Option<usize>,
    pub orientation_index: usize,
}
pub(super) enum ViewInput<'a> {
    Mpr(&'a CTVolume, Orientation),
    Mip,
    Mesh,
}
pub(super) trait Preparation {
    fn upload(&self, model: &AppModel) -> KeplerResult<Arc<RenderContent>>;
    fn create_view(
        &self,
        factory: &DefaultViewFactory,
        content: Arc<RenderContent>,
        input: ViewInput<'_>,
    ) -> KeplerResult<Box<dyn View>> {
        build_view(factory, content, input)
    }
}
pub(super) fn build_view(
    factory: &DefaultViewFactory,
    content: Arc<RenderContent>,
    input: ViewInput<'_>,
) -> KeplerResult<Box<dyn View>> {
    let result = match input {
        ViewInput::Mpr(vol, orientation) => {
            factory.create_mpr_view_with_content(content, vol, orientation, (0, 0), (0, 0))
        }
        ViewInput::Mip => factory.create_mip_view_with_content(content, (0, 0), (0, 0)),
        ViewInput::Mesh => factory.create_mesh_view_with_content(content, (0, 0), (0, 0)),
    };
    result.map_err(|error| KeplerError::Graphics(error.to_string()))
}
pub(super) struct GpuPreparation<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
}
impl Preparation for GpuPreparation<'_> {
    fn upload(&self, model: &AppModel) -> KeplerResult<Arc<RenderContent>> {
        let vol = model.volume()?;
        let (bytes, encoding) = model.get_volume_render_data()?;
        Ok(Arc::new(RenderContent::from_bytes(
            self.device,
            self.queue,
            &bytes,
            "CT Volume",
            vol.dimensions.0 as u32,
            vol.dimensions.1 as u32,
            vol.dimensions.2 as u32,
            encoding,
        )?))
    }
}
fn validate_slot(index: usize, selection: &str) -> KeplerResult<()> {
    if index >= ALL_ORIENTATIONS.len() {
        return Err(KeplerError::Validation(format!(
            "view-layout transition: {selection} index {index} is outside 0..=3"
        )));
    }
    Ok(())
}
pub(super) fn prepare(
    vol: &CTVolume,
    orientations: [usize; 4],
    request: &TransitionRequest,
    mut build: impl FnMut(ViewInput<'_>) -> KeplerResult<Box<dyn View>>,
) -> KeplerResult<(Vec<Box<dyn View>>, Box<dyn LayoutStrategy>)> {
    let mut views = Vec::with_capacity(if request.mode <= 2 { 1 } else { 4 });
    let strategy: Box<dyn LayoutStrategy> = if request.mode <= 2 {
        Box::new(OneCellLayout {
            rows: 1,
            cols: 1,
            spacing: 0,
        })
    } else if request.mode == 4 {
        Box::new(LargeLeft3RightLayout {
            rows: 1,
            cols: 3,
            spacing: 0,
        })
    } else {
        Box::new(GridLayout {
            rows: 2,
            cols: 2,
            spacing: 2,
        })
    };
    match request.mode {
        0 => {
            validate_slot(request.orientation_index, "MPR orientation")?;
            views.push(build(ViewInput::Mpr(
                vol,
                ALL_ORIENTATIONS[request.orientation_index],
            ))?);
        }
        1 => views.push(build(ViewInput::Mip)?),
        2 => views.push(build(ViewInput::Mesh)?),
        _ => {
            for &orientation in &orientations {
                validate_slot(orientation, "MPR orientation")?;
            }
            for (name, index) in [
                ("MIP slot", request.mip_index),
                ("mesh slot", request.mesh_index),
            ] {
                if let Some(index) = index {
                    validate_slot(index, name)?;
                }
            }
            for orientation in orientations {
                views.push(build(ViewInput::Mpr(vol, ALL_ORIENTATIONS[orientation]))?);
            }
            if let Some(index) = request.mip_index {
                views[index] = build(ViewInput::Mip)?;
            }
            if let Some(index) = request.mesh_index {
                views[index] = build(ViewInput::Mesh)?;
            }
        }
    }
    Ok((views, strategy))
}
pub(super) fn apply(
    model: &AppModel,
    app_view: &mut AppView,
    orientations: &mut [usize; 4],
    request: TransitionRequest,
    preparation: &impl Preparation,
) -> KeplerResult<()> {
    let vol = model.volume()?;
    let mut next = *orientations;
    if let Some(slot) = request.orientation_slot {
        validate_slot(slot, "orientation slot")?;
        validate_slot(request.orientation_index, "selected orientation")?;
        next[slot] = request.orientation_index;
    }
    let content = preparation.upload(model)?;
    let (views, strategy) = prepare(vol, next, &request, |input| {
        preparation.create_view(&app_view.view_factory, content.clone(), input)
    })?;
    // No returned failure can occur after this point.
    app_view.save_view_states();
    app_view.layout.replace_all_views(views, Some(strategy));
    *orientations = next;
    app_view.restore_view_states();
    let thickness = vol.voxel_spacing.2 / vol.voxel_spacing.0;
    for index in [request.mesh_index, request.mip_index]
        .into_iter()
        .flatten()
    {
        let _ = app_view.set_slab_thickness(index, thickness);
    }
    Ok(())
}
