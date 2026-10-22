//! Physical BEV aperture/leaf/contour drawing on a host-owned render pass.
use crate::{
    core::{KeplerError, KeplerResult},
    data::bev::{DetectorRect, ProjectedRoi, leaf_rectangles},
};
use bytemuck::{Pod, Zeroable};
use glam::DVec2;
use std::sync::Arc;

const MAX_ITEMS: usize = 106_000;
pub const BEV_OVERLAY_BYTES: u64 = MAX_ITEMS as u64 * 48;
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Item {
    edge: [f32; 4],
    color: [f32; 4],
    style: [f32; 4],
}

#[derive(Default)]
pub struct BevOverlayScene {
    items: Vec<Item>,
}
impl BevOverlayScene {
    fn item(&mut self, a: DVec2, b: DVec2, color: [f32; 4], style: [f32; 4]) -> KeplerResult<()> {
        let edge = [a.x as f32, a.y as f32, b.x as f32, b.y as f32];
        if self.items.len() >= MAX_ITEMS || !edge.iter().all(|v| v.is_finite()) {
            return Err(KeplerError::Graphics(
                "BEV overlay limit or numeric range exceeded".into(),
            ));
        }
        self.items.push(Item { edge, color, style });
        Ok(())
    }
    /// Effective geometric opening. A missing MLC aperture can fall back to the jaw envelope.
    pub fn aperture(&mut self, edges: impl IntoIterator<Item = [DVec2; 2]>) -> KeplerResult<()> {
        for [a, b] in edges {
            self.item(a, b, [0.36, 0.87, 0.92, 1.], [1.8, 0., 0., 0.])?;
        }
        Ok(())
    }
    pub fn jaws(&mut self, jaws: [f64; 4], effective: bool) -> KeplerResult<()> {
        let [x1, x2, y1, y2] = jaws;
        let points = [
            DVec2::new(x1, y1),
            DVec2::new(x2, y1),
            DVec2::new(x2, y2),
            DVec2::new(x1, y2),
            DVec2::new(x1, y1),
        ];
        for edge in points.windows(2) {
            self.item(
                edge[0],
                edge[1],
                if effective {
                    [0.36, 0.87, 0.92, 1.]
                } else {
                    [0.90, 0.74, 0.44, 1.]
                },
                [
                    if effective { 1.8 } else { 1. },
                    0.,
                    if effective { 0. } else { 8. },
                    0.,
                ],
            )?;
        }
        Ok(())
    }
    /// Ideal, translucent leaf bodies; does not alter the effective aperture.
    pub fn mlc_layer(
        &mut self,
        boundaries: &[f64],
        positions: &[f64],
        proximal: bool,
    ) -> KeplerResult<()> {
        for [a, b] in leaf_rectangles(boundaries, positions, [-10_000., 10_000.])? {
            self.item(
                a,
                b,
                if proximal {
                    [0.80, 0.85, 0.92, 0.10]
                } else {
                    [0.56, 0.69, 0.80, 0.12]
                },
                [0., 1., 0., 0.],
            )?;
            let corners = [a, DVec2::new(b.x, a.y), b, DVec2::new(a.x, b.y), a];
            for edge in corners.windows(2) {
                self.item(
                    edge[0],
                    edge[1],
                    [0.68, 0.77, 0.88, 0.35],
                    [0.6, 0., if proximal { 6. } else { 0. }, 0.],
                )?;
            }
        }
        Ok(())
    }
    pub fn roi(&mut self, roi: &ProjectedRoi, color: [u8; 3], selected: bool) -> KeplerResult<()> {
        let rgba = [
            f32::from(color[0]) / 255.,
            f32::from(color[1]) / 255.,
            f32::from(color[2]) / 255.,
            if selected { 1. } else { 0.6 },
        ];
        for &[a, b] in &roi.segments {
            self.item(a, b, rgba, [if selected { 1.7 } else { 0.8 }, 0., 0., 0.])?;
        }
        Ok(())
    }
}

/// One small instance buffer/context per host device and explicit attachment format.
pub struct BevOverlayContext {
    pipeline: wgpu::RenderPipeline,
    binding: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    items: wgpu::Buffer,
    last_scene: Option<Arc<BevOverlayScene>>,
    last_view: Option<[f32; 8]>,
    count: u32,
}
impl BevOverlayContext {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bev-overlay-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(32),
                },
                count: None,
            }],
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bev-overlay-view"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bev-overlay-view"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let items = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bev-overlay-items"),
            size: BEV_OVERLAY_BYTES,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bev-overlay"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/bev_overlay.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bev-overlay"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bev-overlay"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 48,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0=>Float32x4,1=>Float32x4,2=>Float32x4],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
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
        Self {
            pipeline,
            binding,
            uniform,
            items,
            last_scene: None,
            last_view: None,
            count: 0,
        }
    }
    pub fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        scene: &Arc<BevOverlayScene>,
        detector: DetectorRect,
        viewport: [f32; 2],
    ) -> KeplerResult<()> {
        DetectorRect::new(detector.top_left, detector.extent)?;
        let view = [
            detector.top_left.x as f32,
            detector.top_left.y as f32,
            detector.extent.x as f32,
            -detector.extent.y as f32,
            viewport[0],
            viewport[1],
            0.,
            0.,
        ];
        if !view.iter().all(|v| v.is_finite()) || viewport.iter().any(|v| *v <= 0.) {
            return Err(KeplerError::Graphics("Invalid BEV overlay view".into()));
        }
        if !self
            .last_scene
            .as_ref()
            .is_some_and(|last| Arc::ptr_eq(last, scene))
        {
            if !scene.items.is_empty() {
                queue.write_buffer(&self.items, 0, bytemuck::cast_slice(&scene.items));
            }
            self.count = scene.items.len() as u32;
            self.last_scene = Some(Arc::clone(scene));
        }
        if self.last_view != Some(view) {
            queue.write_buffer(&self.uniform, 0, bytemuck::cast_slice(&view));
            self.last_view = Some(view);
        }
        Ok(())
    }
    pub fn render(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.count > 0 {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.binding, &[]);
            pass.set_vertex_buffer(0, self.items.slice(..));
            pass.draw(0..6, 0..self.count);
        }
    }
}
