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
    aspect_ratio: f32,
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

    // ========= Air =========
    if (norm < 0.0 || norm > 1.0) { return vec4<f32>(0.0); }

    // ========== Soft tissue ==========
    let soft_alpha = smoothstep(0.5, 1.0, norm) - smoothstep(1.0, 1.05, norm);
    let soft_color = vec3<f32>(0.95, 0.65, 0.55);

    // ========== Fat ==========
    let fat_alpha  = smoothstep(0.25, 0.52, norm) - smoothstep(0.52, 0.55, norm);
    let fat_color = vec3<f32>(1.0, 0.85, 0.6);

    // ========== Lung ==========
    let lung_alpha = smoothstep(0.4, 0.7, norm) - smoothstep(0.7, 0.8, norm);
    let lung_color = vec3<f32>(0.25, 0.25, 0.25);

    // ========== Muscle ==========
    let muscle_alpha = smoothstep(0.51, 0.76, norm) - smoothstep(0.76, 0.8, norm);
    let muscle_color = vec3<f32>(0.75, 0.25, 0.25);

    // ========== Organ ==========
    let organ_alpha = smoothstep(0.5, 0.75, norm) - smoothstep(0.75, 0.8, norm);
    let organ_color = vec3<f32>(0.9, 0.5, 0.3);

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

// ============================================================
// Compute Gradient (Central Difference)
// Used to approximate surface normals for lighting
// ============================================================
fn compute_gradient(pos: vec3<f32>) -> vec3<f32> {
    let step = u_vol.ray_step_size * 1.5;
    let dx = sample_volume(pos + vec3<f32>(step, 0.0, 0.0)) -
             sample_volume(pos - vec3<f32>(step, 0.0, 0.0));
    let dy = sample_volume(pos + vec3<f32>(0.0, step, 0.0)) -
             sample_volume(pos - vec3<f32>(0.0, step, 0.0));
    let dz = sample_volume(pos + vec3<f32>(0.0, 0.0, step)) -
             sample_volume(pos - vec3<f32>(0.0, 0.0, step));
    return normalize(vec3<f32>(dx, dy, dz)+ vec3<f32>(1e-3));
}

// ============================================================
// Phong Lighting Model
// Adds shading using normal, light direction, and view direction
// ============================================================
fn phong_lighting(normal: vec3<f32>, view_dir: vec3<f32>, base_color: vec3<f32>) -> vec3<f32> {
    let light_dir = normalize(vec3<f32>(0.5, 0.5, -1.0));
    let ambient = 0.35;

    // ===== diffuse =====
    let diff = max(dot(normal, light_dir), 0.0);

    // ===== specular =====
    let reflect_dir = reflect(-light_dir, normal);
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
    let color = base_color * (ambient + 1.1 * diff) + vec3<f32>(1.0) * spec * 0.25;
    return color;
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
            shadow *= mix(0.85, 1.0, atten);
        }
        t += u_vol.ray_step_size * 2.5;
    }
    return clamp(shadow, 0.9, 1.0);
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

        let v0 = sample_volume(pos);
        let v1 = sample_volume(pos + ray_dir * step);
        if (v0 < -950.0 && v1 < -950.0) {
            t += step * 3.0;
            steps += 1u;
            continue;
        }

        // TF
        let tf = transfer_function(v0);
        let grad = compute_gradient(pos);
        let grad_mag = length(grad);

        // ===== 边界增强（壳的来源）=====
        let edge = smoothstep(0.02, 0.12, grad_mag);

        // ===== 内部抑制（防止一坨）=====
        let interior = smoothstep(0.0, 0.04, grad_mag);

        // ===== 密度 =====
        let density = tf.a * u_vol.opacity_multiplier * 120.0;
        let a = (1.0 - exp(-density * step)) * edge * interior;

        if (a > 0.01) {
            let grad = compute_gradient(pos);
            let view_dir = normalize(-ray_dir);

            var c = phong_lighting(grad, view_dir, tf.rgb);
            let rim = pow(1.0 - abs(dot(view_dir, grad)), 2.5);
            c += tf.rgb * rim * 0.3;

            let shadow = compute_shadow(pos, normalize(u_vol.light_dir));
            c *= mix(0.75, 1.0, shadow);

            color += (1.0 - alpha) * c * a;
            alpha += (1.0 - alpha) * a;
        }

        // ISO（骨，优先级最高）
        let current_iso = u_vol.level;
        if ((v0 < current_iso && v1 >= current_iso) || (v0 >= current_iso && v1 < current_iso)) {
            // 精确求交点（线性插值）
            let t_iso = clamp((current_iso - v0) / (v1 - v0 + 1e-5), 0.0, 1.0);
            let iso_pos = pos + t_iso * ray_dir * step;

            // 法线（gradient）
            let normal = compute_gradient(iso_pos);
            let view_dir = normalize(-ray_dir);
            let light_dir = normalize(u_vol.light_dir);
            let diff = max(dot(normal, light_dir), 0.0);

            // 阴影 
            let shadow = compute_shadow(iso_pos, light_dir);

            // AO（凹陷增强，关键）
            let ao_base = clamp(dot(normal, vec3<f32>(0.0, 0.0, 1.0)) * 0.5 + 0.5, 0.0, 1.0);
            let ao = pow(1.0 - ao_base, 1.5);

            // Curvature 曲率
            let g1 = compute_gradient(iso_pos + normal * u_vol.ray_step_size);
            let g2 = compute_gradient(iso_pos - normal * u_vol.ray_step_size);
            let curvature = length(g1 - g2);
            let curv = clamp(curvature * 2.5, 0.0, 1.0);

            // ===== 骨骼颜色 =====
            let base_light = vec3<f32>(1.0, 0.98, 0.9);   // 亮部（偏白）
            let base_mid   = vec3<f32>(0.95, 0.88, 0.7);  // 主体（米黄）
            let base_dark  = vec3<f32>(0.7, 0.6, 0.45);   // 暗部（偏棕）

            // ===== 根据光照插值 =====
            var bone = mix(base_dark, base_mid, diff);
            bone = mix(bone, base_light, pow(diff, 2.0));
            bone *= shadow;
            bone *= mix(1.0, 0.85, ao);
            bone *= mix(0.85, 1.25, curv);

            // ===== 高光=====
            let reflect_dir = reflect(-light_dir, normal);
            let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 24.0);
            bone += vec3<f32>(1.0, 0.98, 0.95) * spec * 0.15;

            // ===== 微弱“厚度感”=====
            let thickness = clamp(length(normal), 0.0, 1.0);
            bone *= mix(0.92, 1.08, thickness);

            // ===== 边缘增强 =====
            let edge = clamp(length(normal) * 1.5, 0.0, 1.0);
            bone *= mix(0.9, 1.1, edge);

            // Tone（医疗质感关键)
            let variation = fract(sin(dot(iso_pos.xy ,vec2<f32>(12.9898,78.233))) * 43758.5453);
            bone *= mix(0.97, 1.03, variation);
            bone = pow(bone, vec3<f32>(0.85));

            let iso_alpha = 0.85;   // 可调（0.7~1.0）

            color += (1.0 - alpha) * bone * iso_alpha;
            alpha += (1.0 - alpha) * iso_alpha;

            t += step * 0.5;
            steps += 1u;
            continue;
        }

        t += step;
        steps += 1u;
    }

    return vec4<f32>(color, alpha);
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
