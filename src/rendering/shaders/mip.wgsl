// MIP (Maximum Intensity Projection) Shader
// Implements basic ray casting for volume rendering with RenderContent compatibility

// Vertex shader for fullscreen quad rendering
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    
    // Generate fullscreen quad vertices
    // Triangle strip: (-1,-1), (1,-1), (-1,1), (1,1)
    let x = f32((vertex_index & 1u) * 2u) - 1.0;
    let y = f32((vertex_index & 2u)) - 1.0;
    
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.tex_coords = vec2<f32>((x + 1.0) * 0.5, (y + 1.0) * 0.5);
    
    return out;
}

// Fragment shader for MIP ray casting

// Texture and sampler bindings (compatible with RenderContent)
@group(0) @binding(0)
var t_volume: texture_3d<f32>;
@group(0) @binding(1)
var s_volume: sampler;

// MIP uniforms
struct MipUniforms {
    // Camera parameters
    camera_pos: vec3<f32>,
    _padding1: f32,
    camera_front: vec3<f32>,
    _padding2: f32,
    camera_up: vec3<f32>,
    _padding3: f32,
    camera_right: vec3<f32>,
    _padding4: f32,
    
    // Volume parameters
    volume_size: vec3<f32>,
    _padding5: f32,
    
    // Ray marching parameters
    ray_step_size: f32,
    max_steps: f32,
    
    // Texture format parameters (reused from existing logic)
    is_packed_rg8: f32,
    _padding6: f32,
    
    // Window/Level for medical imaging
    window: f32,
    level: f32,
    
    // View matrix for coordinate transformation
    view_matrix: mat4x4<f32>,
    
    // Padding to ensure 16-byte alignment for uniform buffer requirements
    _padding_end: vec4<f32>,
}

@group(1) @binding(0)
var<uniform> u_mip: MipUniforms;

// Volume intersection function
// Returns entry and exit points for ray-volume intersection
fn intersect_volume(ray_origin: vec3<f32>, ray_dir: vec3<f32>) -> vec2<f32> {
    // Volume bounds: [0, 1] in all dimensions
    let volume_min = vec3<f32>(0.0, 0.0, 0.0);
    let volume_max = vec3<f32>(1.0, 1.0, 1.0);
    
    // Calculate intersection with volume AABB
    let inv_dir = 1.0 / ray_dir;
    let t_min = (volume_min - ray_origin) * inv_dir;
    let t_max = (volume_max - ray_origin) * inv_dir;
    
    let t1 = min(t_min, t_max);
    let t2 = max(t_min, t_max);
    
    let t_near = max(max(t1.x, t1.y), t1.z);
    let t_far = min(min(t2.x, t2.y), t2.z);
    
    // Ensure we start from the front of the volume
    let t_start = max(t_near, 0.0);
    let t_end = t_far;
    
    return vec2<f32>(t_start, t_end);
}

// Texture sampling function (reused from existing shader_tex.wgsl logic)
fn sample_volume(coords: vec3<f32>) -> f32 {
    // Check bounds
    if (any(coords < vec3<f32>(0.0)) || any(coords > vec3<f32>(1.0))) {
        return 0.0;
    }
    
    // Sample the texture
    let sampled_value = textureSample(t_volume, s_volume, coords);
    
    // Decode based on texture format (reused logic from shader_tex.wgsl)
    var value: f32;
    if (u_mip.is_packed_rg8 > 0.5) {
        // Packed RG8 path: decode to scalar value
        value = (sampled_value.g * 256.0 + sampled_value.r) * 255.0;
    } else {
        // Native float path (R16Float/R32Float): use the red channel
        value = sampled_value.r;
    }
    
    return value;
}

// Apply window/level transformation for display
fn apply_window_level(value: f32) -> f32 {
    // Standard window/level transformation
    let windowed = (value - (u_mip.level - u_mip.window / 2.0)) / u_mip.window;
    let clamped = clamp(windowed, 0.0, 1.0);
    
    // Simple gamma correction without aggressive contrast enhancement
    let gamma = 0.9;
    return pow(clamped, gamma);
}

// MIP ray marching function
fn mip_ray_march(ray_origin: vec3<f32>, ray_dir: vec3<f32>) -> f32 {
    // Get intersection points with volume
    let intersection = intersect_volume(ray_origin, ray_dir);
    let t_start = intersection.x;
    let t_end = intersection.y;
    
    // Early exit if no intersection
    if (t_start >= t_end) {
        return 0.0;
    }
    
    // Initialize maximum intensity
    var max_intensity = 0.0;
    
    // Ray marching loop
    let step_size = u_mip.ray_step_size;
    let max_steps = u32(u_mip.max_steps);
    
    for (var i = 0u; i < max_steps; i = i + 1u) {
        let t = t_start + f32(i) * step_size;
        
        // Exit if we've gone past the volume
        if (t > t_end) {
            break;
        }
        
        // Calculate sample position
        let sample_pos = ray_origin + t * ray_dir;
        
        // Sample volume at current position
        let intensity = sample_volume(sample_pos);
        
        // Update maximum intensity
        max_intensity = max(max_intensity, intensity);
    }
    
    return max_intensity;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Convert texture coordinates to normalized device coordinates [-1, 1]
    let ndc = vec2<f32>(in.tex_coords.x * 2.0 - 1.0, 1.0 - in.tex_coords.y * 2.0);
    
    // For orthographic projection, all rays have the same direction
    // Point rays into the volume (positive Z direction)
    let ray_dir = vec3<f32>(0.0, 0.0, 1.0);
    
    // Fixed ray generation for orthographic MIP
    // Ray origin varies across screen (0 to 1 for X and Y), starts at Z = 0 (front face of volume)
    let ray_origin = vec3<f32>(ndc.x * 0.5 + 0.5, ndc.y * 0.5 + 0.5, 0.0);
    
    // Use ray origin and direction directly in volume space
    let volume_ray_origin = ray_origin;
    let volume_ray_dir = ray_dir;
    
    // Perform MIP ray marching
    let max_intensity = mip_ray_march(volume_ray_origin, volume_ray_dir);
    
    // Apply window/level and return final color
    if (max_intensity > 0.0) {
        let processed = apply_window_level(max_intensity);
        return vec4<f32>(processed, processed, processed, 1.0);
    } else {
        // No volume data found - return black
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
}