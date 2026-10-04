//! Complete synchronous view-layout operation. State remains with AppModel/AppView/App.
//! Returned preparation failures are distinct from asynchronous fatal GPU faults.

use super::{app_model::AppModel, appview::AppView};
use crate::core::error::{KeplerError, KeplerResult};
use crate::data::ct_volume::CTVolume;
use crate::rendering::view::mesh::mesh::{Mesh, MeshVertex};
use crate::rendering::view::{
    DefaultViewFactory, GridLayout, LayoutStrategy, OneCellLayout, Orientation, RenderContent,
    View, ViewFactory, ALL_ORIENTATIONS,
};
use std::sync::Arc;

pub(super) struct TransitionRequest {
    pub mode: usize,
    pub reuse_mesh: bool,
    pub world_min: Option<[f32; 3]>,
    pub world_max: Option<[f32; 3]>,
    pub mesh_index: Option<usize>,
    pub orientation_slot: Option<usize>,
    pub iso_range: (f32, f32),
    pub mip_index: Option<usize>,
    pub orientation_index: usize,
}

pub(super) enum ViewInput<'a> {
    Mpr(&'a CTVolume, Orientation),
    Mip,
    Mesh(&'a Mesh),
}

impl ViewInput<'_> {
    fn name(&self) -> &'static str {
        match self {
            Self::Mpr(..) => "MPR",
            Self::Mip => "MIP",
            Self::Mesh(..) => "mesh",
        }
    }
}

/// Internal borrowed preparation seam; both production and failure tests use `apply`.
pub(super) trait Preparation {
    fn upload(&self, model: &AppModel) -> KeplerResult<Arc<RenderContent>>;

    fn create_view(
        &self,
        factory: &DefaultViewFactory,
        content: Arc<RenderContent>,
        input: ViewInput<'_>,
    ) -> KeplerResult<Box<dyn View>> {
        let result = match input {
            ViewInput::Mpr(vol, orientation) => {
                factory.create_mpr_view_with_content(content, vol, orientation, (0, 0), (0, 0))
            }
            ViewInput::Mip => factory.create_mip_view_with_content(content, (0, 0), (0, 0)),
            ViewInput::Mesh(mesh) => {
                factory.create_mesh_view_with_content(content, mesh, (0, 0), (0, 0))
            }
        };
        result.map_err(|error| KeplerError::Graphics(error.to_string()))
    }
}

pub(super) struct GpuPreparation<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
}

impl Preparation for GpuPreparation<'_> {
    fn upload(&self, model: &AppModel) -> KeplerResult<Arc<RenderContent>> {
        let vol = model.volume()?;
        let (bytes, encoding) = model.get_volume_render_data()?;
        log::info!("Using {:?} volume texture path", encoding);
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

fn generate_mesh(vol: &CTVolume, request: &TransitionRequest) -> Mesh {
    let mut mesh = Mesh::new(
        vol,
        request.iso_range.0,
        request.iso_range.1,
        request.world_min,
        request.world_max,
    );
    // Retain the existing empty-extraction fallback and cache-reuse policy.
    if mesh.vertices.is_empty() {
        log::warn!(
            "Generated mesh is empty (ISO: {}-{}). Injecting dummy triangle.",
            request.iso_range.0,
            request.iso_range.1
        );
        let dummy = MeshVertex {
            position: [0.0; 3],
            normal: [0.0, 0.0, 1.0],
            color: [0.0; 3],
        };
        mesh.vertices.extend([dummy; 3]);
        mesh.indices.extend([0, 1, 2]);
    }
    mesh
}

pub(super) fn apply(
    model: &mut AppModel,
    app_view: &mut AppView,
    cached_mesh: &mut Option<Mesh>,
    orientations: &mut [usize; 4],
    request: TransitionRequest,
    preparation: &impl Preparation,
) -> KeplerResult<()> {
    let vol = model.volume().map_err(|_| {
        KeplerError::Validation("view-layout transition requires a loaded volume".into())
    })?;
    let mut next_orientations = *orientations;
    if let Some(slot) = request.orientation_slot {
        validate_slot(slot, "orientation slot")?;
        validate_slot(request.orientation_index, "selected orientation")?;
        next_orientations[slot] = request.orientation_index;
    }
    match request.mode {
        0 => validate_slot(request.orientation_index, "MPR orientation")?,
        1 | 2 => {}
        _ => {
            for &orientation in &next_orientations {
                validate_slot(orientation, "MPR orientation")?;
            }
            for (selection, index) in [
                ("MIP slot", request.mip_index),
                ("mesh slot", request.mesh_index),
            ] {
                if let Some(index) = index {
                    validate_slot(index, selection)?;
                }
            }
        }
    }
    if request.mode == 2 && request.mesh_index.is_none() && cached_mesh.is_none() {
        return Err(KeplerError::Validation(
            "view-layout transition: single mesh requires cached mesh or requested generation"
                .into(),
        ));
    }

    let strategy: Option<Box<dyn LayoutStrategy>> = if request.mode <= 2 {
        Some(Box::new(OneCellLayout {
            rows: 1,
            cols: 1,
            spacing: 0,
        }))
    } else if app_view.is_one_cell_layout() {
        Some(Box::new(GridLayout {
            rows: 2,
            cols: 2,
            spacing: 2,
        }))
    } else {
        None
    };
    let content = preparation.upload(model).map_err(|error| {
        KeplerError::Graphics(format!(
            "view-layout transition upload preparation for dimensions {:?}: {error}",
            vol.dimensions
        ))
    })?;
    let candidate_mesh = (request.mesh_index.is_some()
        && (!request.reuse_mesh || cached_mesh.is_none()))
    .then(|| generate_mesh(vol, &request));
    let mesh = candidate_mesh.as_ref().or(cached_mesh.as_ref());
    let build = |input: ViewInput<'_>| {
        let kind = input.name();
        preparation
            .create_view(&app_view.view_factory, content.clone(), input)
            .map_err(|error| {
                KeplerError::Graphics(format!(
                    "view-layout transition create {kind} view: {error}"
                ))
            })
    };

    let mut views = Vec::with_capacity(if request.mode <= 2 { 1 } else { 4 });
    match request.mode {
        0 => views.push(build(ViewInput::Mpr(
            vol,
            ALL_ORIENTATIONS[request.orientation_index],
        ))?),
        1 => views.push(build(ViewInput::Mip)?),
        2 => views.push(build(ViewInput::Mesh(mesh.ok_or_else(|| {
            KeplerError::Validation("view-layout transition requires mesh content".into())
        })?))?),
        _ => {
            for orientation in next_orientations {
                views.push(build(ViewInput::Mpr(vol, ALL_ORIENTATIONS[orientation]))?);
            }
            if let Some(index) = request.mip_index {
                views[index] = build(ViewInput::Mip)?;
            }
            if let Some(index) = request.mesh_index {
                views[index] = build(ViewInput::Mesh(mesh.ok_or_else(|| {
                    KeplerError::Validation("view-layout transition requires mesh content".into())
                })?))?;
            }
        }
    }

    // All returned-error paths are above this point. Placement touches incoming views only.
    app_view.layout.replace_all_views(views, strategy);
    *orientations = next_orientations;
    if let Some(mesh) = candidate_mesh {
        *cached_mesh = Some(mesh);
    }
    if request.mesh_index.is_some() {
        model.enable_mesh = true;
    }
    Ok(())
}
