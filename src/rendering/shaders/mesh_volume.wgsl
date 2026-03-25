// Mesh Volume Rendering Shader
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

struct MeshUniforms {
    ray_step_size: f32,
    max_steps: f32,
    is_packed_rg8: f32,
    bias: f32,
    window: f32,
    level: f32,
    _pad0: vec2<f32>,
    pan_x: f32,
    pan_y: f32,
    scale: f32,
    opacity_multiplier: f32,
    light_dir: vec3<f32>,
    shading_strength: f32,
    rotation: mat4x4<f32>,
}
@group(1) @binding(0)
var<uniform> u_vol: MeshUniforms;

// Intersect axis-aligned unit box [0,1]^3
fn intersect_volume(ray_origin: vec3<f32>, ray_dir: vec3<f32>) -> vec2<f32> {
    let inv = 1.0 / ray_dir;
    let t0 = (vec3<f32>(0.0) - ray_origin) * inv;
    let t1 = (vec3<f32>(1.0) - ray_origin) * inv;

    let tmin = max(max(min(t0.x,t1.x), min(t0.y,t1.y)), min(t0.z,t1.z));
    let tmax = min(min(max(t0.x,t1.x), max(t0.y,t1.y)), max(t0.z,t1.z));

    return vec2<f32>(tmin, tmax);
}

fn sample_volume(coords: vec3<f32>) -> f32 {
    if (any(coords < vec3<f32>(0.0)) || any(coords > vec3<f32>(1.0))) {
        return -1024.0;
    }
    let sampled_value = textureSample(t_volume, s_volume, coords);
    var value: f32;
    if (u_vol.is_packed_rg8 > 0.5) {
        let low = sampled_value.r * 255.0;
        let high = sampled_value.g * 255.0;
        let u16_val = low + high * 256.0;
        value = u16_val - u_vol.bias;
    } else {
        value = sampled_value.r;
    }
    return value;
}

// DICOM-style window/level mapping (0..1)
fn apply_window_level(value: f32) -> f32 {
    let center = u_vol.level;
    let width = max(u_vol.window, 1e-6);
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

fn transfer_function(hu: f32) -> vec4<f32> {
    let norm = apply_window_level(hu);
    // air
    if (hu < -500.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    // soft tissue
    if (hu < 300.0) {
        return vec4<f32>(norm, norm * 0.7, norm * 0.6, norm * 0.1);
    }

    // bone
    let t = clamp((hu - 300.0) / 1000.0, 0.0, 1.0);
    return vec4<f32>(1.0, 1.0, 1.0, t * 0.6);
}

fn compute_gradient(p: vec3<f32>) -> vec3<f32> {
    let d = 0.002;

    let gx = sample_volume(p + vec3<f32>(d,0,0)) - sample_volume(p - vec3<f32>(d,0,0));
    let gy = sample_volume(p + vec3<f32>(0,d,0)) - sample_volume(p - vec3<f32>(0,d,0));
    let gz = sample_volume(p + vec3<f32>(0,0,d)) - sample_volume(p - vec3<f32>(0,0,d));

    return normalize(vec3<f32>(gx, gy, gz));
}

fn apply_lighting(color: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let L = normalize(u_vol.light_dir);
    let diff = max(dot(normal, L), 0.0);

    let shaded = color * (0.3 + diff * 0.7);

    return mix(color, shaded, u_vol.shading_strength);
}

// Ray march with front-to-back accumulation
fn volume_ray_march(ray_origin: vec3<f32>, ray_dir: vec3<f32>, t_start: f32, t_end: f32) -> vec4<f32> {
    var color = vec3<f32>(0.0);
    var alpha = 0.0;
    let step = u_vol.ray_step_size;

    for (var t = t_start; t < t_end; t += step) {
        let pos = ray_origin + t * ray_dir;
        let hu = sample_volume(pos);
        let tf = transfer_function(hu);
        var c = tf.rgb;
        var a = tf.a * u_vol.opacity_multiplier;

        // Gradient lighting
        if (a > 0.01) {
            let n = compute_gradient(pos);
            c = apply_lighting(c, n);
        }

        // front-to-back compositing
        color += (1.0 - alpha) * c * a;
        alpha += (1.0 - alpha) * a;
        if (alpha > 0.98) {
            break;
        }
    }
    return vec4<f32>(color, alpha);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let scale = max(u_vol.scale, 0.0001);
    let uv_centered = in.tex_coords - vec2<f32>(0.5, 0.5);
    let uv_scaled = uv_centered / scale;
    let uv = uv_scaled + vec2<f32>(0.5, 0.5) + vec2<f32>(u_vol.pan_x, u_vol.pan_y);
    let uv_clamped = clamp(uv, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));

    // Establish orthographic ray along +Z (texture coords space)
    let center = vec3<f32>(0.5, 0.5, 0.5);
    // Note the flip in y to match screen->texture coord mapping
    let base_ray_origin = vec3<f32>(uv_clamped.x, 1.0 - uv_clamped.y, -0.5);

    let volume_ray_origin = (u_vol.rotation * vec4<f32>(base_ray_origin - center, 1.0)).xyz + center;
    let volume_ray_dir = normalize((u_vol.rotation * vec4<f32>(0.0, 0.0, 1.0, 0.0)).xyz);

    let intersection = intersect_volume(volume_ray_origin, volume_ray_dir);
    let t_start = max(intersection.x, 0.0);
    let t_end = intersection.y;

    if (t_start >= t_end) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0); // transparent background
    }

    let result = volume_ray_march(volume_ray_origin, volume_ray_dir, t_start, t_end);
    return result;
}
