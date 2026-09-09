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
    needle_enabled: f32,
    needle_count : u32,
    _pad: f32,
    ball_enabled: f32,
    balls: array<vec4<f32>, 4>,
    needles : array<NeedleUniform, 32>,
    rotation: mat4x4<f32>,
}
@group(1) @binding(0)
var<uniform> u_mip: MipUniforms;

// Intersect axis-aligned unit box [0,1]^3
fn intersect_volume(ray_origin: vec3<f32>, ray_dir: vec3<f32>) -> vec2<f32> {
    let is_zero = abs(ray_dir) < vec3<f32>(1e-6);
    let sign_dir = select(sign(ray_dir), vec3<f32>(1.0), is_zero);
    let inv = 1.0 / max(abs(ray_dir), vec3<f32>(1e-6)) * sign_dir;
    let t0 = (vec3<f32>(0.0) - ray_origin) * inv;
    let t1 = (vec3<f32>(1.0) - ray_origin) * inv;

    let tmin = max(max(min(t0.x,t1.x), min(t0.y,t1.y)), min(t0.z,t1.z));
    let tmax = min(min(max(t0.x,t1.x), max(t0.y,t1.y)), max(t0.z,t1.z));

    return vec2<f32>(tmin, tmax);
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
    let min_val = center - 0.5 - (width - 1.0) * 0.5;
    let max_val = center - 0.5 + (width - 1.0) * 0.5;
    let v = (value - min_val) / (max_val - min_val);
    return clamp(v, 0.0, 1.0);
}

// Ray march with MIP / MinIP / AvgIP, with optional needle overlay
fn mip_ray_march(ray_origin: vec3<f32>, ray_dir: vec3<f32>, t_start: f32, t_end: f32) -> vec4<f32> {
    var max_intensity = -1e20;
    var min_intensity = 1e20;
    var sum_intensity = 0.0;
    var count: u32 = 0u;
    var needle_color = vec3<f32>(0.0);

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
        var intensity = sample_volume(sample_pos);

        // threshold filtering (uniform-driven)
        if (intensity < u_mip.lower_threshold || intensity > u_mip.upper_threshold) {
            continue;
        }

        // check for needle overlay
        if (u_mip.needle_enabled > 0.5) {
            for (var k: u32 = 0u; k < u_mip.needle_count; k = k + 1u) {
                let needle = u_mip.needles[k];
                if (point_inside_needle(sample_pos, needle.entry, needle.tip, needle.radius)) {
                    needle_color = needle.color.rgb;
                    break;
                }
            }
        }

        if (u_mip.ball_enabled > 0.5) {
            for (var b: u32 = 0u; b < 4u; b = b + 1u) {
                let ball = u_mip.balls[b];
                if (ball.w <= 0.0) { 
                    continue; 
                }
                let center = ball.xyz;
                let radius = ball.w;
                let to_center = sample_pos - center;
                if (dot(to_center, to_center) <= radius * radius) {
                    needle_color = vec3<f32>(1.0, 0.31, 0.0);
                    break;
                }
            }
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
    var final_intensity: f32;
    if (u_mip.mode < 0.5) {
        if (max_intensity < -1e19) { final_intensity = u_mip.lower_threshold; }
        final_intensity = max_intensity;
    } else if (u_mip.mode < 1.5) {
        if (min_intensity > 1e19) { final_intensity = u_mip.lower_threshold; }
        final_intensity = min_intensity;
    } else {
        if (count == 0u) { final_intensity = u_mip.lower_threshold; }
        final_intensity = sum_intensity / f32(count);
    }
    return vec4<f32>(final_intensity, needle_color);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // pan/scale centered at 0.5
    let scale = max(u_mip.scale * 1.5, 0.0001);
    let uv = (in.tex_coords - vec2<f32>(0.5)) * scale + vec2<f32>(0.5) + vec2<f32>(u_mip.pan_x, u_mip.pan_y);

    // Establish orthographic ray along +Z (texture coords space)
    let center = vec3<f32>(0.5, 0.5, 0.5);
    // Note the flip in y to match screen->texture coord mapping
    let base_ray_origin = vec3<f32>(uv.x, 1.0 - uv.y, -0.5);

    let volume_ray_origin = (u_mip.rotation * vec4<f32>(base_ray_origin - center, 1.0)).xyz + center;
    let volume_ray_dir = normalize((u_mip.rotation * vec4<f32>(0.0, 0.0, 1.0, 0.0)).xyz);

    let intersection = intersect_volume(volume_ray_origin, volume_ray_dir);
    let t_start = intersection.x;
    let t_end = intersection.y;

    if (t_start >= t_end) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let mip_result = mip_ray_march(volume_ray_origin, volume_ray_dir, t_start, t_end);
    let intensity = mip_result.r;
    let needle_color = mip_result.gba;

    // Needle overlay: if any needle was hit during raymarching, show needle color
    if (needle_color.r + needle_color.g + needle_color.b > 0.001) {
        return vec4<f32>(needle_color, 0.4);
    }

    // Map intensity -> display value using window/level
    let processed = apply_window_level(intensity);

    return vec4<f32>(processed, processed, processed, 1.0);
}