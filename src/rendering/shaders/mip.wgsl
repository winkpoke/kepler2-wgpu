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

// MIP uniforms - only fields actually used in the shader
struct MipUniforms {
    // Ray marching parameters
    ray_step_size: f32,
    max_steps: f32,
    
    // Texture format parameters (reused from existing logic)
    is_packed_rg8: f32,
    // Packed RG8 bias to recover raw HU
    bias: f32,
    
    // Window/Level for medical imaging
    window: f32,
    level: f32,
    pan_x: f32,
    pan_y: f32,
    scale: f32,
    // Padding to satisfy 16-byte multiple size requirements on WebGL
    _pad0: vec3<f32>,
    rotation: mat4x4<f32>,
}

@group(1) @binding(0)
var<uniform> u_mip: MipUniforms;

// Volume intersection function for orthographic rays along +Z
fn intersect_volume(ray_origin: vec3<f32>, ray_dir: vec3<f32>) -> vec2<f32> {
    let box_min = vec3<f32>(0.0, 0.0, 0.0);
    let box_max = vec3<f32>(1.0, 1.0, 1.0);

    let eps = 1e-6;
    let inv_dir = select(vec3<f32>(1e20, 1e20, 1e20), 1.0 / ray_dir, abs(ray_dir) > vec3<f32>(eps, eps, eps));

    let t0 = (box_min - ray_origin) * inv_dir;
    let t1 = (box_max - ray_origin) * inv_dir;

    let tmin3 = min(t0, t1);
    let tmax3 = max(t0, t1);

    let t_min = max(max(tmin3.x, tmin3.y), tmin3.z);
    let t_max = min(min(tmax3.x, tmax3.y), tmax3.z);

    return vec2<f32>(t_min, t_max);
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
        // Packed RG8 path: decode little-endian u16 and convert back to HU
        let low = sampled_value.r * 255.0;
        let high = sampled_value.g * 255.0;
        let u16_val = low + high * 256.0;
        value = u16_val - u_mip.bias;
    } else {
        // Native float path (R16Float/R32Float): use the red channel
        value = sampled_value.r;
    }
    
    return value;
}

// Apply window/level transformation for display
fn apply_window_level(value: f32) -> f32 {
    // DICOM PS3.3 C.11.2 Window/Level mapping (consistent with MPR)
    let center = u_mip.level;
    let width = u_mip.window;
    var v: f32;
    if (value <= (center - 0.5 - (width - 1.0) / 2.0)) {
        v = 0.0;
    } else if (value > (center - 0.5 + (width - 1.0) / 2.0)) {
        v = 1.0;
    } else {
        v = ((value - (center - 0.5)) / (width - 1.0)) + 0.5;
    }
    return clamp(v, 0.0, 1.0);
}

// MIP ray marching function
fn mip_ray_march(ray_origin: vec3<f32>, ray_dir: vec3<f32>) -> f32 {
    // Get intersection points with volume
    let intersection = intersect_volume(ray_origin, ray_dir);
    let t_start = max(intersection.x, 0.0);
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
    // Apply pan and scale in screen space (centered at 0.5,0.5)
    let scale = max(u_mip.scale, 0.0001);
    let uv_centered = in.tex_coords - vec2<f32>(0.5, 0.5);
    let uv_scaled = uv_centered / scale;
    let uv = uv_scaled + vec2<f32>(0.5, 0.5) + vec2<f32>(u_mip.pan_x, u_mip.pan_y);
    let uv_clamped = clamp(uv, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));

    let center = vec3<f32>(0.5, 0.5, 0.5);
    let base_ray_origin = vec3<f32>(uv_clamped.x, 1.0 - uv_clamped.y, - 0.5);

    let volume_ray_origin = (u_mip.rotation * vec4<f32>(base_ray_origin - center, 1.0)).xyz + center;
    let volume_ray_dir = normalize((u_mip.rotation * vec4<f32>(0.0, 0.0, 1.0, 0.0)).xyz);
    
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
