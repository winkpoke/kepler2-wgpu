//! Reusable source-plane contour outlines in a host-owned single-sample render pass.
//!
//! No egui, window, surface, volume sampling, depth attachment or filesystem ownership.

use crate::{
    core::{KeplerError, KeplerResult},
    data::{dicom::MAX_RTSTRUCT_POINTS, structures::*},
};
use glam::{DVec3, Mat4, Vec4};
use std::{ops::Range, sync::Arc};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Segment {
    start: [f32; 3],
    end: [f32; 3],
    color: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    matrix: [f32; 16],
    style: [f32; 4],
}

struct Draw {
    roi: usize,
    contour: usize,
    segments: Range<u32>,
}

/// Immutable geometry shared across all panes. Points are rebased before conversion to f32.
pub struct ContourBatch {
    structures: Arc<StructureSet>,
    origin: DVec3,
    segments: wgpu::Buffer,
    draws: Vec<Draw>,
}

impl ContourBatch {
    /// Upload once per structure revision, bounded to the RTSTRUCT point budget.
    pub fn new(device: &wgpu::Device, structures: Arc<StructureSet>) -> KeplerResult<Self> {
        let origin = structures
            .rois
            .iter()
            .flat_map(|r| &r.contours)
            .flat_map(|c| c.points())
            .next()
            .copied()
            .unwrap_or(DVec3::ZERO);
        let mut segments = Vec::new();
        let mut draws = Vec::new();
        let mut count = 0usize;
        for (ri, roi) in structures.rois.iter().enumerate() {
            for (ci, contour) in roi.contours.iter().enumerate() {
                count = count.checked_add(contour.points().len()).ok_or_else(|| {
                    KeplerError::Validation("contour point budget overflow".into())
                })?;
                if count > MAX_RTSTRUCT_POINTS {
                    return Err(KeplerError::Validation(
                        "contour GPU point budget exceeded".into(),
                    ));
                }
                if contour.kind() == ContourKind::OpenNonplanar {
                    continue;
                }
                let start = segments.len() as u32;
                let color = [
                    f32::from(roi.color[0]) / 255.,
                    f32::from(roi.color[1]) / 255.,
                    f32::from(roi.color[2]) / 255.,
                    1.,
                ];
                let mut append = |a: DVec3, b: DVec3| -> KeplerResult<()> {
                    let a = (a - origin).as_vec3();
                    let b = (b - origin).as_vec3();
                    if !a.is_finite() || !b.is_finite() {
                        return Err(KeplerError::Validation(
                            "contour exceeds GPU coordinate range".into(),
                        ));
                    }
                    segments.push(Segment {
                        start: a.to_array(),
                        end: b.to_array(),
                        color,
                    });
                    Ok(())
                };
                for pair in contour.points().windows(2) {
                    append(pair[0], pair[1])?;
                }
                if contour.kind().is_closed() {
                    append(
                        contour.points()[contour.points().len() - 1],
                        contour.points()[0],
                    )?;
                }
                if contour.kind() == ContourKind::Point {
                    append(contour.points()[0], contour.points()[0])?;
                }
                draws.push(Draw {
                    roi: ri,
                    contour: ci,
                    segments: start..segments.len() as u32,
                });
            }
        }
        // An empty buffer still has a valid minimum allocation; no draw references it.
        let data = if segments.is_empty() {
            vec![Segment {
                start: [0.; 3],
                end: [0.; 3],
                color: [0.; 4],
            }]
        } else {
            segments
        };
        let bytes = bytemuck::cast_slice(&data);
        if bytes.len() as u64 > device.limits().max_buffer_size {
            return Err(KeplerError::Graphics(
                "contour buffer exceeds device limit".into(),
            ));
        }
        let segments = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("contour-segments"),
            contents: bytes,
            usage: wgpu::BufferUsages::VERTEX,
        });
        Ok(Self {
            structures,
            origin,
            segments,
            draws,
        })
    }
}

/// One cached pipeline per device/attachment format. All panes share it.
pub struct ContourRenderContext {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

impl ContourRenderContext {
    /// Host must use a single-sample color attachment of `format` without a depth attachment.
    /// Creation must be wrapped in the host's validation/allocation error scopes.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("contour-view-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(80),
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("contour-pipeline-layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("contour-outlines"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/contour.wgsl").into()),
        });
        let attributes = wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3,2=>Float32x4];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("contour-overlay"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Segment>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &attributes,
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        Self { pipeline, layout }
    }

    pub fn create_view(&self, device: &wgpu::Device) -> ContourView {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("contour-view"),
            size: 80,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("contour-view"),
            layout: &self.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        ContourView {
            uniform,
            bind,
            ranges: Vec::new(),
            last: None,
        }
    }

    /// Draw after the grayscale slice, before UI handles. Host sets viewport/scissor.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, batch: &ContourBatch, view: &ContourView) {
        if view.ranges.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &view.bind, &[]);
        pass.set_vertex_buffer(0, batch.segments.slice(..));
        for range in &view.ranges {
            pass.draw(0..4, range.clone());
        }
    }
}

/// View state is per pane: queue writes for one pane never overwrite another pane's uniforms.
pub struct ContourView {
    uniform: wgpu::Buffer,
    bind: wgpu::BindGroup,
    ranges: Vec<Range<u32>>,
    last: Option<Uniforms>,
}

/// Mapping from the full physical image to the host's clipped viewport.
#[derive(Clone, Copy, Debug)]
pub struct ContourViewport {
    pub plane: StructurePlane,
    pub uv_origin: [f32; 2],
    pub uv_span: [f32; 2],
    pub pixels: [f32; 2],
    pub width_pixels: f32,
    pub opacity: f32,
}

impl ContourView {
    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
        batch: &ContourBatch,
        visible: &[bool],
        viewport: ContourViewport,
    ) -> KeplerResult<usize> {
        self.ranges.clear();
        viewport.plane.validate()?;
        if visible.len() != batch.structures.rois.len()
            || viewport
                .uv_span
                .iter()
                .chain(&viewport.pixels)
                .any(|v| !v.is_finite() || *v <= 0.)
            || viewport.uv_origin.iter().any(|v| !v.is_finite())
            || !viewport.width_pixels.is_finite()
            || viewport.width_pixels <= 0.
            || viewport.width_pixels > 64.
            || !viewport.opacity.is_finite()
            || !(0.0..=1.0).contains(&viewport.opacity)
        {
            return Err(KeplerError::Validation(
                "invalid contour viewport/style".into(),
            ));
        }
        let plane = viewport.plane;
        // Rebase in f64 to avoid cancellation at large patient origins.
        let x = plane.right * (2. / (plane.extent_mm[0] * f64::from(viewport.uv_span[0])));
        let y = plane.down * (-2. / (plane.extent_mm[1] * f64::from(viewport.uv_span[1])));
        let delta = batch.origin - plane.origin;
        let translation = Vec4::new(
            (delta.dot(x) - 1. - 2. * f64::from(viewport.uv_origin[0] / viewport.uv_span[0]))
                as f32,
            (delta.dot(y) + 1. + 2. * f64::from(viewport.uv_origin[1] / viewport.uv_span[1]))
                as f32,
            0.,
            1.,
        );
        let x = x.as_vec3();
        let y = y.as_vec3();
        let matrix = Mat4::from_cols(
            Vec4::new(x.x, y.x, 0., 0.),
            Vec4::new(x.y, y.y, 0., 0.),
            Vec4::new(x.z, y.z, 0., 0.),
            translation,
        );
        if !matrix.is_finite() {
            return Err(KeplerError::Validation("invalid contour projection".into()));
        }
        let uniforms = Uniforms {
            matrix: matrix.to_cols_array(),
            style: [
                viewport.pixels[0],
                viewport.pixels[1],
                viewport.width_pixels * 0.5,
                viewport.opacity,
            ],
        };
        if self
            .last
            .as_ref()
            .is_none_or(|old| bytemuck::bytes_of(old) != bytemuck::bytes_of(&uniforms))
        {
            queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&uniforms));
            self.last = Some(uniforms);
        }
        for draw in &batch.draws {
            if visible[draw.roi]
                && batch.structures.rois[draw.roi].contours[draw.contour].lies_on(&plane)
            {
                self.ranges.push(draw.segments.clone());
            }
        }
        Ok(self.ranges.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn outline_shader_validates() {
        let module = naga::front::wgsl::parse_str(include_str!("shaders/contour.wgsl"))
            .expect("valid contour WGSL");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .expect("valid contour shader");
    }
    #[test]
    fn gpu_layout_matches_wgsl() {
        assert_eq!(std::mem::size_of::<Uniforms>(), 80);
        assert_eq!(std::mem::size_of::<Segment>(), 40);
        assert_eq!(std::mem::offset_of!(Uniforms, style), 64);
    }
}
