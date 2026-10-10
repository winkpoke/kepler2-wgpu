#![allow(dead_code)]
pub struct PassContext<'a> { pub pass: &'a mut wgpu::RenderPass<'a> }
fn execute<F>(encoder: &mut wgpu::CommandEncoder, descriptor: &wgpu::RenderPassDescriptor<'_>, mut callback: F)
where F: FnMut(PassContext<'_>) {
    let mut pass = encoder.begin_render_pass(descriptor).forget_lifetime();
    callback(PassContext { pass: &mut pass });
}
