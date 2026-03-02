// MIP (Maximum/Minimum/Average Intensity Projection) Shader
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
    let x = f32(((vertex_index & 1u) * 2u)) - 1.0;
    let y = f32((vertex_index & 2u)) - 1.0;

    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.tex_coords = vec2<f32>((x + 1.0) * 0.5, (y + 1.0) * 0.5);
    
    return out;
}

// Fragment shader

@group(0) @binding(0)
var t_volume: texture_3d<f32>;
@group(0) @binding(1)
var s_volume: sampler;

struct MipUniforms {
    ray_step_size: f32,
    max_steps: f32,
    is_packed_rg8: f32,
    bias: f32,
    window: f32,
    level: f32,
    pan_x: f32,
    pan_y: f32,
    scale: f32,
    mode: f32,
    lower_threshold: f32,
    upper_threshold: f32,
    rotation: mat4x4<f32>,
}
@group(1) @binding(0)
var<uniform> u_mip: MipUniforms;

// Intersect axis-aligned unit box [0,1]^3
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

fn sample_volume(coords: vec3<f32>) -> f32 {
    if (any(coords < vec3<f32>(0.0)) || any(coords > vec3<f32>(1.0))) {
        // Return a low value (Air) instead of 0.0 (Water) to avoid artifacts at boundaries
        return -1024.0;
    }
    let sampled_value = textureSample(t_volume, s_volume, coords);
    var value: f32;
    if (u_mip.is_packed_rg8 > 0.5) {
        // decode RG8 -> u16 -> HU
        let low = sampled_value.r * 255.0;
        let high = sampled_value.g * 255.0;
        let u16_val = low + high * 256.0;
        value = u16_val - u_mip.bias;
    } else {
        value = sampled_value.r;
    }
    return value;
}

// DICOM-style window/level mapping (0..1)
fn apply_window_level(value: f32) -> f32 {
    let center = u_mip.level;
    let width = max(u_mip.window, 1e-6);
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

// Ray march with MIP / MinIP / AvgIP
fn mip_ray_march(ray_origin: vec3<f32>, ray_dir: vec3<f32>, t_start: f32, t_end: f32) -> f32 {
    var max_intensity = -1e20;
    var min_intensity = 1e20;
    var sum_intensity = 0.0;
    var count: u32 = 0u;

    let step_size = max(u_mip.ray_step_size, 1e-6);
    let max_steps = u32(max(u_mip.max_steps, 1.0));

    // estimate number of steps needed
    let length = max(0.0, t_end - t_start);
    let est_steps_f = floor(length / step_size) + 1.0;
    let est_steps_u = u32(clamp(est_steps_f, 0.0, f32(max_steps)));
    let loop_steps = min(max_steps, est_steps_u);

    for (var i = 0u; i < loop_steps; i = i + 1u) {
        let t = t_start + f32(i) * step_size;
        if (t > t_end) { break; }

        let sample_pos = ray_origin + t * ray_dir;
        let intensity = sample_volume(sample_pos);

        // threshold filtering (uniform-driven)
        if (intensity < u_mip.lower_threshold || intensity > u_mip.upper_threshold) {
            continue;
        }

        // choose aggregator by mode: mode ~ 0 => MIP, mode ~1 => MinIP, else AvgIP
        if (u_mip.mode < 0.5) {
            max_intensity = max(max_intensity, intensity);
        } else if (u_mip.mode < 1.5) {
            min_intensity = min(min_intensity, intensity);
        } else {
            sum_intensity = sum_intensity + intensity;
            count = count + 1u;
        }
    }

    // Fallback: if nothing sampled, return lower_threshold (so MinIP will invert to bright)
    if (u_mip.mode < 0.5) {
        if (max_intensity < -1e19) { return u_mip.lower_threshold; }
        return max_intensity;
    } else if (u_mip.mode < 1.5) {
        if (min_intensity > 1e19) { return u_mip.lower_threshold; }
        return min_intensity;
    } else {
        if (count == 0u) { return u_mip.lower_threshold; }
        return sum_intensity / f32(count);
    }
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // pan/scale centered at 0.5
    let scale = max(u_mip.scale, 0.0001);
    let uv_centered = in.tex_coords - vec2<f32>(0.5, 0.5);
    let uv_scaled = uv_centered / scale;
    let uv = uv_scaled + vec2<f32>(0.5, 0.5) + vec2<f32>(u_mip.pan_x, u_mip.pan_y);
    let uv_clamped = clamp(uv, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));

    // Establish orthographic ray along +Z (texture coords space)
    let center = vec3<f32>(0.5, 0.5, 0.5);
    // Note the flip in y to match screen->texture coord mapping
    let base_ray_origin = vec3<f32>(uv_clamped.x, 1.0 - uv_clamped.y, -0.5);

    let volume_ray_origin = (u_mip.rotation * vec4<f32>(base_ray_origin - center, 1.0)).xyz + center;
    let volume_ray_dir = normalize((u_mip.rotation * vec4<f32>(0.0, 0.0, 1.0, 0.0)).xyz);

    let intersection = intersect_volume(volume_ray_origin, volume_ray_dir);
    let t_start = max(intersection.x, 0.0);
    let t_end = intersection.y;

    if (t_start >= t_end) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let intensity = mip_ray_march(volume_ray_origin, volume_ray_dir, t_start, t_end);

    // Map intensity -> display value using window/level
    var processed = apply_window_level(intensity);

    // // For MinIP (mode == 1), invert the display mapping so low-HU -> bright.
    // if (u_mip.mode >= 1.0 && u_mip.mode < 2.0) {
    //     processed = 1.0 - processed;
    // }

    return vec4<f32>(processed, processed, processed, 1.0);
}