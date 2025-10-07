#![allow(dead_code)]

use wgpu::{Device, Queue};

use crate::rendering::{get_or_create_mesh_pipeline_with_depth, PipelineManager};

/// Function-level comment: Uniform data structure for camera matrices and position
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniforms {
    pub view_matrix: [[f32; 4]; 4],
    pub projection_matrix: [[f32; 4]; 4],
    pub view_projection_matrix: [[f32; 4]; 4],
    pub camera_position: [f32; 3],
    pub _padding: f32,
}

/// Function-level comment: Individual light source with proper 16-byte alignment for WGSL uniform buffers.
/// Supports directional, point, and spot lights with comprehensive lighting parameters.
/// Uses vec4 layout for optimal GPU alignment.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Light {
    pub position: [f32; 4], // xyz = position, w = unused
    pub color: [f32; 4], // rgb = color, w = intensity
    pub direction: [f32; 4], // xyz = direction, w = light_type (0: directional, 1: point, 2: spot)
    pub params: [f32; 4], // x = range, y = inner_cone_angle, z = outer_cone_angle, w = padding
}

/// Function-level comment: Enhanced lighting system supporting multiple light sources
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightingUniforms {
    pub lights: [Light; 8], // Support up to 8 lights
    pub num_lights: u32,
    pub ambient_color: [f32; 3],
    pub ambient_strength: f32,
    pub _padding: [f32; 3], // Padding to align to 16-byte boundary for WGSL compatibility
}

/// Function-level comment: Uniform data structure for model transformation matrices
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelUniforms {
    pub model_matrix: [[f32; 4]; 4],
    pub normal_matrix: [[f32; 4]; 4],
}

/// Function-level comment: Material properties for PBR rendering
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialProperties {
    pub albedo: [f32; 3],
    pub _albedo_padding: f32, // Padding for vec3 16-byte alignment in WGSL
    pub metallic: f32,
    pub roughness: f32,
    pub ao: f32,
    pub _scalar_padding: f32, // Padding to align emission to 16-byte boundary
    pub emission: [f32; 3],
    pub _emission_padding: f32, // Padding for vec3 16-byte alignment in WGSL
}

/// Function-level comment: Comprehensive buffer performance metrics for optimization analysis.
/// Provides detailed statistics about buffer usage, efficiency, and fragmentation.
#[derive(Debug, Clone)]
pub struct BufferMetrics {
    pub vertex_buffer_size: u64,
    pub index_buffer_size: u64,
    pub vertex_used_bytes: u64,
    pub index_used_bytes: u64,
    pub vertex_efficiency: f32,
    pub index_efficiency: f32,
    pub total_efficiency: f32,
    pub fragmentation_ratio: f32,
    pub num_vertices: u32,
    pub num_indices: u32,
}

/// Function-level comment: Buffer optimization suggestions based on usage patterns.
/// Provides recommendations for improving memory efficiency and performance.
#[derive(Debug, Clone)]
pub struct BufferOptimizationSuggestion {
    pub current_vertex_size: u64,
    pub current_index_size: u64,
    pub suggested_vertex_size: u64,
    pub suggested_index_size: u64,
    pub potential_memory_savings: u64,
    pub suggestions: Vec<String>,
}

pub struct MeshRenderContext {
    pub pipeline: std::sync::Arc<wgpu::RenderPipeline>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_vertices: u32,
    pub num_indices: u32,
    // Uniform buffers for shader data
    pub camera_uniform_buffer: wgpu::Buffer,
    pub lighting_uniform_buffer: wgpu::Buffer,
    pub model_uniform_buffer: wgpu::Buffer,
    pub material_uniform_buffer: wgpu::Buffer,
    // Bind groups for uniform buffers
    pub camera_bind_group: wgpu::BindGroup,
    pub lighting_bind_group: wgpu::BindGroup,
    pub model_bind_group: wgpu::BindGroup,
    pub material_bind_group: wgpu::BindGroup,
}

impl MeshRenderContext {
    /// Create a MeshRenderContext using the centralized mesh pipeline helper with indexed triangle rendering.
    /// This acquires a cached MeshBasic pipeline (TriangleList, CCW front face) without depth testing initially.
    /// It uploads both vertex and index buffers for efficient triangle rasterization across native and WASM.
    pub fn new(manager: &mut PipelineManager, device: &Device, queue: &Queue, mesh: &super::mesh::Mesh, use_depth: bool) -> Self {
        log::info!("MeshRenderContext::new - Starting");
        
        // Test 1: Check if pipeline creation is the issue
        log::info!("MeshRenderContext::new - Creating pipeline");
        let pipeline = get_or_create_mesh_pipeline_with_depth(manager, device, use_depth);
        log::info!("MeshRenderContext::new - Pipeline created successfully");

        // Test 2: Check if vertex buffer creation is the issue
        log::info!("MeshRenderContext::new - Creating vertex buffer, vertices count: {}", mesh.vertices.len());
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Mesh Vertex Buffer"),
            size: (mesh.vertices.len() * std::mem::size_of::<super::mesh::MeshVertex>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        log::info!("MeshRenderContext::new - Vertex buffer created, writing data");
        queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&mesh.vertices));
        log::info!("MeshRenderContext::new - Vertex buffer data written");
        let num_vertices = mesh.vertices.len() as u32;

        // Test 3: Check if index buffer creation is the issue
        log::info!("MeshRenderContext::new - Creating index buffer, indices count: {}", mesh.indices.len());
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Mesh Index Buffer"),
            size: (mesh.indices.len() * std::mem::size_of::<u32>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !mesh.indices.is_empty() {
            log::info!("MeshRenderContext::new - Writing index buffer data");
            queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(&mesh.indices));
            log::info!("MeshRenderContext::new - Index buffer data written");
        }
        let num_indices = mesh.indices.len() as u32;

        // Create uniform buffers for camera, lighting, and model data
        log::info!("MeshRenderContext::new - Creating uniform buffers");
        
        // Initialize default uniform data
        let identity_matrix = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
        
        let camera_uniforms = CameraUniforms {
            view_matrix: identity_matrix,
            projection_matrix: identity_matrix,
            view_projection_matrix: identity_matrix,
            camera_position: [0.0, 0.0, 5.0],
            _padding: 0.0,
        };
        
        // Create a default light setup with one directional light
        let default_light = Light {
            position: [2.0, 2.0, 2.0, 0.0], // xyz = position, w = unused
            color: [1.0, 1.0, 1.0, 1.0], // rgb = color, w = intensity
            direction: [-0.5, -0.5, -0.5, 0.0], // xyz = direction, w = light_type (0: directional)
            params: [100.0, 0.0, 0.0, 0.0], // x = range, y = inner_cone_angle, z = outer_cone_angle, w = padding
        };
        
        let mut lights = [Light {
            position: [0.0, 0.0, 0.0, 0.0],
            color: [0.0, 0.0, 0.0, 0.0],
            direction: [0.0, 0.0, 0.0, 0.0],
            params: [0.0, 0.0, 0.0, 0.0],
        }; 8];
        lights[0] = default_light; // Set the first light
        
        let lighting_uniforms = LightingUniforms {
            lights,
            num_lights: 1, // Only one light active
            ambient_color: [0.1, 0.1, 0.1],
            ambient_strength: 0.1,
            _padding: [0.0, 0.0, 0.0],
        };
        
        let model_uniforms = ModelUniforms {
            model_matrix: identity_matrix,
            normal_matrix: identity_matrix,
        };

        // Create uniform buffers
        let camera_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Uniform Buffer"),
            size: std::mem::size_of::<CameraUniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let lighting_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Lighting Uniform Buffer"),
            size: std::mem::size_of::<LightingUniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let model_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Model Uniform Buffer"),
            size: std::mem::size_of::<ModelUniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let material_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Material Uniform Buffer"),
            size: std::mem::size_of::<MaterialProperties>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create default material properties
        let material_properties = MaterialProperties {
            albedo: [0.6, 0.2, 0.8], // Purple
            _albedo_padding: 0.0,
            metallic: 0.0,
            roughness: 0.5,
            ao: 1.0,
            _scalar_padding: 0.0,
            emission: [0.0, 0.0, 0.0],
            _emission_padding: 0.0,
        };

        // Write initial data to uniform buffers
        queue.write_buffer(&camera_uniform_buffer, 0, bytemuck::cast_slice(&[camera_uniforms]));
        queue.write_buffer(&lighting_uniform_buffer, 0, bytemuck::cast_slice(&[lighting_uniforms]));
        queue.write_buffer(&model_uniform_buffer, 0, bytemuck::cast_slice(&[model_uniforms]));
        queue.write_buffer(&material_uniform_buffer, 0, bytemuck::cast_slice(&[material_properties]));

        // Create bind group layouts (these should match the pipeline's bind group layouts)
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let lighting_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Lighting Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let model_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Model Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let material_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Material Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Create bind groups
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform_buffer.as_entire_binding(),
            }],
        });

        let lighting_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Lighting Bind Group"),
            layout: &lighting_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: lighting_uniform_buffer.as_entire_binding(),
            }],
        });

        let model_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Model Bind Group"),
            layout: &model_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: model_uniform_buffer.as_entire_binding(),
            }],
        });

        let material_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Material Bind Group"),
            layout: &material_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: material_uniform_buffer.as_entire_binding(),
            }],
        });

        log::info!("MeshRenderContext::new - Completed successfully");
        Self { 
            pipeline, 
            vertex_buffer, 
            index_buffer, 
            num_vertices, 
            num_indices,
            camera_uniform_buffer,
            lighting_uniform_buffer,
            model_uniform_buffer,
            material_uniform_buffer,
            camera_bind_group,
            lighting_bind_group,
            model_bind_group,
            material_bind_group,
        }
    }

    /// Function-level comment: Update the pipeline to use depth testing when depth texture is available.
    /// This allows switching from a no-depth pipeline to a depth-enabled pipeline after initialization.
    pub fn enable_depth(&mut self, manager: &mut crate::rendering::core::pipeline::PipelineManager, device: &Device) {
        self.pipeline = crate::rendering::core::pipeline::get_or_create_mesh_pipeline_with_depth(manager, device, true);
    }

    /// Function-level comment: Update mesh data with dynamic buffer resizing for efficient memory usage.
    /// Reallocates buffers only when necessary to minimize GPU memory fragmentation.
    pub fn update_mesh(&mut self, device: &Device, queue: &Queue, mesh: &super::mesh::Mesh) -> Result<(), String> {
        // Validate mesh data before processing
        if mesh.vertices.is_empty() {
            return Err("Mesh must contain at least one vertex".to_string());
        }

        let new_vertex_size = (mesh.vertices.len() * std::mem::size_of::<super::mesh::MeshVertex>()) as wgpu::BufferAddress;
        let new_index_size = (mesh.indices.len() * std::mem::size_of::<u32>()) as wgpu::BufferAddress;

        // Resize vertex buffer if needed (with 25% growth buffer to reduce reallocations)
        if new_vertex_size > self.vertex_buffer.size() || new_vertex_size < self.vertex_buffer.size() / 2 {
            let buffer_size = (new_vertex_size as f32 * 1.25) as wgpu::BufferAddress;
            log::info!("Resizing vertex buffer from {} to {} bytes", self.vertex_buffer.size(), buffer_size);
            
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Mesh Vertex Buffer (Resized)"),
                size: buffer_size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        // Resize index buffer if needed
        if new_index_size > self.index_buffer.size() || new_index_size < self.index_buffer.size() / 2 {
            let buffer_size = (new_index_size as f32 * 1.25) as wgpu::BufferAddress;
            log::info!("Resizing index buffer from {} to {} bytes", self.index_buffer.size(), buffer_size);
            
            self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Mesh Index Buffer (Resized)"),
                size: buffer_size,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        // Update buffer data
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&mesh.vertices));
        if !mesh.indices.is_empty() {
            queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&mesh.indices));
        }

        // Update counts
        self.num_vertices = mesh.vertices.len() as u32;
        self.num_indices = mesh.indices.len() as u32;

        log::info!("Mesh updated: {} vertices, {} indices", self.num_vertices, self.num_indices);
        Ok(())
    }

    /// Function-level comment: Validate buffer state before rendering to prevent GPU errors.
    /// Performs comprehensive validation including size, alignment, and data integrity checks.
    pub fn validate_buffers(&self) -> Result<(), String> {
        if self.num_vertices == 0 {
            return Err("No vertices available for rendering".to_string());
        }

        let expected_vertex_size = (self.num_vertices as usize * std::mem::size_of::<super::mesh::MeshVertex>()) as wgpu::BufferAddress;
        if expected_vertex_size > self.vertex_buffer.size() {
            return Err(format!("Vertex buffer too small: expected {}, got {}", expected_vertex_size, self.vertex_buffer.size()));
        }

        if self.num_indices > 0 {
            let expected_index_size = (self.num_indices as usize * std::mem::size_of::<u32>()) as wgpu::BufferAddress;
            if expected_index_size > self.index_buffer.size() {
                return Err(format!("Index buffer too small: expected {}, got {}", expected_index_size, self.index_buffer.size()));
            }
        }

        // Validate buffer alignment for optimal GPU performance
        self.validate_buffer_alignment()?;

        // Additional validation: check for reasonable buffer sizes
        const MAX_REASONABLE_BUFFER_SIZE: u64 = 1024 * 1024 * 1024; // 1GB
        if self.vertex_buffer.size() > MAX_REASONABLE_BUFFER_SIZE {
            return Err(format!("Vertex buffer size {} exceeds reasonable limit", self.vertex_buffer.size()));
        }
        
        if self.index_buffer.size() > MAX_REASONABLE_BUFFER_SIZE {
            return Err(format!("Index buffer size {} exceeds reasonable limit", self.index_buffer.size()));
        }

        Ok(())
    }

    /// Function-level comment: Get buffer memory usage statistics for monitoring and optimization.
    pub fn get_memory_stats(&self) -> (u64, u64, f32, f32) {
        let vertex_used = (self.num_vertices as u64 * std::mem::size_of::<super::mesh::MeshVertex>() as u64);
        let index_used = (self.num_indices as u64 * std::mem::size_of::<u32>() as u64);
        let vertex_efficiency = vertex_used as f32 / self.vertex_buffer.size() as f32;
        let index_efficiency = if self.index_buffer.size() > 0 {
            index_used as f32 / self.index_buffer.size() as f32
        } else {
            1.0
        };
        
        (vertex_used, index_used, vertex_efficiency, index_efficiency)
    }

    /// Function-level comment: Get detailed buffer performance metrics for optimization analysis.
    /// Returns comprehensive statistics including fragmentation and allocation efficiency.
    pub fn get_detailed_buffer_metrics(&self) -> BufferMetrics {
        let vertex_used = (self.num_vertices as u64 * std::mem::size_of::<super::mesh::MeshVertex>() as u64);
        let index_used = (self.num_indices as u64 * std::mem::size_of::<u32>() as u64);
        let total_allocated = self.vertex_buffer.size() + self.index_buffer.size();
        let total_used = vertex_used + index_used;
        
        BufferMetrics {
            vertex_buffer_size: self.vertex_buffer.size(),
            index_buffer_size: self.index_buffer.size(),
            vertex_used_bytes: vertex_used,
            index_used_bytes: index_used,
            vertex_efficiency: vertex_used as f32 / self.vertex_buffer.size() as f32,
            index_efficiency: if self.index_buffer.size() > 0 {
                index_used as f32 / self.index_buffer.size() as f32
            } else {
                1.0
            },
            total_efficiency: total_used as f32 / total_allocated as f32,
            fragmentation_ratio: 1.0 - (total_used as f32 / total_allocated as f32),
            num_vertices: self.num_vertices,
            num_indices: self.num_indices,
        }
    }

    /// Function-level comment: Validate buffer alignment and GPU compatibility.
    /// Ensures buffers meet hardware requirements for optimal performance.
    pub fn validate_buffer_alignment(&self) -> Result<(), String> {
        // Check vertex buffer alignment (typically 4-byte aligned for f32)
        if self.vertex_buffer.size() % 4 != 0 {
            return Err("Vertex buffer size not aligned to 4-byte boundary".to_string());
        }

        // Check index buffer alignment (4-byte aligned for u32 indices)
        if self.index_buffer.size() % 4 != 0 {
            return Err("Index buffer size not aligned to 4-byte boundary".to_string());
        }

        // Validate vertex stride alignment
        let vertex_stride = std::mem::size_of::<super::mesh::MeshVertex>();
        if vertex_stride % 4 != 0 {
            return Err(format!("Vertex stride {} not aligned to 4-byte boundary", vertex_stride));
        }

        Ok(())
    }

    /// Function-level comment: Optimize buffer sizes based on usage patterns.
    /// Suggests optimal buffer sizes to reduce memory waste and improve performance.
    pub fn suggest_buffer_optimization(&self) -> BufferOptimizationSuggestion {
        let metrics = self.get_detailed_buffer_metrics();
        
        let mut suggestions = Vec::new();
        
        // Check for over-allocation
        if metrics.vertex_efficiency < 0.5 && metrics.vertex_buffer_size > 1024 {
            suggestions.push("Consider reducing vertex buffer size - current efficiency is low".to_string());
        }
        
        if metrics.index_efficiency < 0.5 && metrics.index_buffer_size > 1024 {
            suggestions.push("Consider reducing index buffer size - current efficiency is low".to_string());
        }
        
        // Check for potential under-allocation
        if metrics.vertex_efficiency > 0.95 {
            suggestions.push("Vertex buffer is nearly full - consider pre-allocating more space".to_string());
        }
        
        if metrics.index_efficiency > 0.95 {
            suggestions.push("Index buffer is nearly full - consider pre-allocating more space".to_string());
        }
        
        // Calculate optimal sizes (with some headroom)
        let optimal_vertex_size = (metrics.vertex_used_bytes as f32 * 1.5) as u64;
        let optimal_index_size = (metrics.index_used_bytes as f32 * 1.5) as u64;
        
        BufferOptimizationSuggestion {
            current_vertex_size: metrics.vertex_buffer_size,
            current_index_size: metrics.index_buffer_size,
            suggested_vertex_size: optimal_vertex_size,
            suggested_index_size: optimal_index_size,
            potential_memory_savings: (metrics.vertex_buffer_size + metrics.index_buffer_size)
                .saturating_sub(optimal_vertex_size + optimal_index_size),
            suggestions,
        }
    }

    /// Function-level comment: Update camera uniform buffer with view and projection matrices.
    /// Calculates view-projection matrix for efficient vertex transformation in the shader.
    pub fn update_camera_uniforms(&self, queue: &Queue, camera: &super::camera::Camera, aspect_ratio: f32) {
        let view_matrix = camera.view_matrix();
        let projection_matrix = camera.projection_matrix(aspect_ratio);
        let view_projection_matrix = camera.view_projection_matrix(aspect_ratio);
        
        let camera_uniforms = CameraUniforms {
            view_matrix: view_matrix.data,
            projection_matrix: projection_matrix.data,
            view_projection_matrix: view_projection_matrix.data,
            camera_position: camera.eye,
            _padding: 0.0,
        };
        
        queue.write_buffer(&self.camera_uniform_buffer, 0, bytemuck::cast_slice(&[camera_uniforms]));
    }

    /// Function-level comment: Update lighting uniform buffer with light direction and intensity.
    /// Provides data for PBR lighting calculations in the fragment shader.
    pub fn update_lighting_uniforms(&self, queue: &Queue, lighting: &super::lighting::Lighting) {
        // Create a light from the lighting data
        let light = Light {
            position: [lighting.direction[0], lighting.direction[1], lighting.direction[2], 0.0], // Use direction as light position for directional light
            color: [lighting.intensity, lighting.intensity, lighting.intensity, lighting.intensity], // Use intensity for white light
            direction: [lighting.direction[0], lighting.direction[1], lighting.direction[2], 0.0], // xyz = direction, w = light_type (0: directional)
            params: [100.0, 0.0, 0.0, 0.0], // x = range, y = inner_cone_angle, z = outer_cone_angle, w = padding
        };
        
        let mut lights = [Light {
            position: [0.0, 0.0, 0.0, 0.0],
            color: [0.0, 0.0, 0.0, 0.0],
            direction: [0.0, 0.0, 0.0, 0.0],
            params: [0.0, 0.0, 0.0, 0.0],
        }; 8];
        lights[0] = light; // Set the first light
        
        let lighting_uniforms = LightingUniforms {
            lights,
            num_lights: 1, // Only one light active
            ambient_color: [0.1, 0.1, 0.1],
            ambient_strength: 0.1,
            _padding: [0.0, 0.0, 0.0],
        };
        
        queue.write_buffer(&self.lighting_uniform_buffer, 0, bytemuck::cast_slice(&[lighting_uniforms]));
    }

    /// Function-level comment: Update model uniform buffer with transformation and normal matrices.
    /// Normal matrix is the inverse transpose of the model matrix for correct normal transformation.
    pub fn update_model_uniforms(&self, queue: &Queue, model_matrix: &crate::core::coord::Matrix4x4<f32>) {
       // Calculate normal matrix (inverse transpose of upper-left 3x3 of model matrix)
        let normal_matrix = model_matrix.inv().unwrap_or_else(|| crate::core::coord::Matrix4x4::eye()).transpose();
        
        let model_uniforms = ModelUniforms {
            model_matrix: model_matrix.data,
            normal_matrix: normal_matrix.data,
        };
        
        queue.write_buffer(&self.model_uniform_buffer, 0, bytemuck::cast_slice(&[model_uniforms]));
    }

    /// Function-level comment: Update all uniform buffers with default values when no specific data is provided.
    /// Sets up identity matrices and basic lighting for fallback rendering.
    pub fn update_default_uniforms(&self, queue: &Queue, aspect_ratio: f32) {
        log::debug!("[MESH_UNIFORMS] Updating default uniforms with aspect_ratio: {}", aspect_ratio);
        let identity_matrix = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
        
        // Create a simple perspective projection matrix
        let fov = 45.0_f32.to_radians();
        let near = 0.1;
        let far = 100.0;
        let f = 1.0 / (fov / 2.0).tan();
        let projection_matrix = [
            [f / aspect_ratio, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, (far + near) / (near - far), (2.0 * far * near) / (near - far)],
            [0.0, 0.0, -1.0, 0.0],
        ];
        
        // Create a simple view matrix (camera at (0, 0, 5) looking at origin)
        let view_matrix = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, -5.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        
        // Calculate proper view-projection matrix by multiplying projection * view
        let mut view_projection_matrix = [[0.0; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    view_projection_matrix[i][j] += projection_matrix[i][k] * view_matrix[k][j];
                }
            }
        }
        
        let camera_uniforms = CameraUniforms {
            view_matrix,
            projection_matrix,
            view_projection_matrix,
            camera_position: [0.0, 0.0, 5.0],
            _padding: 0.0,
        };
        
        // Create a default light setup
        let default_light = Light {
            position: [2.0, 2.0, 2.0, 0.0], // xyz = position, w = unused
            color: [1.0, 1.0, 1.0, 1.0], // rgb = color, w = intensity
            direction: [-0.5, -0.5, -0.5, 0.0], // xyz = direction, w = light_type (0: directional)
            params: [100.0, 0.0, 0.0, 0.0], // x = range, y = inner_cone_angle, z = outer_cone_angle, w = padding
        };
        
        let mut lights = [Light {
            position: [0.0, 0.0, 0.0, 0.0],
            color: [0.0, 0.0, 0.0, 0.0],
            direction: [0.0, 0.0, 0.0, 0.0],
            params: [0.0, 0.0, 0.0, 0.0],
        }; 8];
        lights[0] = default_light; // Set the first light
        
        let lighting_uniforms = LightingUniforms {
            lights,
            num_lights: 1, // Only one light active
            ambient_color: [0.1, 0.1, 0.1],
            ambient_strength: 0.1,
            _padding: [0.0, 0.0, 0.0],
        };
        
        let model_uniforms = ModelUniforms {
            model_matrix: identity_matrix,
            normal_matrix: identity_matrix,
        };
        
        queue.write_buffer(&self.camera_uniform_buffer, 0, bytemuck::cast_slice(&[camera_uniforms]));
        queue.write_buffer(&self.lighting_uniform_buffer, 0, bytemuck::cast_slice(&[lighting_uniforms]));
        queue.write_buffer(&self.model_uniform_buffer, 0, bytemuck::cast_slice(&[model_uniforms]));
    }
}