#![allow(dead_code)]

use std::sync::Arc;
use crate::core::coord::{array_to_slice, Matrix4x4};
use crate::rendering::view::render_content::RenderContent;
use super::mpr_render_context::MprRenderContext;

/// Uniform data structures for MPR rendering
#[repr(C)]
#[derive(Copy, Clone, Default, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformsVert {
    pub _padding: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformsFrag {
    pub window_width: f32,
    pub window_level: f32,
    pub slice: f32,
    pub is_packed_rg8: f32,
    pub bias: f32,
    pub _pad0: [f32; 3],
    pub mat: [f32; 16],
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub vert: UniformsVert,
    pub frag: UniformsFrag,
}

/// Per-view GPU implementation for MPR rendering
/// Contains view-specific GPU resources like bind groups and uniform buffers
/// References shared context and content for resource efficiency
pub struct MprViewWgpuImpl {
    /// Reference to shared global GPU state
    pub render_context: Arc<MprRenderContext>,
    
    /// Reference to shared texture content
    pub render_content: Arc<RenderContent>,
    
    /// View-specific texture bind group
    pub texture_bind_group: wgpu::BindGroup,
    
    /// View-specific vertex uniform buffer
    pub uniform_vert_buffer: wgpu::Buffer,
    
    /// View-specific vertex uniform bind group
    pub uniform_vert_bind_group: wgpu::BindGroup,
    
    /// View-specific fragment uniform buffer
    pub uniform_frag_buffer: wgpu::Buffer,
    
    /// View-specific fragment uniform bind group
    pub uniform_frag_bind_group: wgpu::BindGroup,
    
    /// Current uniform values for this view
    pub uniforms: Uniforms,
}

impl MprViewWgpuImpl {
    /// Create a new MprViewWgpuImpl with view-specific GPU resources
    /// 
    /// # Arguments
    /// * `render_context` - Shared global GPU state
    /// * `device` - WGPU device for creating GPU resources
    /// * `render_content` - Shared texture content
    /// * `transform_matrix` - 4x4 matrix for view transforms
    /// 
    /// # Returns
    /// A new MprViewWgpuImpl with initialized per-view resources
    pub fn new(
        render_context: Arc<MprRenderContext>,
        device: &wgpu::Device,
        render_content: Arc<RenderContent>,
        transform_matrix: Matrix4x4<f32>,
    ) -> Self {
        // Initialize uniform data
        let u_vert_data = UniformsVert {
            ..Default::default()
        };
        
        let is_packed = matches!(render_content.texture_format, wgpu::TextureFormat::Rg8Unorm);
        let u_frag_data = UniformsFrag {
            window_width: 350.,
            window_level: 40.0,
            slice: 0.0,
            is_packed_rg8: if is_packed { 1.0 } else { 0.0 },
            bias: if is_packed { 1100.0 } else { 0.0 },
            mat: *array_to_slice(&transform_matrix.data),
            ..Default::default()
        };
        
        log::info!(
            "MprViewWgpuImpl defaults => window_width: {:.1}, window_level: {:.1}, is_packed_rg8: {}",
            u_frag_data.window_width,
            u_frag_data.window_level,
            is_packed
        );
        
        let uniforms = Uniforms {
            vert: u_vert_data,
            frag: u_frag_data,
        };

        // Create view-specific texture bind group using shared layout
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &render_context.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&render_content.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&render_content.sampler),
                },
            ],
            label: Some("mpr_view_texture_bind_group"),
        });

        // Create view-specific vertex uniform buffer and bind group
        let (uniform_vert_buffer, uniform_vert_bind_group) = 
            Self::create_vertex_uniform_bind_group(device, &render_context.vertex_bind_group_layout, &uniforms.vert);

        // Create view-specific fragment uniform buffer and bind group
        let (uniform_frag_buffer, uniform_frag_bind_group) = 
            Self::create_fragment_uniform_bind_group(device, &render_context.fragment_bind_group_layout, &uniforms.frag);

        log::debug!("MprViewWgpuImpl created with view-specific GPU resources");

        Self {
            render_context,
            render_content,
            texture_bind_group,
            uniform_vert_buffer,
            uniform_vert_bind_group,
            uniform_frag_buffer,
            uniform_frag_bind_group,
            uniforms,
        }
    }

    /// Set the uniform values for this view
    /// 
    /// # Arguments
    /// * `new_uniforms` - New uniform values to set
    pub fn set_uniforms(&mut self, new_uniforms: Uniforms) {
        // Use the separate set methods for better modularity and consistency
        self.set_vertex_uniforms(new_uniforms.vert);
        self.set_fragment_uniforms(new_uniforms.frag);
        
        log::trace!("Set both vertex and fragment uniform values");
    }

    /// Set transformation matrix
    /// 
    /// # Arguments
    /// * `matrix` - New transformation matrix
    pub fn set_matrix(&mut self, matrix: [f32; 16]) {
        self.uniforms.frag.mat = matrix;
    }

    /// Set slice position
    /// 
    /// # Arguments
    /// * `slice` - New slice position
    pub fn set_slice(&mut self, slice: f32) {
        self.uniforms.frag.slice = slice;
    }

    /// Set window level only
    /// 
    /// # Arguments
    /// * `window_level` - New window level value
    pub fn set_window_level(&mut self, window_level: f32) {
        self.uniforms.frag.window_level = window_level;
    }

    /// Set window width only
    /// 
    /// # Arguments
    /// * `window_width` - New window width value
    pub fn set_window_width(&mut self, window_width: f32) {
        self.uniforms.frag.window_width = window_width;
    }

    /// Set only the vertex uniform values
    /// 
    /// # Arguments
    /// * `vertex_uniforms` - New vertex uniform values to set
    pub fn set_vertex_uniforms(&mut self, vertex_uniforms: UniformsVert) {
        self.uniforms.vert = vertex_uniforms;
        
        log::trace!("Set vertex uniform values");
    }

    /// Set only the fragment uniform values
    /// 
    /// # Arguments
    /// * `fragment_uniforms` - New fragment uniform values to set
    pub fn set_fragment_uniforms(&mut self, fragment_uniforms: UniformsFrag) {
        self.uniforms.frag = fragment_uniforms;
        
        log::trace!("Set fragment uniform values with window_width: {:.1}, window_level: {:.1}, slice: {:.1}", 
                   fragment_uniforms.window_width, fragment_uniforms.window_level, fragment_uniforms.slice);
    }

    /// Update vertex uniform buffer with current uniform values
    /// 
    /// # Arguments
    /// * `queue` - WGPU queue for buffer updates
    pub fn update_vertex_uniforms_buffer(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.uniform_vert_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms.vert]),
        );
        
        log::trace!("Updated vertex uniform buffer");
    }

    /// Update fragment uniform buffer with current uniform values
    /// 
    /// # Arguments
    /// * `queue` - WGPU queue for buffer updates
    pub fn update_fragment_uniforms_buffer(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.uniform_frag_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms.frag]),
        );
        
        log::trace!("Updated fragment uniform buffer with window_width: {:.1}, window_level: {:.1}, slice: {:.1}", 
                   self.uniforms.frag.window_width, self.uniforms.frag.window_level, self.uniforms.frag.slice);
    }

    /// Update both uniform buffers with current uniform values
    /// 
    /// # Arguments
    /// * `queue` - WGPU queue for buffer updates
    pub fn update_uniforms_buffers(&self, queue: &wgpu::Queue) {
        self.update_vertex_uniforms_buffer(queue);
        self.update_fragment_uniforms_buffer(queue);
        
        log::trace!("Updated both vertex and fragment uniform buffers");
    }

    /// Helper function to create vertex uniform buffer and bind group
    fn create_vertex_uniform_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        uniforms: &UniformsVert,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        use wgpu::util::DeviceExt;
        
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MPR Vertex Uniform Buffer"),
            contents: bytemuck::cast_slice(&[*uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("MPR Vertex Uniform Bind Group"),
        });

        (buffer, bind_group)
    }

    /// Helper function to create fragment uniform buffer and bind group
    fn create_fragment_uniform_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        uniforms: &UniformsFrag,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        use wgpu::util::DeviceExt;
        
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MPR Fragment Uniform Buffer"),
            contents: bytemuck::cast_slice(&[*uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("MPR Fragment Uniform Bind Group"),
        });

        (buffer, bind_group)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Function-level comment: 验证片段统一缓冲结构体的尺寸与对齐（含专用填充字段）
    #[test]
    fn test_uniforms_frag_size_alignment() {
        let size = std::mem::size_of::<UniformsFrag>();
        assert_eq!(size, 96);
        let vert_size = std::mem::size_of::<UniformsVert>();
        assert_eq!(vert_size, 16);
        let uniforms_size = std::mem::size_of::<Uniforms>();
        assert_eq!(uniforms_size, vert_size + size);
    }
}
