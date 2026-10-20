//! Independent quantitative-dose color wash, hosted in the existing color pass.
use crate::{
    core::{KeplerError, KeplerResult},
    data::{dose::DoseGrid, slice::SlicePlane},
};
use glam::{DVec3, Mat4, Vec4};
use wgpu::util::DeviceExt;
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
struct Uniforms {
    matrix: [f32; 16],
    style: [f32; 4],
    shape: [u32; 4],
}
fn invalid(s: &str) -> KeplerError {
    KeplerError::Graphics(s.into())
}
/// One dose upload per revision. Original samples remain owned by the host's DoseGrid.
pub struct DoseTexture {
    texture: wgpu::Texture,
    offsets: wgpu::Buffer,
}
impl DoseTexture {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, grid: &DoseGrid) -> KeplerResult<Self> {
        let dims = grid.dimensions();
        if dims
            .iter()
            .any(|v| *v > device.limits().max_texture_dimension_3d as usize)
        {
            return Err(invalid("dose dimensions exceed device 3D texture limit"));
        }
        let offsets: Vec<f32> = grid
            .geometry()
            .frame_offsets
            .iter()
            .map(|v| *v as f32)
            .collect();
        if offsets.iter().any(|v| !v.is_finite()) || offsets.windows(2).any(|p| p[0] >= p[1]) {
            return Err(invalid("dose frame positions exceed GPU precision"));
        }
        if offsets.len() * 4 > device.limits().max_storage_buffer_binding_size as usize {
            return Err(invalid("dose frame positions exceed GPU buffer limit"));
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("dose-grid"),
            size: wgpu::Extent3d {
                width: dims[0] as u32,
                height: dims[1] as u32,
                depth_or_array_layers: dims[2] as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let values = grid.display_values();
        if values.iter().any(|v| !v.is_finite()) {
            return Err(invalid("dose exceeds GPU display range"));
        }
        queue.write_texture(
            texture.as_image_copy(),
            bytemuck::cast_slice(&values),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(dims[0] as u32 * 4),
                rows_per_image: Some(dims[1] as u32),
            },
            texture.size(),
        );
        let offsets = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("dose-frame-offsets"),
            contents: bytemuck::cast_slice(&offsets),
            usage: wgpu::BufferUsages::STORAGE,
        });
        Ok(Self { texture, offsets })
    }
}
pub struct DoseRenderContext {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
}
impl DoseRenderContext {
    /// Host wraps constructors in allocation/validation scopes; single sample, no depth.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("dose-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(96),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(8),
                    },
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("dose-color-wash"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/dose.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("dose-layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("dose-color-wash"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
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
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        Self { layout, pipeline }
    }
    pub fn create_view(&self, device: &wgpu::Device, texture: &DoseTexture) -> DoseView {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("dose-view"),
            size: 96,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let view = texture.texture.create_view(&Default::default());
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("dose-view"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: texture.offsets.as_entire_binding(),
                },
            ],
        });
        DoseView {
            uniform,
            bind,
            last: None,
            valid: false,
        }
    }
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, view: &DoseView) {
        if view.valid {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &view.bind, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}
/// Physical slice and independent display settings; no CT window/level enters this pipeline.
#[derive(Clone, Copy)]
pub struct DoseViewport {
    pub plane: SlicePlane,
    pub uv_origin: [f32; 2],
    pub uv_span: [f32; 2],
    pub lower: f32,
    pub upper: f32,
    pub cutoff: f32,
    pub opacity: f32,
}
pub struct DoseView {
    uniform: wgpu::Buffer,
    bind: wgpu::BindGroup,
    last: Option<Uniforms>,
    valid: bool,
}
impl DoseView {
    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
        grid: &DoseGrid,
        v: DoseViewport,
    ) -> KeplerResult<()> {
        self.valid = false;
        v.plane.validate()?;
        if [v.lower, v.upper, v.cutoff, v.opacity]
            .iter()
            .any(|x| !x.is_finite())
            || v.lower < 0.
            || v.upper <= v.lower
            || v.cutoff < 0.
            || !(0.0..=1.0).contains(&v.opacity)
            || v.uv_origin.iter().any(|x| !x.is_finite())
            || v.uv_span.iter().any(|x| !x.is_finite() || *x <= 0.)
        {
            return Err(invalid("invalid dose viewport or style"));
        }
        let p = v.plane;
        let local = |delta: DVec3| {
            let g = grid.geometry();
            DVec3::new(
                delta.dot(g.column) / g.spacing[0],
                delta.dot(g.row) / g.spacing[1],
                delta.dot(g.column.cross(g.row)),
            )
        };
        let x = local(p.right * p.extent_mm[0]);
        let y = local(p.down * p.extent_mm[1]);
        let origin = grid.local_position(p.origin)
            + x * f64::from(v.uv_origin[0])
            + y * f64::from(v.uv_origin[1]);
        let x = (x * f64::from(v.uv_span[0])).as_vec3();
        let y = (y * f64::from(v.uv_span[1])).as_vec3();
        let matrix = Mat4::from_cols(
            x.extend(0.),
            y.extend(0.),
            Vec4::ZERO,
            origin.as_vec3().extend(1.),
        );
        if !matrix.is_finite() {
            return Err(invalid("dose projection exceeds GPU range"));
        }
        let dims = grid.dimensions();
        let u = Uniforms {
            matrix: matrix.to_cols_array(),
            style: [v.lower, v.upper, v.cutoff, v.opacity],
            shape: [dims[0] as u32, dims[1] as u32, dims[2] as u32, 0],
        };
        if self.last != Some(u) {
            queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&u));
            self.last = Some(u);
        }
        self.valid = true;
        Ok(())
    }
}
/// Fixed color ramp shared by legends and native GPU pixel tests. t is clamped to [0,1].
pub fn dose_color(t: f32) -> [f32; 3] {
    let colors = [
        [0.05, 0.1, 0.8],
        [0., 0.8, 0.9],
        [1., 0.9, 0.],
        [0.95, 0.05, 0.],
    ];
    let t = t.clamp(0., 1.) * 3.;
    let i = (t.floor() as usize).min(2);
    let f = t - i as f32;
    std::array::from_fn(|a| colors[i][a] * (1. - f) + colors[i + 1][a] * f)
}
#[cfg(test)]
mod tests {
    #[test]
    fn dose_shader_layout_and_validation() {
        let module =
            naga::front::wgsl::parse_str(include_str!("shaders/dose.wgsl")).expect("dose shader");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .expect("dose WGSL validation");
        assert_eq!(std::mem::size_of::<super::Uniforms>(), 96);
    }
}
