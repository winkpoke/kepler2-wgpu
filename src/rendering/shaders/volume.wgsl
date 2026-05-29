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
    pan_x: f32,
    pan_y: f32,
    roi_min: vec3<f32>,
    scale: f32,
    roi_max: vec3<f32>,
    opacity_multiplier: f32,
    light_dir: vec3<f32>,
    aspect_ratio: f32,
    rotation: mat4x4<f32>,
    vol_dims: vec3<f32>,
    preset: f32,
    needle_entry: vec3<f32>,    // 针入口点 (归一化坐标 0-1)
    needle_enabled: f32,        // 是否显示针 (0.0 或 1.0)
    needle_target: vec3<f32>,   // 针目标点 (归一化坐标 0-1)
    needle_radius: f32,         // 针半径 (归一化)
    needle_pos: vec3<f32>,      // 针当前位置 (归一化坐标 0-1)
    needle_length: f32,         // 针总长度 (归一化)
}
@group(1) @binding(0)
var<uniform> u_vol: MeshUniforms;

// Ray-box intersection (Axis-Aligned Bounding Box [0,1]^3)
// Computes entry and exit distances (tmin, tmax)
fn intersect_box(ray_origin: vec3<f32>, ray_dir: vec3<f32>, box_min: vec3<f32>, box_max: vec3<f32>) -> vec2<f32> {
    let is_zero = abs(ray_dir) < vec3<f32>(1e-6);
    let sign_dir = select(sign(ray_dir), vec3<f32>(1.0), is_zero);
    let inv = 1.0 / max(abs(ray_dir), vec3<f32>(1e-6)) * sign_dir;

    let t0 = (box_min - ray_origin) * inv;
    let t1 = (box_max - ray_origin) * inv;

    let tmin = max(max(min(t0.x,t1.x), min(t0.y,t1.y)), min(t0.z,t1.z));
    let tmax = min(min(max(t0.x,t1.x), max(t0.y,t1.y)), max(t0.z,t1.z));

    return vec2<f32>(tmin, tmax);
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
    let center = u_vol.level;
    let width = max(u_vol.window, 1e-6);
    let min_val = center - 0.5 - (width - 1.0) * 0.5;
    let max_val = center - 0.5 + (width - 1.0) * 0.5;
    let v = (value - min_val) / max(max_val - min_val, 1e-6);
    return clamp(v, 0.0, 1.0);
}

fn hash(uv: vec2<f32>) -> f32 {
    return fract(sin(dot(uv, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn get_iso_threshold() -> f32 {
    if (u_vol.preset < 0.5) {
        return 300.0; // BONE
    }
    return u_vol.level; // SOFT
}

// Compute Gradient
// Used to approximate surface normals for lighting
fn compute_normal(pos: vec3<f32>) -> vec3<f32> {
    let h = 1.0 / u_vol.vol_dims;
    let dx = sample_volume(pos + vec3(h.x,0,0)) - sample_volume(pos - vec3(h.x,0,0));
    let dy = sample_volume(pos + vec3(0,h.y,0)) - sample_volume(pos - vec3(0,h.y,0));
    let dz = sample_volume(pos + vec3(0,0,h.z)) - sample_volume(pos - vec3(0,0,h.z));
    let g = vec3<f32>(dx, dy, dz);
    return normalize(select(vec3<f32>(0.0,0.0,1.0), g, length(g) > 1e-6));
}

// Multi-Light Phong Lighting Model
// Uses 3 lights positioned around the VIEW direction for uniform illumination.
// The view direction changes with camera rotation, so lighting stays balanced.
//   Light 0 (Key):   slightly right of camera, main illumination
//   Light 1 (Fill):  left and behind camera, fills shadows
//   Light 2 (Rim):   from behind, highlights silhouette edges
fn compute_lighting(normal_in: vec3<f32>, view_dir_in: vec3<f32>, base_color: vec3<f32>) -> vec3<f32> {
    let n = normalize(normal_in);
    let v = normalize(view_dir_in);

    // Light 0: Key light (slightly right of camera)
    let l0 = normalize(v + vec3<f32>(0.3, 0.1, -0.1));
    let diff0 = max(dot(n, l0), 0.0);
    let h0 = normalize(l0 + v);
    let spec0 = pow(max(dot(n, h0), 0.0), 16.0);
    let color0 = vec3<f32>(1.0, 0.99, 0.96);
    let kd0 = 0.9;
    let ks0 = 0.3;

    // Light 1: Fill light (left and behind camera, warm)
    let l1 = normalize(-v + vec3<f32>(-0.3, 0.2, 0.1));
    let diff1 = max(dot(n, l1), 0.0);
    let h1 = normalize(l1 + v);
    let spec1 = pow(max(dot(n, h1), 0.0), 4.0);
    let color1 = vec3<f32>(1.0, 0.9, 0.78);
    let kd1 = 0.6;
    let ks1 = 0.12;

    // Light 2: Rim light (from behind, edge highlight)
    let l2 = normalize(v + vec3<f32>(0.0, -0.4, 0.5));
    let diff2 = max(dot(n, l2), 0.0);
    let h2 = normalize(l2 + v);
    let spec2 = pow(max(dot(n, h2), 0.0), 24.0);
    let color2 = vec3<f32>(0.88, 0.92, 1.0);
    let kd2 = 0.35;
    let ks2 = 0.2;

    let ambient = 0.5;
    let diffuse = color0 * kd0 * diff0 + color1 * kd1 * diff1 + color2 * kd2 * diff2;
    let specular = color0 * ks0 * spec0 + color1 * ks1 * spec1 + color2 * ks2 * spec2;

    return base_color * (ambient + diffuse) + specular;
}

fn bone_base_color(hu: f32) -> vec3<f32> {
    let t = clamp((hu - 200.0) / 1200.0, 0.0, 1.0);
    let dark = vec3<f32>(0.85, 0.85, 0.85);
    let light = vec3<f32>(1.0, 1.0, 1.0);
    return mix(dark, light, t);
}

fn transfer_function(hu: f32, grad_mag: f32) -> vec4<f32> {
    let norm = apply_window_level(hu);
    
    // 如果值低于窗底，完全不可见
    if (norm <= 0.0) {
        return vec4<f32>(0.0);
    }
    
    // 基础颜色设定：从深红棕色（低值）过渡到浅黄色/白色（高值）
    let color_low = vec3<f32>(0.18, 0.14, 0.12);   // 深灰褐色（空气/低密度组织过渡）
    let color_mid = vec3<f32>(0.72, 0.55, 0.44);   // 肉色浅棕（软组织）
    let color_high = vec3<f32>(0.98, 0.96, 0.92);  // 骨白色（高密度骨骼）

    // 使用 smoothstep 根据 norm 在三种颜色之间平滑插值
    var color = mix(color_low, color_mid, smoothstep(0.0, 0.5, norm));
    color = mix(color, color_high, smoothstep(0.5, 0.9, norm));

    // Alpha (不透明度) 映射：
    // 在 3D Slicer 中，低值通常更透明，高值更不透明。
    // 这里我们使用非线性曲线（例如二次方或立方），让过渡更自然，剥离感更强。
    var base_alpha = pow(norm, 2.5); // 指数越大，低值区域越透明，剥离感越强
    let edge_factor = 1.0 + 1.5 * smoothstep(0.01, 0.1, grad_mag);
    let final_alpha = clamp(base_alpha * edge_factor, 0.0, 1.0);

    return vec4<f32>(color, final_alpha);
}

// ==================== Needle Rendering Functions ====================
// 虚拟针渲染：用于手术规划和穿刺模拟
fn distance_to_line_segment(point: vec3<f32>, line_start: vec3<f32>, line_end: vec3<f32>) -> f32 {
    let line_dir = line_end - line_start;
    let line_len_sq = dot(line_dir, line_dir);
    if (line_len_sq < 1e-12) {
        return length(point - line_start);
    }
    let t = clamp(dot(point - line_start, line_dir) / line_len_sq, 0.0, 1.0);
    let closest = line_start + t * line_dir;
    return length(point - closest);
}

fn project_point_on_line(point: vec3<f32>, line_start: vec3<f32>, line_end: vec3<f32>) -> f32 {
    let line_dir = line_end - line_start;
    let line_len_sq = dot(line_dir, line_dir);
    if (line_len_sq < 1e-12) {
        return 0.0;
    }
    let t = dot(point - line_start, line_dir) / line_len_sq;
    return t;
}

fn sample_needle_at_point(pos: vec3<f32>) -> vec4<f32> {
    if (u_vol.needle_enabled < 0.5) {
        return vec4<f32>(0.0);
    }

    let entry = u_vol.needle_entry;
    let needle_pos = u_vol.needle_pos;
    let needle_target = u_vol.needle_target;
    let radius = u_vol.needle_radius;

    let t = project_point_on_line(pos, entry, needle_target);
    let dist = distance_to_line_segment(pos, entry, needle_target);

    if (dist < radius) {
        let edge_factor = 1.0 - smoothstep(0.0, radius, dist);
        var color: vec3<f32>;
        var alpha: f32;

        if (t >= 0.0 && t <= 1.0) {
            if (t < 1.0) {
                let solid_t = project_point_on_line(needle_pos, entry, needle_target);
                if (t <= solid_t) {
                    color = vec3<f32>(1.0, 0.85, 0.0);
                    let highlight = pow(edge_factor, 1.5);
                    color = color * (0.7 + 0.3 * highlight);
                    alpha = clamp(edge_factor * 0.95, 0.0, 1.0);
                } else {
                    let dash_scale = 20.0;
                    let dash_pos = t * dash_scale;
                    let dash = fract(dash_pos);
                    if (dash < 0.5) {
                        color = vec3<f32>(1.0, 0.5, 0.0);
                        alpha = clamp(edge_factor * 0.7, 0.0, 1.0);
                    } else {
                        return vec4<f32>(0.0);
                    }
                }
            } else {
                return vec4<f32>(0.0);
            }
        } else {
            return vec4<f32>(0.0);
        }

        return vec4<f32>(color, alpha);
    }

    return vec4<f32>(0.0);
}

fn sample_needle_along_ray(ray_origin: vec3<f32>, ray_dir: vec3<f32>, t_start: f32, t_end: f32) -> vec4<f32> {
    if (u_vol.needle_enabled < 0.5) {
        return vec4<f32>(0.0);
    }

    let entry = u_vol.needle_entry;
    let needle_target_pt = u_vol.needle_target;
    let radius = u_vol.needle_radius;

    let needle_dir = needle_target_pt - entry;
    let needle_len_sq = dot(needle_dir, needle_dir);
    if (needle_len_sq < 1e-12) {
        return vec4<f32>(0.0);
    }

    let ray_to_entry = entry - ray_origin;
    let a = dot(ray_dir, ray_dir);
    let b = -2.0 * dot(ray_dir, needle_dir);
    let c = dot(needle_dir, needle_dir) * (radius * radius) - length(cross(ray_to_entry, needle_dir));

    let discriminant = b * b - 4.0 * a * c;
    if (discriminant < 0.0) {
        let closest_dist = distance_to_line_segment(ray_origin, entry, needle_target_pt);
        if (closest_dist >= radius) {
            return vec4<f32>(0.0);
        }
    }

    var accum_rgb = vec3<f32>(0.0);
    var accum_a = 0.0;
    var t = t_start;
    let step = (t_end - t_start) / 64.0;

    for (var i = 0u; i < 64u; i = i + 1u) {
        if (t > t_end || accum_a > 0.95) {
            break;
        }

        let pos = ray_origin + t * ray_dir;
        let needle_sample = sample_needle_at_point(pos);

        if (needle_sample.a > 0.005) {
            accum_rgb += (1.0 - accum_a) * needle_sample.rgb * needle_sample.a;
            accum_a += (1.0 - accum_a) * needle_sample.a;
        }

        t = t + step;
    }

    return vec4<f32>(accum_rgb, clamp(accum_a, 0.0, 1.0));
}

fn iso_ray_march(ray_origin: vec3<f32>, ray_dir: vec3<f32>, t0: f32, t1: f32, iso: f32) -> vec4<f32> {
    let dims = u_vol.vol_dims;
    let ray_dir_vox = ray_dir * dims;
    let inv_len = 1.0 / max(length(ray_dir_vox), 1e-6);
    
    let step_vox = 1.0; 
    let dt = step_vox * inv_len;
    let max_steps = u32(max(u_vol.max_steps, 1.0));

    var t = t0;
    var v_prev = sample_volume(ray_origin + t * ray_dir) - iso;

    for (var i = 0u; i < max_steps; i = i + 1u) {
        t = t + dt;
        if (t > t1) {
            break;
        }

        let p = ray_origin + t * ray_dir;
        let v_cur = sample_volume(p) - iso;

        if (v_prev * v_cur <= 0.0) {
            var a = t - dt;
            var b = t;
            var va = v_prev;

            for (var j = 0u; j < 4u; j = j + 1u) {
                let m = 0.5 * (a + b);
                let vm = sample_volume(ray_origin + m * ray_dir) - iso;
                if (va * vm <= 0.0) {
                    b = m;
                } else {
                    a = m;
                    va = vm;
                }
            }

            let t_hit = 0.5 * (a + b);
            let hit_pos = ray_origin + t_hit * ray_dir;

            let n = compute_normal(hit_pos);
            let v = normalize(-ray_dir);

            let hu = sample_volume(hit_pos);
            let base = bone_base_color(hu);
            let col = compute_lighting(n, v, base);

            // Exponential mapping
            let mapped_opacity = pow(u_vol.opacity_multiplier, 6.0);
            let density = mapped_opacity * 3.0;
            let sample_alpha = 1.0 - exp(-density);
            return vec4<f32>(col * sample_alpha, sample_alpha);
        }

        v_prev = v_cur;
    }

    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}

fn dvr_ray_march(ray_origin: vec3<f32>, ray_dir: vec3<f32>, t0: f32, t1: f32) -> vec4<f32> {
    let dims = u_vol.vol_dims;
    let ray_dir_vox = ray_dir * dims;
    let inv_len = 1.0 / max(length(ray_dir_vox), 1e-6);
    
    let step_vox = 0.5; 
    let dt = step_vox * inv_len;
    let step_len = step_vox; 
    let max_steps = u32(max(u_vol.max_steps, 1.0));

    var accum_rgb = vec3<f32>(0.0);
    var accum_a = 0.0;
    var t = t0;

    let center = u_vol.level;
    let width = max(u_vol.window, 1e-6);
    let min_val = center - 0.5 - (width - 1.0) * 0.5;

    for (var i = 0u; i < max_steps; i = i + 1u) {
        if (t > t1 || accum_a > 0.97) {
            break;
        }

        let pos = ray_origin + t * ray_dir;
        let hu = sample_volume(pos);

        if (hu < min_val) {
            t = t + dt * 2.0;
            continue;
        }

        let n = compute_normal(pos);
        let grad_mag = length(n);
        let tf = transfer_function(hu, grad_mag);

        if (tf.a > 0.005) {
            let vdir = normalize(-ray_dir);
            let lit_color = compute_lighting(n, vdir, tf.rgb);

            let mapped_opacity = pow(u_vol.opacity_multiplier, 6.0);
            let density = tf.a * mapped_opacity * 3.0; 
            let volume_alpha = 1.0 - exp(-density * step_len);

            let needle_sample = sample_needle_at_point(pos);
            var final_color = lit_color;
            var final_alpha = volume_alpha;

            if (needle_sample.a > 0.005) {
                final_color = mix(lit_color, needle_sample.rgb, needle_sample.a * 0.85);
                final_alpha = max(volume_alpha, needle_sample.a * 0.9);
            }

            accum_rgb += (1.0 - accum_a) * final_color * final_alpha;
            accum_a += (1.0 - accum_a) * final_alpha;
        }

        t = t + dt;
    }

    return vec4<f32>(accum_rgb, clamp(accum_a, 0.0, 1.0));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let scale = max(u_vol.scale, 0.0001);
    var uv_centered = in.tex_coords - vec2<f32>(0.5, 0.5);

    if (u_vol.aspect_ratio > 1.0) {
        uv_centered.x = uv_centered.x * u_vol.aspect_ratio;
    } else if (u_vol.aspect_ratio < 1.0 && u_vol.aspect_ratio > 0.0) {
        uv_centered.y = uv_centered.y / u_vol.aspect_ratio;
    }

    let uv = (uv_centered / scale) + vec2<f32>(0.5, 0.5) + vec2<f32>(u_vol.pan_x, u_vol.pan_y);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let center = vec3<f32>(0.5, 0.5, 0.5);
    let base_ray_origin = vec3<f32>(uv.x, 1.0 - uv.y, -0.5);

    let ray_origin = (u_vol.rotation * vec4<f32>(base_ray_origin - center, 1.0)).xyz + center;
    let ray_dir = normalize((u_vol.rotation * vec4<f32>(0.0, 0.0, 1.0, 0.0)).xyz);

    let inter_vol = intersect_box(ray_origin, ray_dir, vec3<f32>(0.0), vec3<f32>(1.0));
    var t_start = inter_vol.x;
    var t_end = inter_vol.y;
    if (t_start >= t_end) {
        return vec4<f32>(0.0);
    }

    let inter_roi = intersect_box(ray_origin, ray_dir, u_vol.roi_min, u_vol.roi_max);
    t_start = max(t_start, inter_roi.x);
    t_end = min(t_end, inter_roi.y);
    if (t_start >= t_end) {
        return vec4<f32>(0.0);
    }

    let dims = u_vol.vol_dims;
    let ray_dir_vox = ray_dir * dims;
    let inv_len = 1.0 / max(length(ray_dir_vox), 1e-6);
    let step_vox = 0.5;
    let dt = step_vox * inv_len;
    
    t_start = t_start + hash(in.tex_coords) * dt;

    // Step 1: Render volume normally (ISO or DVR mode)
    var volume_result: vec4<f32>;
    if (u_vol.preset < 0.5) {
        let iso = get_iso_threshold();
        volume_result = iso_ray_march(ray_origin, ray_dir, t_start, t_end, iso);
    } else {
        volume_result = dvr_ray_march(ray_origin, ray_dir, t_start, t_end);
    }

    // Step 2: Render needle as overlay (always visible on top of volume)
    let needle_overlay = sample_needle_along_ray(ray_origin, ray_dir, t_start, t_end);

    // Step 3: Composite: needle over volume
    if (needle_overlay.a > 0.001) {
        let vol_rgb = volume_result.rgb;
        let vol_a = volume_result.a;
        let needle_a = needle_overlay.a;
        let out_a = vol_a + (1.0 - vol_a) * needle_a;
        if (out_a > 0.001) {
            let out_rgb = (vol_rgb * vol_a * (1.0 - needle_a) + needle_overlay.rgb * needle_a) / out_a;
            return vec4<f32>(out_rgb, out_a);
        }
    }

    return volume_result;
}