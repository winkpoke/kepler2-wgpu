//! Cached qualitative divergent CT projection, independent of windowing/UI frameworks.
use super::{RenderContent, shaders::volume_shader_source};
use crate::{
    core::{KeplerError, KeplerResult},
    data::bev::{BeamProjection, DetectorRect, VolumeGeometry},
};
use std::sync::Arc;

const UNIFORM_BYTES: u64 = 112;
pub const MAX_BEV_EDGE: u32 = 1024;

/// Explicit physical inputs. The host owns association and image-intensity semantics.
pub struct BevRenderRequest<'a> {
    pub beam: BeamProjection,
    pub volume: &'a VolumeGeometry,
    pub detector: DetectorRect,
    pub size: [u32; 2],
    /// 1..=4096; increasing step length covers the full ray when this limit is reached.
    pub max_steps: u32,
    /// Positive qualitative display coefficient, inverse relative-density mm (not calibrated).
    pub contrast: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BevRenderStats {
    pub integrations: u64,
    pub display_mappings: u64,
}

pub struct BevRenderContext {
    _volume: Arc<RenderContent>,
    integrate: wgpu::RenderPipeline,
    display: wgpu::RenderPipeline,
    buffer: wgpu::Buffer,
    binding: wgpu::BindGroup,
    output_layout: wgpu::BindGroupLayout,
    target: Option<Target>,
    last: Option<Parameters>,
    generation: u64,
    stats: BevRenderStats,
}

struct Target {
    _integral: wgpu::Texture,
    integral_view: wgpu::TextureView,
    color: wgpu::Texture,
    color_view: wgpu::TextureView,
    output: wgpu::BindGroup,
    size: [u32; 2],
}

#[derive(Clone, Copy, PartialEq)]
struct Parameters {
    integration: [[f32; 4]; 6],
    contrast: f32,
}
impl Parameters {
    fn bytes(self) -> Vec<u8> {
        self.integration
            .into_iter()
            .flatten()
            .chain([self.contrast, 0., 0., 0.])
            .flat_map(f32::to_ne_bytes)
            .collect()
    }
}

fn invalid(s: &str) -> KeplerError {
    KeplerError::Graphics(format!("BEV: {s}"))
}
fn texture_entry(
    binding: u32,
    dimension: wgpu::TextureViewDimension,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: dimension,
            multisampled: false,
        },
        count: None,
    }
}

impl BevRenderContext {
    /// Host wraps construction and target allocation in device validation/allocation scopes.
    /// This context belongs to the supplied device and one immutable resident volume.
    pub fn new(device: &wgpu::Device, volume: Arc<RenderContent>) -> KeplerResult<Self> {
        if !matches!(
            volume.texture_format,
            wgpu::TextureFormat::Rg8Unorm | wgpu::TextureFormat::R16Float
        ) {
            return Err(invalid("unsupported CT texture format"));
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bev-input"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(UNIFORM_BYTES),
                    },
                    count: None,
                },
                texture_entry(1, wgpu::TextureViewDimension::D3),
            ],
        });
        let output_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bev-integral-input"),
            entries: &[texture_entry(0, wgpu::TextureViewDimension::D2)],
        });
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bev-parameters"),
            size: UNIFORM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bev-volume"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&volume.view),
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bev-divergent-ct"),
            source: wgpu::ShaderSource::Wgsl(
                volume_shader_source(include_str!("shaders/bev.wgsl")).into(),
            ),
        });
        let integrate = pipeline(
            device,
            &shader,
            &[Some(&layout)],
            "integrate",
            wgpu::TextureFormat::R32Float,
        );
        let display = pipeline(
            device,
            &shader,
            &[Some(&layout), Some(&output_layout)],
            "display",
            wgpu::TextureFormat::Rgba8Unorm,
        );
        Ok(Self {
            _volume: volume,
            integrate,
            display,
            buffer,
            binding,
            output_layout,
            target: None,
            last: None,
            generation: 0,
            stats: Default::default(),
        })
    }

    /// Generation changes only when the bounded output pair is replaced.
    pub fn target_generation(&self) -> u64 {
        self.generation
    }
    pub fn color_view(&self) -> Option<&wgpu::TextureView> {
        self.target.as_ref().map(|t| &t.color_view)
    }
    /// Output access supports bounded export/readback by the host; no CPU volume copy is created.
    pub fn color_texture(&self) -> Option<&wgpu::Texture> {
        self.target.as_ref().map(|t| &t.color)
    }
    pub fn stats(&self) -> BevRenderStats {
        self.stats
    }
    pub fn target_bytes(&self) -> u64 {
        self.target
            .as_ref()
            .map_or(0, |t| u64::from(t.size[0]) * u64::from(t.size[1]) * 8)
    }
    pub fn target_size(&self) -> Option<[u32; 2]> {
        self.target.as_ref().map(|t| t.size)
    }

    /// Allocate bounded outputs before the host registers them for compositing.
    pub fn prepare_target(&mut self, device: &wgpu::Device, size: [u32; 2]) -> KeplerResult<()> {
        if size.contains(&0)
            || size.into_iter().max().unwrap_or(0)
                > MAX_BEV_EDGE.min(device.limits().max_texture_dimension_2d)
        {
            return Err(invalid("invalid bounded target size"));
        }
        if self.target.as_ref().is_none_or(|t| t.size != size) {
            self.target = Some(Target::new(device, size, &self.output_layout));
            self.last = None;
            self.generation = self.generation.wrapping_add(1);
        }
        Ok(())
    }

    /// Encode on the host's existing encoder. Unchanged requests record no GPU work.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        request: &BevRenderRequest<'_>,
    ) -> KeplerResult<()> {
        let [width, height] = request.size;
        if width == 0
            || height == 0
            || width.max(height) > MAX_BEV_EDGE.min(device.limits().max_texture_dimension_2d)
            || !(1..=4096).contains(&request.max_steps)
        {
            return Err(invalid("invalid bounded target size or ray step limit"));
        }
        let params = parameters(request, &self._volume)?;
        self.prepare_target(device, request.size)?;
        if self.last == Some(params) {
            return Ok(());
        }
        let target = self
            .target
            .as_ref()
            .ok_or_else(|| invalid("missing projection target"))?;
        queue.write_buffer(&self.buffer, 0, &params.bytes());
        if self
            .last
            .is_none_or(|last| last.integration != params.integration)
        {
            draw(
                encoder,
                &target.integral_view,
                &self.integrate,
                &self.binding,
                None,
            );
            self.stats.integrations += 1;
        }
        draw(
            encoder,
            &target.color_view,
            &self.display,
            &self.binding,
            Some(&target.output),
        );
        self.stats.display_mappings += 1;
        self.last = Some(params);
        Ok(())
    }
}

fn pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layouts: &[Option<&wgpu::BindGroupLayout>],
    entry: &str,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bev-pipeline-layout"),
        bind_group_layouts: layouts,
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(entry),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vertex"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}

impl Target {
    fn new(device: &wgpu::Device, size: [u32; 2], layout: &wgpu::BindGroupLayout) -> Self {
        let make = |label, format| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            })
        };
        let integral = make("bev-relative-density-mm", wgpu::TextureFormat::R32Float);
        let color = make("bev-qualitative-drr", wgpu::TextureFormat::Rgba8Unorm);
        let integral_view = integral.create_view(&Default::default());
        let color_view = color.create_view(&Default::default());
        let output = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bev-integral"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&integral_view),
            }],
        });
        Self {
            _integral: integral,
            integral_view,
            color,
            color_view,
            output,
            size,
        }
    }
}

fn parameters(request: &BevRenderRequest<'_>, volume: &RenderContent) -> KeplerResult<Parameters> {
    let beam = request.beam;
    beam.validate()?;
    let detector = DetectorRect::new(request.detector.top_left, request.detector.extent)?;
    if !request.contrast.is_finite() || request.contrast <= 0. || request.contrast > 0.2 {
        return Err(invalid("contrast must be finite in (0,0.2]"));
    }
    let mapping = request.volume.patient_to_texture();
    let corner = detector.top_left;
    let extent = detector.extent;
    let origin = mapping.transform_point3(beam.isocenter + beam.x * corner.x + beam.y * corner.y);
    let source = mapping.transform_point3(beam.isocenter + beam.z * beam.sad);
    let right = mapping.transform_vector3(beam.x * extent.x);
    let down = mapping.transform_vector3(-beam.y * extent.y);
    let vector = |v: glam::DVec3, w: f32| [v.x as f32, v.y as f32, v.z as f32, w];
    let decode = volume.decode_parameters();
    let integration = [
        vector(source, beam.sad as f32),
        vector(
            origin,
            (request.volume.minimum_spacing_mm() * 0.5).max(0.01) as f32,
        ),
        vector(right, decode.is_packed_flag as f32),
        vector(down, decode.bias),
        [
            corner.x as f32,
            corner.y as f32,
            extent.x as f32,
            -extent.y as f32,
        ],
        [request.max_steps as f32, 0., 0., 0.],
    ];
    if !integration.iter().flatten().all(|v| v.is_finite()) {
        return Err(invalid("projection exceeds GPU coordinate range"));
    }
    Ok(Parameters {
        integration,
        contrast: request.contrast,
    })
}

fn draw(
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    input: &wgpu::BindGroup,
    output: Option<&wgpu::BindGroup>,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("bev-cached-projection"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, input, &[]);
    if let Some(output) = output {
        pass.set_bind_group(1, output, &[]);
    }
    pass.draw(0..3, 0..1);
}
