//! Batched geometric beam overlays on a host-owned color pass. One buffer owner per pane.
use crate::{
    core::{KeplerError, KeplerResult},
    data::{
        beam::{BeamOverlayGeometry, slice_segments},
        slice::SlicePlane,
    },
};
use std::sync::Arc;

const MAX_BEAMS: usize = 128;
const MAX_SEGMENTS: usize = (2048 + 512) * MAX_BEAMS;

#[derive(Clone)]
pub struct BeamInstance {
    pub geometry: BeamOverlayGeometry,
    pub selected: bool,
    pub arc_directions: Option<Arc<[glam::DVec3]>>,
}

#[derive(Clone, Copy)]
pub struct BeamViewport {
    pub plane: SlicePlane,
    pub uv_origin: [f32; 2],
    pub uv_span: [f32; 2],
    pub pixels: [f32; 2],
    pub fields: bool,
    pub axes: bool,
    pub off_slice: bool,
}

pub struct BeamOverlayContext {
    pipeline: wgpu::RenderPipeline,
    aperture_pipeline: wgpu::RenderPipeline,
    views: [View; 3],
}
struct View {
    buffer: wgpu::Buffer,
    binding: wgpu::BindGroup,
    last: Vec<Draw>,
    stride: u32,
    segment_capacity: usize,
    ranges: Vec<std::ops::Range<u32>>,
    segments: wgpu::Buffer,
}
#[derive(Clone, PartialEq)]
struct Draw {
    values: [f32; 40],
    segments: Vec<[f32; 4]>,
}
fn invalid(message: &str) -> KeplerError {
    KeplerError::Graphics(message.into())
}

impl BeamOverlayContext {
    /// Replace one complete pane batch. Rejected input clears its previous draw ranges.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        index: usize,
        beams: &[BeamInstance],
        viewport: BeamViewport,
    ) -> KeplerResult<()> {
        if index >= 3 {
            return Err(invalid("Invalid beam pane index"));
        }
        let result = self.update(device, queue, index, beams, viewport);
        if result.is_err() {
            self.views[index].last.clear();
            self.views[index].ranges.clear();
        }
        result
    }
    fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        index: usize,
        beams: &[BeamInstance],
        viewport: BeamViewport,
    ) -> KeplerResult<()> {
        viewport.plane.validate()?;
        if beams.len() > MAX_BEAMS
            || !viewport
                .uv_origin
                .iter()
                .chain(&viewport.uv_span)
                .chain(&viewport.pixels)
                .all(|v| v.is_finite())
            || viewport
                .uv_span
                .iter()
                .chain(&viewport.pixels)
                .any(|v| *v <= 0.)
        {
            return Err(invalid("Invalid beam batch or viewport"));
        }
        let draws = beams
            .iter()
            .map(|beam| parameters(beam, viewport))
            .collect::<KeplerResult<Vec<_>>>()?;
        let view = &mut self.views[index];
        if view.last != draws {
            let count: usize = draws.iter().map(|d| d.segments.len()).sum();
            if count > MAX_SEGMENTS {
                view.last.clear();
                view.ranges.clear();
                return Err(invalid("Beam batch segment limit exceeded"));
            }
            if count > view.segment_capacity {
                view.segment_capacity = count.next_power_of_two().min(MAX_SEGMENTS);
                view.segments = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("beam-batch-segments"),
                    size: (view.segment_capacity * 16) as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            }
            view.ranges.clear();
            let mut segments = Vec::with_capacity(count * 16);
            let mut start = 0;
            for (index, draw) in draws.iter().enumerate() {
                let bytes: Vec<u8> = draw.values.iter().flat_map(|v| v.to_le_bytes()).collect();
                queue.write_buffer(&view.buffer, index as u64 * u64::from(view.stride), &bytes);
                segments.extend(draw.segments.iter().flatten().flat_map(|v| v.to_le_bytes()));
                #[allow(clippy::cast_possible_truncation)]
                let end = start + draw.segments.len() as u32;
                view.ranges.push(start..end);
                start = end;
            }
            if !segments.is_empty() {
                queue.write_buffer(&view.segments, 0, &segments);
            }
            view.last = draws;
        }
        Ok(())
    }
    pub fn render(&self, pass: &mut wgpu::RenderPass<'_>, index: usize) {
        if index >= 3 {
            return;
        }
        let view = &self.views[index];
        for (index, range) in view.ranges.iter().enumerate().take(view.last.len()) {
            #[allow(clippy::cast_possible_truncation)]
            let offset = index as u32 * view.stride;
            pass.set_bind_group(0, &view.binding, &[offset]);
            pass.set_pipeline(&self.pipeline);
            pass.draw(0..3, 0..1);
            if !range.is_empty() {
                pass.set_pipeline(&self.aperture_pipeline);
                pass.set_vertex_buffer(0, view.segments.slice(..));
                pass.draw(0..6, range.clone());
            }
        }
    }
}

fn parameters(instance: &BeamInstance, viewport: BeamViewport) -> KeplerResult<Draw> {
    let beam = &instance.geometry;
    beam.validate()?;
    let plane = viewport.plane;
    let mut values = [0.; 40];
    for (i, vector) in [
        plane.origin - beam.isocenter,
        plane.right * plane.extent_mm[0],
        plane.down * plane.extent_mm[1],
        beam.x,
        beam.y,
        beam.z,
    ]
    .into_iter()
    .enumerate()
    {
        values[i * 4..i * 4 + 3].copy_from_slice(&vector.to_array().map(|v| v as f32));
    }
    values[23] = beam.sad as f32;
    values[24..28].copy_from_slice(&beam.jaws.unwrap_or([0.; 4]).map(|v| v as f32));
    values[28..32].copy_from_slice(&[
        viewport.uv_origin[0],
        viewport.uv_origin[1],
        viewport.uv_span[0],
        viewport.uv_span[1],
    ]);
    values[32..36].copy_from_slice(&[
        viewport.pixels[0],
        viewport.pixels[1],
        f32::from(viewport.fields),
        f32::from(viewport.axes),
    ]);
    values[36..40].copy_from_slice(&[
        f32::from(viewport.off_slice),
        if instance.selected { 0.95 } else { 0.5 },
        f32::from(beam.jaws.is_some() && beam.aperture.is_none()),
        f32::from(instance.selected),
    ]);
    if values.iter().any(|v| !v.is_finite()) {
        return Err(invalid("Beam exceeds GPU numeric range"));
    }
    let mut segments: Vec<[f32; 4]> = if viewport.fields {
        slice_segments(beam, &plane)
            .into_iter()
            .map(|line| {
                [
                    line[0].x as f32,
                    line[0].y as f32,
                    line[1].x as f32,
                    line[1].y as f32,
                ]
            })
            .collect()
    } else {
        Vec::new()
    };
    if let Some(directions) = &instance.arc_directions {
        if directions.len() > 512
            || directions
                .iter()
                .any(|v| !v.is_finite() || (v.length_squared() - 1.).abs() > 1e-6)
        {
            return Err(invalid("Invalid arc orientation guide"));
        }
        // A schematic angular guide at a bounded display radius. Its orthogonal
        // projection is distinct from the true central-axis/field intersection.
        let radius = plane.extent_mm[0].min(plane.extent_mm[1]) * 0.32;
        let uv = |v: glam::DVec3| {
            let p = beam.isocenter + v * radius - plane.origin;
            glam::DVec2::new(
                p.dot(plane.right) / plane.extent_mm[0],
                p.dot(plane.down) / plane.extent_mm[1],
            )
        };
        for pair in directions.windows(2) {
            let a = uv(pair[0]);
            let b = uv(pair[1]);
            if a.distance_squared(b) > 1e-16 {
                segments.push([a.x as f32, a.y as f32, b.x as f32, b.y as f32]);
            }
        }
    }
    if segments.iter().flatten().any(|v| !v.is_finite()) {
        return Err(invalid("Beam lines exceed GPU numeric range"));
    }
    Ok(Draw { values, segments })
}
impl BeamOverlayContext {
    #[allow(clippy::too_many_lines)] // Construct both compatible pipelines and their per-pane resources together.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("beam-overlay-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(160),
                },
                count: None,
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("beam-overlay"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/beam.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("beam-overlay-pipeline-layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mpr-beam-overlay"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let aperture_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mpr-mlc-aperture"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("aperture_vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 16,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 0,
                        shader_location: 0,
                    }],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("aperture_fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let stride = device.limits().min_uniform_buffer_offset_alignment.max(256);
        let views = std::array::from_fn(|_| {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("beam-pane-uniforms"),
                size: u64::from(stride) * MAX_BEAMS as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("beam-pane-binding"),
                layout: &layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(160),
                    }),
                }],
            });
            View {
                buffer,
                binding,
                last: Vec::new(),
                stride,
                segment_capacity: 1,
                ranges: Vec::new(),
                segments: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("mlc-pane-segments"),
                    size: 16,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
            }
        });
        Self {
            pipeline,
            aperture_pipeline,
            views,
        }
    }
}
