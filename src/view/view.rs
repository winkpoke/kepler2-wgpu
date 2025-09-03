use std::any::Any;
use super::Renderable;
use crate::coord::{array_to_slice, Base};
use crate::geometry::GeometryBuilder;
use crate::render_content::RenderContent;
use crate::view::RenderContext;
use crate::CTVolume;

pub trait View: Renderable + Any {
    fn position(&self) -> (i32, i32);
    fn dimensions(&self) -> (u32, u32);
    fn move_to(&mut self, pos: (i32, i32));
    fn resize(&mut self, dim: (u32, u32));
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn as_mpr(&mut self) -> Option<&mut dyn MPRView> {
        None
    }
}

pub trait MPRView: View {
    // fn set_slice(&mut self, slice: u32);
    fn set_window_level(&mut self, window_level: f32);
    fn set_window_width(&mut self, window_width: f32);
    fn set_slice(&mut self, slice: f32);
    fn set_scale(&mut self, scale: f32);
    fn set_translate(&mut self, translate: [f32; 3]);
    fn set_translate_in_screen_coord(&mut self, translate: [f32; 3]);
    fn set_pan(&mut self, x: f32, y: f32); // pan in screen space
    fn set_pan_mm(&mut self, x_mm: f32, y_mm: f32); // pan in mm
}

pub enum Orientation {
    Oblique,
    Sagittal,
    Coronal,
    Transverse,
}

impl Orientation {
    fn build_base(&self, vol: &CTVolume) -> Base<f32> {
        match self {
            Orientation::Oblique => GeometryBuilder::build_oblique_base(vol),
            Orientation::Sagittal => GeometryBuilder::build_sagittal_base(vol),
            Orientation::Coronal => GeometryBuilder::build_coronal_base(vol),
            Orientation::Transverse => GeometryBuilder::build_transverse_base(vol),
        }
    }

    fn default_slice_speed(&self) -> f32 {
        match self {
            Orientation::Transverse => 0.006,
            _ => 0.0005,
        }
    }
}

pub struct GenericMPRView {
    view: RenderContext,
    r_speed: f32,
    s_speed: f32,
    slice: f32,
    base_screen: Base<f32>,
    base_uv: Base<f32>,
    scale: f32,
    translate: [f32; 3],
    pan: [f32; 3],
    pos: (i32, i32),
    dim: (u32, u32),
}

impl GenericMPRView {
    pub fn new(
        device: &wgpu::Device,
        texture: &RenderContent,
        vol: &CTVolume,
        orientation: Orientation,
        scale: f32,
        translate: [f32; 3],
        pos: (i32, i32),
        dim: (u32, u32),
    ) -> Self {
        let r_speed = 0.0;
        let s_speed = orientation.default_slice_speed();

        let base_screen = orientation.build_base(vol);
        let base_uv = GeometryBuilder::build_uv_base(vol);

        let mut base_screen_with_scale = base_screen.clone();
        base_screen_with_scale.scale(scale);
        let mut base_screen_with_translate = base_screen_with_scale.clone();
        base_screen_with_translate.translate(translate);

        let transform_matrix = base_screen_with_translate.to_base(&base_uv).transpose();

        let view = RenderContext::new(&device, &texture, transform_matrix);

        Self {
            view,
            r_speed,
            s_speed,
            slice: 0.0,
            base_screen,
            base_uv,
            scale,
            translate,
            pan: [0.0, 0.0, 0.0],
            pos,
            dim,
        }
    }

    pub fn set_slice_speed(&mut self, speed: f32) {
        log::info!("MPRView set_slice_speed: {}", speed);
        self.s_speed = speed;
    }

    fn update_transform_matrix(&mut self) {
        let mut base_screen_with_scale = self.base_screen.clone();
        base_screen_with_scale.scale(self.scale);
        let mut base_screen_with_translate = base_screen_with_scale.clone();
        base_screen_with_translate.translate_in_screen_coord(self.pan);
        base_screen_with_translate.translate(self.translate);
        let transform_matrix = base_screen_with_translate
            .to_base(&self.base_uv)
            .transpose();
        self.view.uniforms.frag.mat = *array_to_slice(&transform_matrix.data);
    }
}

impl Renderable for GenericMPRView {
    fn update(&mut self, queue: &wgpu::Queue) {
        self.view.uniforms.vert.rotation_angle_y += self.r_speed;
        self.view.uniforms.frag.slice = self.slice;
        self.update_transform_matrix();

        queue.write_buffer(
            &self.view.uniform_vert_buffer,
            0,
            bytemuck::cast_slice(&[self.view.uniforms.vert]),
        );
        queue.write_buffer(
            &self.view.uniform_frag_buffer,
            0,
            bytemuck::cast_slice(&[self.view.uniforms.frag]),
        );
    }

    fn render(&mut self, render_pass: &mut wgpu::RenderPass) -> Result<(), wgpu::SurfaceError> {
        render_pass.set_pipeline(&self.view.render_pipeline);

        let (x, y) = (self.pos.0 as f32, self.pos.1 as f32);
        let (width, height) = (self.dim.0, self.dim.1);

        render_pass.set_viewport(x, y, width as f32, height as f32, 0.0, 1.0);
        render_pass.set_bind_group(0, &self.view.texture_bind_group, &[]);
        render_pass.set_bind_group(1, &self.view.uniform_vert_bind_group, &[]);
        render_pass.set_bind_group(2, &self.view.uniform_frag_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.view.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.view.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.view.num_indices, 0, 0..1);
        Ok(())
    }
}

impl View for GenericMPRView {
    fn position(&self) -> (i32, i32) {
        self.pos
    }
    fn dimensions(&self) -> (u32, u32) {
        self.dim
    }
    fn move_to(&mut self, pos: (i32, i32)) {
        self.pos = pos;
    }
    fn resize(&mut self, dim: (u32, u32)) {
        self.dim = dim;
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn as_mpr(&mut self) -> Option<&mut dyn MPRView> {
        Some(self)
    }
}

impl MPRView for GenericMPRView {
    fn set_window_level(&mut self, window_level: f32) {
        self.view.uniforms.frag.window_level = window_level;
    }
    fn set_window_width(&mut self, window_width: f32) {
        self.view.uniforms.frag.window_width = window_width;
    }
    fn set_slice(&mut self, slice: f32) {
        self.slice = slice.clamp(0.0, 1.0);
    }
    fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }
    fn set_translate(&mut self, translate: [f32; 3]) {
        self.translate = translate;
    }
    fn set_translate_in_screen_coord(&mut self, translate: [f32; 3]) {
        self.pan = translate;
    }
    fn set_pan(&mut self, x: f32, y: f32) {
        self.pan[0] = x;
        self.pan[1] = y;
    }
    fn set_pan_mm(&mut self, x_mm: f32, y_mm: f32) {
        let [scale_x, scale_y, _] = self.base_screen.get_scale_factors();
        self.pan[0] = x_mm / scale_x;
        self.pan[1] = y_mm / scale_y;
    }
}

// Optional: keep type aliases for old names
pub type ObliqueView = GenericMPRView;
pub type SagittalView = GenericMPRView;
pub type TransverseView = GenericMPRView;
pub type CoronalView = GenericMPRView;