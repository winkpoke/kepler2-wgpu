// ============================================================
// Mesh Volume Rendering Shader
// Implements GPU-based volume ray casting using ray marching
// ============================================================

// ============================================================
// Vertex Output Structure
// Carries clip-space position and UV coordinates to fragment stage
// ============================================================
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

// ============================================================
// Vertex Shader
// Generates a full-screen quad procedurally (no vertex buffer)
// Each vertex corresponds to one corner of the screen
// ============================================================
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

// ============================================================
// Volume Texture & Sampler
// ============================================================
@group(0) @binding(0)
var t_volume: texture_3d<f32>;// 3D volume texture (CT/MRI data)
@group(0) @binding(1)
var s_volume: sampler;// Sampler for interpolation

// ============================================================
// Uniform Buffer
// Controls rendering, interaction, and lighting
// ============================================================
struct MeshUniforms {
    ray_step_size: f32,
    max_steps: f32,
    is_packed_rg8: f32,
    bias: f32,
    window: f32,
    level: f32,
    pan_x: f32,
    pan_y: f32,
    roi_min: vec3<f32>,
    scale: f32,
    roi_max: vec3<f32>,
    opacity_multiplier: f32,
    light_dir: vec3<f32>,
    shading_strength: f32,
    rotation: mat4x4<f32>,
}
@group(1) @binding(0)
var<uniform> u_vol: MeshUniforms;

// ============================================================
// Ray-box intersection (Axis-Aligned Bounding Box [0,1]^3)
// Computes entry and exit distances (tmin, tmax)
// ============================================================
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

// ============================================================
// Sample volume texture
// Supports both float and packed 16-bit (RG8) formats
// ============================================================
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

// ============================================================
// DICOM Window/Level Mapping
// Maps HU values to normalized intensity [0,1]
// ============================================================
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

// ============================================================
// Transfer Function
// Converts HU → color + opacity
// Models different tissue types (air, lung, fat, soft tissue, bone)
// ============================================================
fn transfer_function(hu: f32) -> vec4<f32> {
    let norm = apply_window_level(hu);
    let visibility = pow(norm, 1.5);

    // ========= 1. Air =========
    let air_alpha = 1.0 - smoothstep(-1000.0, -900.0, hu);
    let lung_t = smoothstep(-900.0, -300.0, hu);
    let lung_alpha = lung_t * 0.05;
    let lung_color = mix(
        vec3<f32>(0.15, 0.15, 0.15),
        vec3<f32>(0.25, 0.25, 0.25),
        lung_t
    );

    // ========= 2. Fat =========
    let fat_t = smoothstep(-300.0, -100.0, hu);
    let fat_alpha = fat_t * 0.15;
    let fat_color = mix(vec3<f32>(1.0, 0.9, 0.7),vec3<f32>(1.0, 0.8, 0.5),smoothstep(0.3, 0.7, norm)) * 0.8;

    // ========= 3. Soft tissue =========
    let soft_t = smoothstep(-100.0, 200.0, hu);
    let soft_alpha = soft_t * smoothstep(0.2, 0.8, norm) * 0.5;
    let soft_color = mix(vec3<f32>(0.9, 0.5, 0.4), vec3<f32>(1.0, 0.7, 0.6), soft_t)* (1.0 + 0.2 * norm);

    // ========= 4. Bone =========
    let bone_t = smoothstep(200.0, 1200.0, hu);
    let bone_alpha = clamp(bone_t * 1.2, 0.0, 1.0);
    let bone_color = mix(
        vec3<f32>(0.95, 0.93, 0.88),
        vec3<f32>(1.0, 1.0, 1.0),
        norm
    );

    let color = lung_color * lung_alpha
        + fat_color * fat_alpha
        + soft_color * soft_alpha
        + bone_color * bone_alpha
        + vec3<f32>(0.0) * air_alpha
    ;
    let final_color = pow(color, vec3<f32>(0.9));
    var alpha = (lung_alpha + fat_alpha + soft_alpha + bone_alpha) * visibility;;
    alpha = clamp(alpha, 0.0, 1.0);  
    return vec4<f32>(final_color, alpha);
}

// ============================================================
// Compute Gradient (Central Difference)
// Used to approximate surface normals for lighting
// ============================================================
fn compute_gradient(pos: vec3<f32>) -> vec3<f32> {
    let step = 1.0 / 512.0;
    let dx = sample_volume(pos + vec3<f32>(step, 0.0, 0.0)) -
             sample_volume(pos - vec3<f32>(step, 0.0, 0.0));
    let dy = sample_volume(pos + vec3<f32>(0.0, step, 0.0)) -
             sample_volume(pos - vec3<f32>(0.0, step, 0.0));
    let dz = sample_volume(pos + vec3<f32>(0.0, 0.0, step)) -
             sample_volume(pos - vec3<f32>(0.0, 0.0, step));
    return normalize(vec3<f32>(dx, dy, dz));
}

// ============================================================
// Phong Lighting Model
// Adds shading using normal, light direction, and view direction
// ============================================================
fn phong_lighting(normal: vec3<f32>, view_dir: vec3<f32>, base_color: vec3<f32>) -> vec3<f32> {
    let light_dir = normalize(vec3<f32>(0.5, 0.5, -1.0));
    let ambient = 0.2;

    // ===== diffuse =====
    let diff = max(dot(normal, light_dir), 0.0);

    // ===== specular =====
    let reflect_dir = reflect(-light_dir, normal);
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
    let color = base_color * (ambient + 0.8 * diff) + vec3<f32>(1.0) * spec * 0.3;
    return color;
}

// ============================================================
// Volume Ray Marching 连续采样
// Performs front-to-back compositing along the ray
// ============================================================
fn volume_ray_march(ray_origin: vec3<f32>, ray_dir: vec3<f32>, t_start: f32, t_end: f32) -> vec4<f32> {
    var color = vec3<f32>(0.0);
    var alpha = 0.0;
    let step = u_vol.ray_step_size;

    var t = t_start;
    var steps: u32 = 0u;
    loop{
        if (t > t_end || alpha > 0.98 || steps > u32(u_vol.max_steps)) {
            break;
        }
 
        // Ray Tracing 求交点
        let pos = ray_origin + t * ray_dir;

        // ROI
        if (any(pos < u_vol.roi_min) || any(pos > u_vol.roi_max)) {
            t += step;
            steps += 1u;
            continue;
        }

        let hu = sample_volume(pos);
        let tf = transfer_function(hu);
        var c = tf.rgb;

        // Convert TF alpha to physical density (Beer-Lambert law)
        let density_scale = 400.0;
        let density = tf.a * u_vol.opacity_multiplier * density_scale;

        // Compute alpha using exponential absorption model
        let a = 1.0 - exp(-density * step);

        // Apply shading if voxel is sufficiently opaque
        if (a > 0.05) {
            let grad = compute_gradient(pos); //“表面方向”（法线）
            let view_dir = normalize(-ray_dir);
            c = phong_lighting(grad, view_dir, c);
            let edge = clamp(length(grad) * 2.0, 0.0, 1.0);
            c *= mix(0.7, 1.3, edge);
        }
        c = pow(c, vec3<f32>(0.85)) * 1.2;

        // front-to-back compositing 前向累积
        color += (1.0 - alpha) * c * a;
        alpha += (1.0 - alpha) * a;

        t += step;
        steps += 1u;
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
