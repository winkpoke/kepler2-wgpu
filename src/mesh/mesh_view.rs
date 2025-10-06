#![allow(dead_code)]

use super::{mesh::Mesh, material::Material, camera::Camera, lighting::Lighting};

use crate::view::{Renderable, View};

#[derive(Default)]
pub struct MeshView {
    pub mesh: Option<Mesh>,
    pub material: Option<Material>,
    pub camera: Option<Camera>,
    pub lighting: Option<Lighting>,
    ctx: Option<super::mesh_render_context::MeshRenderContext>,
    pos: (i32, i32),
    dim: (u32, u32),
}

impl MeshView {
    pub fn new() -> Self { Self::default() }
    pub fn attach_context(&mut self, ctx: super::mesh_render_context::MeshRenderContext) {
        self.ctx = Some(ctx);
    }
}

impl Renderable for MeshView {
    fn update(&mut self, queue: &wgpu::Queue) {
        // inert
        let _ = queue; // silence unused for now
    }
    fn render(&mut self, render_pass: &mut wgpu::RenderPass) -> Result<(), wgpu::SurfaceError> {
        if let Some(ctx) = &self.ctx {
            render_pass.set_pipeline(&*ctx.pipeline);
            render_pass.set_vertex_buffer(0, ctx.vertex_buffer.slice(..));
            let (x, y) = (self.pos.0 as f32, self.pos.1 as f32);
            let (width, height) = (self.dim.0, self.dim.1);
            render_pass.set_viewport(x, y, width as f32, height as f32, 0.0, 1.0);
            render_pass.draw(0..ctx.num_vertices, 0..1);
        }
        Ok(())
    }
}

impl View for MeshView {
    fn position(&self) -> (i32, i32) { self.pos }
    fn dimensions(&self) -> (u32, u32) { self.dim }
    fn move_to(&mut self, pos: (i32, i32)) { self.pos = pos; }
    fn resize(&mut self, dim: (u32, u32)) { self.dim = dim; }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}