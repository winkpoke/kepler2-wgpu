#![allow(dead_code)]

use std::any::Any;
use std::sync::Arc;
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
    fn set_slice_mm(&mut self, z: f32);
    fn set_scale(&mut self, scale: f32);
    fn set_translate(&mut self, translate: [f32; 3]);
    fn set_translate_in_screen_coord(&mut self, translate: [f32; 3]);
    fn set_pan(&mut self, x: f32, y: f32); // pan in screen space
    fn set_pan_mm(&mut self, x_mm: f32, y_mm: f32); // pan in mm
    /// Returns current window level used by the fragment shader.
    fn get_window_level(&self) -> f32;
    /// Returns current window width used by the fragment shader.
    fn get_window_width(&self) -> f32;
    /// Returns current slice position in millimeters along view normal.
    fn get_slice_mm(&self) -> f32;
    /// Returns current scale factor applied in screen space.
    fn get_scale(&self) -> f32;
    /// Returns current pan/translation in screen coordinates [x, y, z].
    fn get_translate_in_screen_coord(&self) -> [f32; 3];
    /// Returns current translation in view/model coordinates [x, y, z].
    fn get_translate(&self) -> [f32; 3];
}

#[derive(Debug, Clone, Copy)]
pub enum Orientation {
    Oblique,
    Sagittal,
    Coronal,
    Transverse,
}

pub const ALL_ORIENTATIONS: [Orientation; 4] = [
    Orientation::Transverse,
    Orientation::Coronal,
    Orientation::Sagittal,
    Orientation::Oblique,
];

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
    texture: Arc<RenderContent>,
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
        manager: &mut crate::pipeline::PipelineManager,
        device: &wgpu::Device,
        texture: Arc<RenderContent>,
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

        let pan = [0.0, 0.0, 0.0];
        let slice = 0.0;

        let mut base_screen_cloned = base_screen.clone();
        // the following is matrix multiplication so it's in reversed order
        base_screen_cloned.translate([-pan[0], -pan[1], -pan[2]]); 
        base_screen_cloned.translate([0.5, 0.5, 0.0]); // move back
        base_screen_cloned.scale([scale, scale, 1.0]);
        base_screen_cloned.translate([-0.5, -0.5, 0.0]); // move to center

        let transform_matrix = base_screen_cloned.to_base(&base_uv).transpose();

        let view = RenderContext::new(manager, device, &texture, transform_matrix);

        log::trace!("Created GenericMPRView with orientation: {:?}, scale: {}, translate: {:#?}, pos: {:#?}, dim: {:#?}",
            orientation, scale, translate, pos, dim);
        Self {
            view,
            texture,
            r_speed,
            s_speed,
            slice,
            base_screen,
            base_uv,
            scale,
            translate,
            pan,
            pos,
            dim,
        }
    }

    pub fn set_slice_speed(&mut self, speed: f32) {
        log::info!("MPRView set_slice_speed: {}", speed);
        self.s_speed = speed;
    }

    fn update_transform_matrix(&mut self) {
        let mut base_screen_cloned = self.base_screen.clone();
        base_screen_cloned.translate([-self.pan[0], -self.pan[1], -self.pan[2]]); 
        base_screen_cloned.translate([0.5, 0.5, 0.0]); // move back
        base_screen_cloned.scale([self.scale, self.scale, 1.0]);
        base_screen_cloned.translate([-0.5, -0.5, 0.0]); // move to center

        let transform_matrix = base_screen_cloned
            .to_base(&self.base_uv)
            .transpose();
        self.view.uniforms.frag.mat = *array_to_slice(&transform_matrix.data);
    }
}

impl Drop for GenericMPRView {
    fn drop(&mut self) {
        log::info!("Dropping GenericMPRView");
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
        log::trace!("View position: {:#?}", self.pos);
        self.pos
    }
    fn dimensions(&self) -> (u32, u32) {
        log::trace!("View dimensions: {:#?}", self.dim);
        self.dim
    }
    fn move_to(&mut self, pos: (i32, i32)) {
        log::trace!("View move_to: {:#?}", pos);
        self.pos = pos;
    }
    fn resize(&mut self, dim: (u32, u32)) {
        log::trace!("View resize: {:#?}", dim);
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
    fn set_slice_mm(&mut self, z: f32) {
        let [_, _, scale_z] = self.base_screen.get_scale_factors();
        self.pan[2] = z / scale_z;
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
    /// Function-level comment: Retrieve current window level from fragment uniforms for state snapshotting.
    fn get_window_level(&self) -> f32 { self.view.uniforms.frag.window_level }
    /// Function-level comment: Retrieve current window width from fragment uniforms for state snapshotting.
    fn get_window_width(&self) -> f32 { self.view.uniforms.frag.window_width }
    /// Function-level comment: Convert internal pan.z (in screen units) back to millimeters using base scale factors.
    fn get_slice_mm(&self) -> f32 {
        let [_, _, scale_z] = self.base_screen.get_scale_factors();
        self.pan[2] * scale_z
    }
    /// Function-level comment: Return current screen-space scale factor.
    fn get_scale(&self) -> f32 { self.scale }
    /// Function-level comment: Return current screen-space translation vector.
    fn get_translate_in_screen_coord(&self) -> [f32; 3] { self.pan }
    /// Function-level comment: Return current view/model-space translation vector.
    fn get_translate(&self) -> [f32; 3] { self.translate }
}

// Optional: keep type aliases for old names
pub type ObliqueView = GenericMPRView;
pub type SagittalView = GenericMPRView;
pub type TransverseView = GenericMPRView;
pub type CoronalView = GenericMPRView;