use crate::geometry::GeometryBuilder;
use crate::{view, CTVolume};
use crate::render_content::RenderContent;
use crate::coord::{array_to_slice, Base, Matrix4x4};

pub struct TransverseView {
    view: view::RenderContext,
    r_speed: f32,
    s_speed: f32,

    slice: f32,
    base_screen: Base<f32>,
    base_uv: Base<f32>,
    scale: f32,
    translate: [f32;3],

    pos: (i32, i32),
    dim: (u32, u32),
}

impl TransverseView {
    pub fn new(device: &wgpu::Device, texture: &RenderContent, 
               vol: &CTVolume, scale: f32, translate: [f32;3], 
            //    pos: (i32, i32), dim: (u32, u32)
            ) -> Self {
        let r_speed = 0.00;
        let s_speed = 0.006;
        
        let base_screen = GeometryBuilder::build_transverse_base(&vol);
        let base_uv = GeometryBuilder::build_uv_base(&vol);

        let mut base_screen_with_scale = base_screen.clone();
        base_screen_with_scale.scale(scale);
        let mut base_screen_with_translate = base_screen_with_scale.clone();
        base_screen_with_translate.translate(translate);

        let transform_matrix = base_screen_with_translate.to_base(&base_uv);
        println!("row major: {:?}", transform_matrix);

        let transform_matrix = transform_matrix.transpose(); // row major to column major
        println!("column major: {:?}", transform_matrix);

        let view = view::RenderContext::new(&device, &texture, transform_matrix);
        let slice = 0.0;

        let pos = (0, 0); // Default position
        let dim = (800, 800); // Default dimensions

        Self {
            view,
            r_speed,
            s_speed,
            slice,
            base_screen,
            base_uv,
            scale,
            translate,
            pos,
            dim
        }
    }

    pub fn set_translate(&mut self, translate: [f32;3]) {
        self.translate = translate;
    }

    pub fn set_slice_speed(&mut self, speed: f32) {
        log::info!("TransverseView set_slice_speed: {}", speed);
        self.s_speed = speed;
        log::info!("TransverseView slice_speed set to: {}", self.s_speed);
    }

    fn update_transform_matrix(&mut self) {
        let mut base_screen_with_scale = self.base_screen.clone();
        base_screen_with_scale.scale(self.scale);
        let mut base_screen_with_translate = base_screen_with_scale.clone();
        base_screen_with_translate.translate(self.translate);
        let transform_matrix = base_screen_with_translate.to_base(&self.base_uv);
        let transform_matrix = transform_matrix.transpose(); 
        self.view.uniforms.frag.mat = *array_to_slice(&transform_matrix.data);
    }
}

impl view::Renderable for TransverseView {
    fn update(&mut self, queue: &wgpu::Queue) {
        // Update the rotation angle, e.g., incrementing it over time
        self.view.uniforms.vert.rotation_angle_y += self.r_speed; //0.05; // Update rotation angle
        // self.view.uniforms.vert.rotation_angle_z += self.r_speed; //0.05; // Update rotation angle
        // if self.slice >= 1.0 {
        //     self.slice = 0.0;
        // } else {
        //     self.slice += self.s_speed; //0.005;
        // }

        self.update_transform_matrix();

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
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_mpr(&mut self) -> Option<&mut dyn view::MPRView> {
        Some(self)
    }
}

impl view::MPRView for TransverseView {
    fn set_window_level(&mut self, window_level: f32) {
        self.view.uniforms.frag.window_level = window_level;
    }
    fn set_window_width(&mut self, window_width: f32) {
        self.view.uniforms.frag.window_width = window_width;
    }
    fn set_slice(&mut self, slice: f32) {
        // check the value of slice
        // it shall no more than 1.0 and no less than 0.0
        let mut slice = slice;
        if slice > 1.0 {
            slice = 1.0;
            log::info!("TransverseView set_slice: slice value exceeded 1.0, setting to 1.0");
        } else if slice < 0.0 {
            slice = 0.0;
            log::info!("TransverseView set_slice: slice value less than 0.0, setting to 0.0");
        }
        self.slice = slice;
    }

    fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
        log::info!("TransverseView set_scale: scale set to {}", scale);
    }

    fn set_translate(&mut self, translate: [f32; 3]) {
        self.set_translate(translate);
        log::info!("TransverseView set_translate: translate set to {:?}", translate);
    }
}