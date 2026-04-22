// ============================================================
// Mesh Volume Rendering Shader
// Implements GPU-based volume ray casting using ray marching
// ============================================================

// Vertex Output Structure
// Carries clip-space position and UV coordinates to fragment stage
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

// Vertex Shader
// Generates a full-screen quad procedurally (no vertex buffer)
// Each vertex corresponds to one corner of the screen
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

@group(0) @binding(0)
var t_volume: texture_3d<f32>;// 3D volume texture (CT/MRI data)
@group(0) @binding(1)
var s_volume: sampler;// Sampler for interpolation

// Uniform Buffer
// Controls rendering, interaction, and lighting
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
    aspect_ratio: f32,
    rotation: mat4x4<f32>,
    preset: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}
@group(1) @binding(0)
var<uniform> u_vol: MeshUniforms;

// Ray-box intersection (Axis-Aligned Bounding Box [0,1]^3)
// Computes entry and exit distances (tmin, tmax)
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

// MODE DEFINITIONS
fn get_iso_threshold() -> f32 {
    if (u_vol.preset < 0.5) {
        return 250.0; // ANGIO
    } else if (u_vol.preset < 1.5) {
        return 300.0; // BONE
    }
    return u_vol.level; // SOFT
}

// Phong Lighting Model
// Adds shading using normal, light direction, and view direction
fn compute_lighting(normal: vec3<f32>, view_dir: vec3<f32>, base_color: vec3<f32>) -> vec3<f32> {
    let light_dir = normalize(u_vol.light_dir);

    // diffuse
    let diff = max(dot(normal, light_dir), 0.0);

    // specular
    let reflect_dir = reflect(-light_dir, normal);
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
    return base_color * (0.45 + 1.0 * diff) + vec3<f32>(0.9) * spec * 0.2;
}

// ISO SURFACE
fn compute_iso_surface(
    pos: vec3<f32>,
    ray_dir: vec3<f32>,
    v0: f32,
    v1: f32,
    iso: f32
) -> vec4<f32> {
    if (!((v0 < iso && v1 >= iso) || (v0 >= iso && v1 < iso))) {
        return vec4<f32>(0.0);
    }

    // 精确求交点（线性插值）
    let t_iso = clamp((iso - v0) / (v1 - v0 + 1e-5), 0.0, 1.0);
    let iso_pos = pos + t_iso * ray_dir * u_vol.ray_step_size;

    // 法线（gradient）
    let normal = compute_gradient_smooth(iso_pos);
    let view_dir = normalize(-ray_dir);

    let light_dir = normalize(u_vol.light_dir);
    let diff = max(dot(normal, light_dir), 0.0);
    let shadow = compute_shadow(iso_pos, light_dir);

    // AO 凹陷增强
    let ao_base = clamp(dot(normal, vec3<f32>(0.0, 0.0, 1.0)) * 0.5 + 0.5, 0.0, 1.0);
    let ao = pow(1.0 - ao_base, 1.2);

    // Curvature 曲率
    let g1 = compute_gradient_smooth(iso_pos + normal * u_vol.ray_step_size);
    let g2 = compute_gradient_smooth(iso_pos - normal * u_vol.ray_step_size);
    let curv = clamp(length(g1 - g2) * 2.0, 0.0, 1.0);

    // 高光 (Specular)
    let reflect_dir = reflect(-light_dir, normal);
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 48.0);

    // 微弱“厚度感” (Thickness)
    let thickness = clamp(length(normal), 0.0, 1.0);

    // 边缘增强 (Edge Enhancement)
    let edge = clamp(1.0 - dot(normal, view_dir), 0.0, 1.0);

    // noise tone (微小噪点带来质感)
    let variation = fract(sin(dot(iso_pos.xy ,vec2<f32>(12.9898,78.233))) * 43758.5453);

    var color: vec3<f32>;
    if (u_vol.preset < 0.5) { // ANGIO
        // ANGIO style colors
        let base_light = vec3<f32>(1.0, 0.88, 0.72);
        let base_mid   = vec3<f32>(0.85, 0.55, 0.32);
        let base_dark  = vec3<f32>(0.45, 0.18, 0.08);

        var angio = mix(base_dark, base_mid, diff);
        angio = mix(angio, base_light, smoothstep(0.3, 0.9, diff));
        
        // Apply lighting factors
        angio *= mix(0.4, 1.0, shadow); // Deeper shadows
        angio *= mix(1.0, 0.5, ao);     // Stronger AO
        angio *= mix(0.9, 1.1, curv);
        angio += vec3<f32>(1.0, 0.9, 0.8) * spec * 0.15; // Reduce specular blowout
        angio *= mix(0.9, 1.05, thickness);
        angio *= mix(0.9, 1.1, edge);
        angio *= mix(0.95, 1.02, variation);
        
        color = clamp(angio, vec3<f32>(0.0), vec3<f32>(1.0));
    } else if (u_vol.preset < 1.5){ // BONE
        // BONE style colors (More natural bone tones)
        let base_light = vec3<f32>(0.85, 0.80, 0.70);
        let base_mid   = vec3<f32>(0.75, 0.65, 0.50);
        let base_dark  = vec3<f32>(0.45, 0.35, 0.25);

        // Apply lighting factors
        var bone = mix(base_dark, base_mid, diff);
        bone = mix(bone, base_light, smoothstep(0.2, 0.9, diff));
        bone *= mix(0.5, 1.0, shadow);
        bone *= mix(1.0, 0.6, ao);
        bone *= mix(0.9, 1.1, curv);
        bone += vec3<f32>(1.0, 0.95, 0.9) * spec * 0.1;
        bone *= mix(0.9, 1.05, thickness);
        bone *= mix(0.9, 1.1, edge);
        bone *= mix(0.95, 1.02, variation);
        
        color = clamp(bone, vec3<f32>(0.0), vec3<f32>(1.0));
    }
    return vec4<f32>(color, 0.5);
}

// Sample volume texture
// Supports both float and packed 16-bit (RG8) formats
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

// DICOM Window/Level Mapping
// Maps HU values to normalized intensity [0,1]
fn apply_window_level(value: f32) -> f32 {
    var center = u_vol.level;
    var width = u_vol.window;
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

// Transfer Function
// Converts HU → color + opacity
// Models different tissue types (air, lung, fat, soft tissue, bone)
fn transfer_function(hu: f32, grad_mag: f32) -> vec4<f32> {
    let edge = 0.3 + 0.7 * smoothstep(0.005, 0.05, grad_mag);
    let norm = apply_window_level(hu);

    // Air
    if (norm < 0.0 || norm > 1.0) { return vec4<f32>(0.0); }

    let soft_color = vec3<f32>(0.95, 0.65, 0.55);
    let fat_color = vec3<f32>(1.0, 0.85, 0.6);
    let lung_color = vec3<f32>(0.25, 0.25, 0.25);
    let muscle_color = vec3<f32>(0.75, 0.25, 0.25);
    let organ_color = vec3<f32>(0.9, 0.5, 0.3);

    let soft_alpha = (smoothstep(0.5, 1.0, norm) - smoothstep(1.0, 1.05, norm)) * edge;
    let fat_alpha  = (smoothstep(0.25, 0.52, norm) - smoothstep(0.52, 0.55, norm)) * edge;
    let lung_alpha = (smoothstep(0.4, 0.7, norm) - smoothstep(0.7, 0.8, norm)) * edge;
    let muscle_alpha = (smoothstep(0.51, 0.76, norm) - smoothstep(0.76, 0.8, norm)) * edge;
    let organ_alpha = (smoothstep(0.58, 0.70, norm) - smoothstep(0.70, 0.75, norm)) * edge;

    // ===== 主颜色 =====
    var color = vec3<f32>(0.0);
    var alpha = 0.0;
    color += lung_color * lung_alpha * (1.0 - alpha);
    alpha += lung_alpha * (1.0 - alpha);

    color += fat_color * fat_alpha * (1.0 - alpha);
    alpha += fat_alpha * (1.0 - alpha);

    color += soft_color * soft_alpha * (1.0 - alpha);
    alpha += soft_alpha * (1.0 - alpha);

    color += muscle_color * muscle_alpha * (1.0 - alpha);
    alpha += muscle_alpha * (1.0 - alpha);

    color += organ_color * organ_alpha * (1.0 - alpha);
    alpha += organ_alpha * (1.0 - alpha);

    return vec4<f32>(color, clamp(alpha, 0.0, 1.0));
}

// Compute Gradient (Central Difference)
// Used to approximate surface normals for lighting
fn compute_gradient_smooth(pos: vec3<f32>) -> vec3<f32> {
    let h: f32 = u_vol.ray_step_size * 0.5;
    let dx = sample_volume(pos + vec3<f32>(h,0,0)) - sample_volume(pos - vec3<f32>(h,0,0));
    let dy = sample_volume(pos + vec3<f32>(0,h,0)) - sample_volume(pos - vec3<f32>(0,h,0));
    let dz = sample_volume(pos + vec3<f32>(0,0,h)) - sample_volume(pos - vec3<f32>(0,0,h));
    return normalize(vec3<f32>(dx, dy, dz));
}

// ================== Shadow ==================
fn compute_shadow(pos: vec3<f32>, light_dir: vec3<f32>) -> f32 {
    var shadow: f32 = 1.0;
    var t: f32 = u_vol.ray_step_size * 2.0;
    let iso = u_vol.level;

    for (var i: i32 = 0; i < 8; i++) {
        let p = pos + light_dir * t;
        let v = sample_volume(p);
        if (v > iso) {
            let atten = exp(-t * 2.0);
            shadow *= mix(0.55, 1.0, atten);
        }
        t += u_vol.ray_step_size * 2.5;
    }
    return clamp(shadow, 0.9, 1.0);
}

// Volume Ray Marching 连续采样
// Performs front-to-back compositing along the ray
fn volume_ray_march(
    ray_origin: vec3<f32>, 
    ray_dir: vec3<f32>, 
    t_start: f32, 
    t_end: f32
) -> vec4<f32> {
    var color = vec3<f32>(0.0);
    var alpha = 0.0;
    let step = u_vol.ray_step_size;
    let iso = get_iso_threshold();

    var t = t_start;
    var steps: u32 = 0u;
    loop{
        // Early Termination
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

        let v0 = sample_volume(pos);
        let v1 = sample_volume(pos + ray_dir * step);
        if (v0 < -950.0 && v1 < -950.0) {
            t += step * 3.0;
            steps += 1u;
            continue;
        }

        let iso_hit = compute_iso_surface(pos, ray_dir, v0, v1, iso);
        let tf = transfer_function(v0, 0.0);
        let tf1 = transfer_function(v1, 0.0);

        let avg_color = (tf.rgb + tf1.rgb) * 0.5;
        let avg_alpha = (tf.a + tf1.a) * 0.5;
        var density = avg_alpha * u_vol.opacity_multiplier * 120.0;
        // Empty space skipping
        if (density < (0.002 * (1.0 + u_vol.opacity_multiplier))) {
            t += step * 3.0;
            continue;
        }

        let a = 1.0 - exp(-density * step);
        let iso_weight = iso_hit.a * 2.5;

        if (a > 0.001 || iso_weight > 0.001) {
            let grad = compute_gradient_smooth(pos);
            let lit = compute_lighting(grad, -ray_dir, avg_color);
            let iso_color = iso_hit.rgb * iso_weight;
            let final_color = lit * (1.0 - iso_weight) + iso_color;

            color += (1.0 - alpha) * final_color * max(a, iso_weight);
            alpha += (1.0 - alpha) * max(a, iso_weight);
        }

        t += step;
        steps += 1u;
    }

    return vec4<f32>(color, alpha);
}

// DVR（软组织）
fn compute_dvr(
    pos: vec3<f32>,
    ray_dir: vec3<f32>,
    v0: f32,
    v1: f32
) -> vec4<f32> {
    let grad = compute_gradient_smooth(pos);
    let grad_mag = length(grad);
    let tf0 = transfer_function(v0, grad_mag);
    let tf1 = transfer_function(v1, grad_mag);

    // 平均密度
    let avg_alpha = (tf0.a + tf1.a) * 0.5;
    let density = avg_alpha * u_vol.opacity_multiplier * 120.0;

    // 指数积分（连续介质）
    let a = 1.0 - exp(-density * u_vol.ray_step_size);
    if (a < 0.01) {
        return vec4<f32>(0.0);
    }
    let view_dir = normalize(-ray_dir);
    let avg_color = (tf0.rgb + tf1.rgb) * 0.5;
    let color = compute_lighting(grad, view_dir, avg_color);
    return vec4<f32>(color, a);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let scale = max(u_vol.scale, 0.0001);
    var uv_centered = in.tex_coords - vec2<f32>(0.5, 0.5);
    
    // Apply aspect ratio compensation
    // If width > height (aspect_ratio > 1.0), stretch x to avoid horizontal squashing
    if (u_vol.aspect_ratio > 1.0) {
        uv_centered.x *= u_vol.aspect_ratio;
    } else if (u_vol.aspect_ratio < 1.0 && u_vol.aspect_ratio > 0.0) {
        // If height > width (aspect_ratio < 1.0), stretch y to avoid vertical squashing
        uv_centered.y /= u_vol.aspect_ratio;
    }
    
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