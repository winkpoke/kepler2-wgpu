/// Enhanced depth-only shader for shadow mapping and depth pre-pass
/// 
/// This shader renders geometry to a depth buffer for shadow mapping,
/// depth pre-pass optimization, and other depth-based techniques.

// Uniform buffer for transformation matrices
struct CameraUniforms {
    view_matrix: mat4x4<f32>,
    projection_matrix: mat4x4<f32>,
    view_projection_matrix: mat4x4<f32>,
    camera_position: vec3<f32>,
    _padding: f32,
};

// Uniform buffer for model transformation
struct ModelUniforms {
    model_matrix: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
};

// Bind group 0: Camera uniforms (light's perspective for shadow mapping)
@group(0) @binding(0) var<uniform> camera: CameraUniforms;

// Bind group 1: Model uniforms
@group(1) @binding(0) var<uniform> model: ModelUniforms;

// Vertex shader output structure for depth pass
struct VSOut {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
};

/// Enhanced depth vertex shader
/// Transforms vertices to light space for shadow mapping or camera space for depth pre-pass
@vertex
fn vs_main(
    @location(0) position: vec3<f32>
) -> VSOut {
    // Transform position to world space
    let world_position = (model.model_matrix * vec4<f32>(position, 1.0)).xyz;
    
    // Transform to clip space using the light's view-projection matrix
    let clip_position = camera.view_projection_matrix * vec4<f32>(world_position, 1.0);
    
    return VSOut(
        clip_position,
        world_position
    );
}

/// Enhanced depth fragment shader
/// Provides proper depth output for shadow mapping and depth testing
@fragment
fn fs_main(input: VSOut) -> @location(0) vec4<f32> {
    // For shadow mapping, we typically don't need to output color
    // The depth buffer is written automatically by the GPU
    // This can be used for debugging or special depth visualization
    
    // Calculate linear depth for better shadow map precision
    let linear_depth = length(input.world_position - camera.camera_position);
    
    // Normalize depth to [0, 1] range for visualization
    let normalized_depth = linear_depth / 100.0; // Adjust range as needed
    
    // Output depth as grayscale for debugging
    return vec4<f32>(normalized_depth, normalized_depth, normalized_depth, 1.0);
}