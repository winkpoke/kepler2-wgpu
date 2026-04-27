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
    if (u_vol.preset > 0.5) {
        return 250.0; // ANGIO
    } else if (u_vol.preset < 1.5) {
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

// Phong Lighting Model
// Adds shading using normal, light direction, and view direction
fn compute_lighting(normal_in: vec3<f32>, view_dir_in: vec3<f32>, base_color: vec3<f32>) -> vec3<f32> {
    let n = normalize(normal_in);
    let v = normalize(view_dir_in);
    // 假设光源来自相机略微偏右上的位置
    let l = normalize(u_vol.light_dir + vec3<f32>(0.2, 0.2, 0.0)); 

    let diff = max(dot(n, l), 0.0);

    let h = normalize(l + v);
    // 降低高光指数 (shininess) 让高光点更大、更柔和 (原来是 48.0)
    let spec = pow(max(dot(n, h), 0.0), 16.0); 

    let ambient = 0.4;  // 稍微提高环境光，避免背光面死黑
    let kd = 0.8;       // 漫反射系数
    let ks = 0.3;       // 稍微增强高光强度

    return base_color * (ambient + kd * diff) + vec3<f32>(1.0) * (ks * spec);
}

fn bone_base_color(hu: f32) -> vec3<f32> {
    let t = clamp((hu - 200.0) / 1200.0, 0.0, 1.0);
    let dark = vec3<f32>(0.45, 0.35, 0.25);
    let light = vec3<f32>(0.90, 0.86, 0.78);
    return mix(dark, light, t);
}

fn transfer_function(hu: f32, grad_mag: f32) -> vec4<f32> {
    let norm = apply_window_level(hu);
    
    // 如果值低于窗底，完全不可见
    if (norm <= 0.0) {
        return vec4<f32>(0.0);
    }
    
    // 基础颜色设定：从深红棕色（低值）过渡到浅黄色/白色（高值）
    let color_low = vec3<f32>(0.4, 0.1, 0.05);   // 暗红棕色 (类似外层组织)
    let color_mid = vec3<f32>(0.8, 0.4, 0.2);    // 亮橙棕色 (类似肌肉)
    let color_high = vec3<f32>(1.0, 0.95, 0.85); // 骨白色

    // 使用 smoothstep 根据 norm 在三种颜色之间平滑插值
    var color = mix(color_low, color_mid, smoothstep(0.0, 0.5, norm));
    color = mix(color, color_high, smoothstep(0.5, 0.9, norm));

    // Alpha (不透明度) 映射：
    // 在 3D Slicer 中，低值通常更透明，高值更不透明。
    // 这里我们使用非线性曲线（例如二次方或立方），让过渡更自然，剥离感更强。
    var base_alpha = pow(norm, 2.5); // 指数越大，低值区域越透明，剥离感越强

    // 边缘增强：梯度大的地方（边界处）略微增加不透明度，让结构更清晰
    // 避免纯色区域一团糊
    let edge_factor = 1.0 + 1.5 * smoothstep(0.01, 0.1, grad_mag);
    let final_alpha = clamp(base_alpha * edge_factor, 0.0, 1.0);

    return vec4<f32>(color, final_alpha);
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

            return vec4<f32>(col, 1.0);
        }

        v_prev = v_cur;
    }

    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}

fn dvr_ray_march(ray_origin: vec3<f32>, ray_dir: vec3<f32>, t0: f32, t1: f32) -> vec4<f32> {
    let dims = u_vol.vol_dims; // 例如 vec3(512.0, 512.0, 300.0)
    let ray_dir_vox = ray_dir * dims; // 将光线方向从 [0,1] 空间映射到体素空间
    let inv_len = 1.0 / max(length(ray_dir_vox), 1e-6);
    
    // step_vox 决定了每次采样的跨度（0.5 表示每次走半个体素）
    // 你可以通过 u_vol.ray_step_size 来控制这个系数，或者写死
    let step_vox = 0.5; 
    let dt = step_vox * inv_len; // 这是在 [0,1] 归一化空间中，光线每步应该前进的 t 增量
    let step_len = step_vox;     // 这是用于 Alpha 积分的物理/体素长度
    let max_steps = u32(max(u_vol.max_steps, 1.0));

    var accum_rgb = vec3<f32>(0.0);
    var accum_a = 0.0;
    var t = t0;

    // 提取出 min_val 用于早期跳过 (Empty Space Skipping)
    // 只要 HU 低于窗底，apply_window_level 就会返回 0，产生 0 的 alpha。
    // 我们可以直接跳过这些采样，极大提升性能！
    let center = u_vol.level;
    let width = max(u_vol.window, 1e-6);
    let min_val = center - 0.5 - (width - 1.0) * 0.5;

    for (var i = 0u; i < max_steps; i = i + 1u) {
        if (t > t1 || accum_a > 0.97) {
            break;
        }

        let pos = ray_origin + t * ray_dir;
        let hu = sample_volume(pos);

        // 如果 HU 低于窗底，完全透明，加大步长跳过
        if (hu < min_val) {
            t = t + dt * 2.0;
            continue;
        }

        let n = compute_normal(pos);
        let grad_mag = length(n);
        let tf = transfer_function(hu, grad_mag);

        // 如果该体素有可见贡献
        if (tf.a > 0.005) {
            let vdir = normalize(-ray_dir);
            // 光照计算保持不变
            let lit_color = compute_lighting(n, vdir, tf.rgb);

            // 转换 Alpha 到密度进行合成 (Front-to-Back)
            let density = tf.a * u_vol.opacity_multiplier * 50.0; 
            let sample_alpha = 1.0 - exp(-density * step_len); 

            accum_rgb += (1.0 - accum_a) * lit_color * sample_alpha;
            accum_a += (1.0 - accum_a) * sample_alpha;
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
    let uv_clamped = clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0),);

    let center = vec3<f32>(0.5, 0.5, 0.5);
    let base_ray_origin = vec3<f32>(uv_clamped.x, 1.0 - uv_clamped.y, -0.5);

    let ray_origin = (u_vol.rotation * vec4<f32>(base_ray_origin - center, 1.0)).xyz + center;
    let ray_dir = normalize((u_vol.rotation * vec4<f32>(0.0, 0.0, 1.0, 0.0)).xyz);

    let inter_vol = intersect_box(ray_origin, ray_dir, vec3<f32>(0.0), vec3<f32>(1.0));
    var t_start = max(inter_vol.x, 0.0);
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

    // 计算步长抖动 (Jittering) 以减少条纹
    let dims = u_vol.vol_dims;
    let ray_dir_vox = ray_dir * dims;
    let inv_len = 1.0 / max(length(ray_dir_vox), 1e-6);
    let step_vox = 0.5; // 根据模式不同，这里可能略有差异，可以使用 uniform 传过来
    let dt = step_vox * inv_len;
    
    t_start = t_start + hash(in.tex_coords) * dt;

    // 进入具体的渲染模式
    if (u_vol.preset < 1.5) {
        let iso = get_iso_threshold();
        // 传入经过 ROI 裁剪后的 t_start 和 t_end
        return iso_ray_march(ray_origin, ray_dir, t_start, t_end, iso);
    }

    return dvr_ray_march(ray_origin, ray_dir, t_start, t_end);
}