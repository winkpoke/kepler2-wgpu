use crate::geometry::GeometryBuilder;
use crate::{view, CTVolume};
use crate::render_content::RenderContent;
use crate::coord::Matrix4x4;

pub struct TransverseView {
    view: view::RenderContext,
    r_speed: f32,
    s_speed: f32,

    slice: f32,
    scale: f32,
    translate: [f32;3],

    pos: (i32, i32),
    dim: (u32, u32),
}

impl TransverseView {
    pub fn new(device: &wgpu::Device, texture: &RenderContent, 
               vol: &CTVolume, scale: f32, translate: [f32;3], 
               pos: (i32, i32), dim: (u32, u32)) -> Self {
        let r_speed = 0.00;
        let s_speed = 0.0005;
        
        let mut base_screen = GeometryBuilder::build_transverse_base(&vol);
        let base_uv = GeometryBuilder::build_uv_base(&vol);

        base_screen.scale(scale);
        base_screen.translate(translate);

        let transform_matrix = base_screen.to_base(&base_uv);
        println!("row major: {:?}", transform_matrix);

        let transform_matrix = transform_matrix.transpose(); // row major to column major
        println!("column major: {:?}", transform_matrix);

        let view = view::RenderContext::new(&device, &texture, transform_matrix);
        let slice = 0.0;

        Self {
            view,
            r_speed,
            s_speed,
            slice,
            scale,
            translate,
            pos,
            dim
        }
    }

    pub fn set_slice(&mut self, slice: f32) {
        self.slice = slice;
    }

    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    pub fn set_translate(&mut self, translate: [f32;3]) {
        self.translate = translate;
    }
}

impl view::Renderable for TransverseView {
    fn update(&mut self, queue: &wgpu::Queue) {
        // Update the rotation angle, e.g., incrementing it over time
        self.view.uniforms.vert.rotation_angle_y += self.r_speed; //0.05; // Update rotation angle
        // self.view.uniforms.vert.rotation_angle_z += self.r_speed; //0.05; // Update rotation angle
        if self.slice >= 1.0 {
            self.slice = 0.0;
        } else {
            self.slice += self.s_speed; //0.005;
        }

        self.view.uniforms.frag.slice = self.slice;

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
        render_pass.set_pipeline(&self.view.render_pipeline); // 2.

        let x: f32 = self.pos.0 as f32;
        let y: f32 = self.pos.1 as f32;
        let width = self.dim.0;
        let height = self.dim.1;

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

impl view::View for TransverseView {
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
}