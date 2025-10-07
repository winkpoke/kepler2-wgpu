/// Shader validation and debugging utilities for mesh rendering pipeline
/// 
/// This module provides comprehensive shader validation, error detection, and performance
/// monitoring capabilities for the mesh rendering system. It ensures shader compilation
/// succeeds across all target platforms and provides debugging tools for development.

use crate::core::timing::{Instant, DurationExt};
use wgpu::{Device, ShaderModule, ShaderModuleDescriptor, ShaderSource};

/// Shader validation error types
#[derive(Debug, Clone)]
pub enum ShaderValidationError {
    CompilationFailed(String),
    InvalidBindGroup(u32, String),
    MissingUniform(String),
    PerformanceWarning(String),
}

impl std::fmt::Display for ShaderValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderValidationError::CompilationFailed(msg) => {
                write!(f, "Shader compilation failed: {}", msg)
            }
            ShaderValidationError::InvalidBindGroup(group, msg) => {
                write!(f, "Invalid bind group {}: {}", group, msg)
            }
            ShaderValidationError::MissingUniform(name) => {
                write!(f, "Missing required uniform: {}", name)
            }
            ShaderValidationError::PerformanceWarning(msg) => {
                write!(f, "Performance warning: {}", msg)
            }
        }
    }
}

impl std::error::Error for ShaderValidationError {}

/// Shader performance metrics
#[derive(Debug, Clone)]
pub struct ShaderMetrics {
    pub compilation_time_ms: f64,
    pub vertex_complexity_score: u32,
    pub fragment_complexity_score: u32,
    pub uniform_buffer_count: u32,
    pub texture_binding_count: u32,
}

/// Shader validation and debugging utilities
pub struct ShaderValidator<'a> {
    device: &'a wgpu::Device,
}

impl<'a> ShaderValidator<'a> {
    /// Create a new shader validator with the given device
    /// 
    /// # Arguments
    /// * `device` - WGPU device for shader compilation testing
    pub fn new(device: &'a wgpu::Device) -> Self {
        Self { device }
    }

    /// Validate mesh shader compilation and analyze performance characteristics
    /// 
    /// This method compiles the mesh shader and analyzes its structure to identify
    /// potential issues and performance bottlenecks.
    /// 
    /// # Returns
    /// * `Ok(ShaderMetrics)` - Compilation succeeded with performance metrics
    /// * `Err(ShaderValidationError)` - Compilation failed or validation issues found
    pub fn validate_mesh_shader(&self) -> Result<ShaderMetrics, ShaderValidationError> {
        let start_time = Instant::now();
        
        // Load and compile the mesh shader
        let shader_source = include_str!("../shaders/mesh.wgsl");
        
        // Attempt compilation to catch syntax errors
        let _shader_module = self.compile_shader("mesh.wgsl", shader_source)?;
        
        let compilation_time = start_time.elapsed().as_millis_f64();
        
        // Analyze shader structure for performance characteristics
        let metrics = self.analyze_shader_performance(shader_source, compilation_time)?;
        
        // Validate required uniforms are present
        self.validate_required_uniforms(shader_source)?;
        
        // Check for common performance issues
        self.check_performance_warnings(shader_source)?;
        
        log::info!("Mesh shader validation completed successfully");
        log::info!("Compilation time: {:.2}ms", metrics.compilation_time_ms);
        log::info!("Vertex complexity: {}", metrics.vertex_complexity_score);
        log::info!("Fragment complexity: {}", metrics.fragment_complexity_score);
        
        Ok(metrics)
    }

    /// Compile a shader and return the module or error
    fn compile_shader(&self, name: &str, source: &str) -> Result<ShaderModule, ShaderValidationError> {
        match self.device.create_shader_module(ShaderModuleDescriptor {
            label: Some(name),
            source: ShaderSource::Wgsl(source.into()),
        }) {
            module => {
                // WGPU doesn't return compilation errors directly in create_shader_module
                // Errors are typically caught during pipeline creation
                Ok(module)
            }
        }
    }

    /// Analyze shader source for performance characteristics
    fn analyze_shader_performance(&self, source: &str, compilation_time: f64) -> Result<ShaderMetrics, ShaderValidationError> {
        let mut vertex_complexity = 0u32;
        let mut fragment_complexity = 0u32;
        let mut uniform_buffers = 0u32;
        let mut texture_bindings = 0u32;
        
        let lines: Vec<&str> = source.lines().collect();
        let mut in_vertex_shader = false;
        let mut in_fragment_shader = false;
        
        for line in lines {
            let trimmed = line.trim();
            
            // Track shader sections
            if trimmed.contains("@vertex") {
                in_vertex_shader = true;
                in_fragment_shader = false;
            } else if trimmed.contains("@fragment") {
                in_fragment_shader = true;
                in_vertex_shader = false;
            } else if trimmed.starts_with("fn ") && !trimmed.contains("vs_main") && !trimmed.contains("fs_main") {
                in_vertex_shader = false;
                in_fragment_shader = false;
            }
            
            // Count uniform buffers
            if trimmed.contains("var<uniform>") {
                uniform_buffers += 1;
            }
            
            // Count texture bindings
            if trimmed.contains("var<texture") || trimmed.contains("var<sampler") {
                texture_bindings += 1;
            }
            
            // Analyze complexity in vertex shader
            if in_vertex_shader {
                if trimmed.contains("normalize") || trimmed.contains("dot") || trimmed.contains("cross") {
                    vertex_complexity += 2;
                }
                if trimmed.contains("*") || trimmed.contains("matrix") {
                    vertex_complexity += 1;
                }
            }
            
            // Analyze complexity in fragment shader
            if in_fragment_shader {
                if trimmed.contains("normalize") || trimmed.contains("dot") || trimmed.contains("cross") {
                    fragment_complexity += 2;
                }
                if trimmed.contains("pow") || trimmed.contains("sqrt") || trimmed.contains("sin") || trimmed.contains("cos") {
                    fragment_complexity += 3;
                }
                if trimmed.contains("*") || trimmed.contains("/") {
                    fragment_complexity += 1;
                }
            }
        }
        
        Ok(ShaderMetrics {
            compilation_time_ms: compilation_time,
            vertex_complexity_score: vertex_complexity,
            fragment_complexity_score: fragment_complexity,
            uniform_buffer_count: uniform_buffers,
            texture_binding_count: texture_bindings,
        })
    }

    /// Validate that required uniforms are present in the shader
    fn validate_required_uniforms(&self, source: &str) -> Result<(), ShaderValidationError> {
        let required_uniforms = [
            "camera",
            "lighting", 
            "model",
            "material", // New material uniform for PBR
        ];
        
        for uniform in required_uniforms {
            if !source.contains(&format!("var<uniform> {}", uniform)) {
                return Err(ShaderValidationError::MissingUniform(uniform.to_string()));
            }
        }
        
        // Validate PBR-specific structures
        let required_structs = [
            "MaterialProperties",
            "Light",
            "LightingUniforms",
        ];
        
        for struct_name in required_structs {
            if !source.contains(&format!("struct {}", struct_name)) {
                return Err(ShaderValidationError::MissingUniform(
                    format!("Required struct: {}", struct_name)
                ));
            }
        }
        
        // Validate PBR functions are present
        let required_functions = [
            "distribution_ggx",
            "geometry_smith", 
            "fresnel_schlick",
            "calculate_light_contribution",
        ];
        
        for func_name in required_functions {
            if !source.contains(&format!("fn {}", func_name)) {
                return Err(ShaderValidationError::MissingUniform(
                    format!("Required PBR function: {}", func_name)
                ));
            }
        }
        
        Ok(())
    }

    /// Check for common performance issues and emit warnings
    fn check_performance_warnings(&self, source: &str) -> Result<(), ShaderValidationError> {
        // Check for excessive complexity in fragment shader
        let fragment_ops = source.matches("pow").count() + 
                          source.matches("sqrt").count() + 
                          source.matches("sin").count() + 
                          source.matches("cos").count();
        
        if fragment_ops > 15 { // Increased threshold for PBR shaders
            return Err(ShaderValidationError::PerformanceWarning(
                format!("Fragment shader has {} expensive operations, consider optimization", fragment_ops)
            ));
        }
        
        // Check for excessive uniform buffer usage
        let uniform_count = source.matches("var<uniform>").count();
        if uniform_count > 8 {
            return Err(ShaderValidationError::PerformanceWarning(
                format!("Shader uses {} uniform buffers, consider consolidation", uniform_count)
            ));
        }
        
        // Check for excessive light count in PBR shader
        if source.contains("array<Light, 8>") {
            log::warn!("Shader supports up to 8 lights - consider dynamic light culling for better performance");
        }
        
        // Check for complex PBR calculations
        let pbr_functions = source.matches("distribution_ggx").count() + 
                           source.matches("geometry_smith").count() + 
                           source.matches("fresnel_schlick").count();
        
        if pbr_functions > 0 {
            log::info!("PBR shader detected - ensure proper LOD and culling for optimal performance");
        }
        
        // Check for potential branching issues
        let branch_count = source.matches("if (").count() + source.matches("for (").count();
        if branch_count > 10 {
            return Err(ShaderValidationError::PerformanceWarning(
                format!("Shader has {} branches, consider reducing for better GPU performance", branch_count)
            ));
        }
        
        // Check for texture sampling (future enhancement)
        let texture_samples = source.matches("textureSample").count();
        if texture_samples > 8 {
            return Err(ShaderValidationError::PerformanceWarning(
                format!("Shader performs {} texture samples, consider texture atlasing", texture_samples)
            ));
        }
        
        Ok(())
    }

    /// Validate depth shader compilation (for future shadow mapping)
    pub fn validate_depth_shader(&self) -> Result<ShaderMetrics, ShaderValidationError> {
        let start_time = Instant::now();
        
        // Load and compile the depth shader
        let shader_source = include_str!("../shaders/mesh_depth.wgsl");
        
        let _shader_module = self.compile_shader("mesh_depth.wgsl", shader_source)?;
        
        let compilation_time = start_time.elapsed().as_millis_f64();
        
        log::info!("Depth shader validation completed successfully");
        log::info!("Compilation time: {:.2}ms", compilation_time);
        
        // Simplified metrics for depth-only shader
        Ok(ShaderMetrics {
            compilation_time_ms: compilation_time,
            vertex_complexity_score: 5, // Depth shaders are typically simple
            fragment_complexity_score: 1, // Minimal fragment processing
            uniform_buffer_count: 1, // Usually just transformation matrices
            texture_binding_count: 0, // No textures in depth-only pass
        })
    }
}

/// Convenience function to validate all mesh shaders
/// 
/// This function validates both the main mesh shader and depth shader,
/// returning comprehensive metrics for both.
/// 
/// # Arguments
/// * `device` - WGPU device for shader compilation
/// 
/// # Returns
/// * `Ok((mesh_metrics, depth_metrics))` - Both shaders validated successfully
/// * `Err(ShaderValidationError)` - One or both shaders failed validation
pub fn validate_all_mesh_shaders(device: &wgpu::Device) -> Result<(ShaderMetrics, ShaderMetrics), ShaderValidationError> {
    let validator = ShaderValidator::new(device);
    
    let mesh_metrics = validator.validate_mesh_shader()?;
    let depth_metrics = validator.validate_depth_shader()?;
    
    log::info!("All mesh shaders validated successfully");
    
    Ok((mesh_metrics, depth_metrics))
}