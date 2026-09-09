// =============================================================================
// Mesh Volume Rendering Shader
// =============================================================================
// Renders a 3D medical volume (CT / MR) for a single fullscreen quad view.
//
// Within the DVR path, `u_vol.needle_enabled` selects one of four sub-modes:
//   * < 0.5         → needles off
//   * [0.5, 1.5)    → needles visible (shaft + head disc), volume unclipped
//   * [1.5, 2.5)    → needles visible + half-space clip by the rotating plane
//   * >= 2.5        → needles visible + plane visualized as a finite quad
//                     (volume stays unclipped; the quad is composited on top)
//
// Features:
//   * Dual texture format support: R16Float (preferred) and packed RG8 (RGBA8 → u16)
//   * DICOM window/level mapping to normalized intensity
//   * 3-light Phong shading for surface rendering
//   * Needle overlays (shaft + head disc) for biopsy/insertion guidance
//   * ROI cropping when needle plane clipping is active
//   * Per-pixel blue-noise dithering to hide DVR banding
//
// Bindings:
//   group(0): volume texture (3D) + sampler, segmentation texture (3D R8Uint) + sampler
//   group(1): Volume uniform buffer
// =============================================================================


// -----------------------------------------------------------------------------
// Vertex shader: fullscreen triangle (3 verts, no VBO)
// -----------------------------------------------------------------------------
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(((vertex_index & 1u) * 2u)) - 1.0;
    let y = f32((vertex_index & 2u)) - 1.0;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.tex_coords = vec2<f32>((x + 1.0) * 0.5, (y + 1.0) * 0.5);
    return out;
}


// -----------------------------------------------------------------------------
// Fragment bindings
// -----------------------------------------------------------------------------
@group(0) @binding(0)
var t_volume: texture_3d<f32>;
@group(0) @binding(1)
var s_volume: sampler;

// Segmentation overlay texture (R8Uint, non-filterable). Same layout as the
// MPR view so a single bind group can serve both pipelines.
@group(0) @binding(2)
var t_segmentation: texture_3d<u32>;
@group(0) @binding(3)
var s_segmentation: sampler;

struct ObliquePlane {
    center : vec3<f32>,
    visible : f32,
    normal : vec3<f32>,
    plane_alpha: f32,
};

struct VolumeUniforms {
    ray_step_size: f32,
    max_steps: f32,
    is_packed_rg8: f32,
    bias: f32,
    roi_min: vec3<f32>,
    window: f32,
    roi_max: vec3<f32>,
    level: f32,
    vol_dims: vec3<f32>,
    opacity: f32,
    view_proj:mat4x4<f32>,
    inv_view_proj:mat4x4<f32>,
    camera_position:vec3<f32>,
    ball_enabled: f32,
    balls: array<vec4<f32>, 4>,
    volume_scale: vec3<f32>,
    _volume_scale_pad: f32,
    light_dir: vec3<f32>,
    aspect_ratio: f32,
    needle_count : u32,
    needle_enabled: f32,
    needle_index: u32,
    plane_rotation_angle: f32,
    oblique_planes: array<ObliquePlane, 4>,
    needles : array<NeedleUniform, 32>,
}
@group(1) @binding(0)
var<uniform> u_vol: VolumeUniforms;


// -----------------------------------------------------------------------------
// Geometry helpers
// -----------------------------------------------------------------------------


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

// Ray vs finite plane-quad. The quad is the set of points
//   `center + a*u + b*v` for a,b in [-half_size, +half_size],
// where `u` and `v` are assumed to be mutually orthogonal unit vectors
// spanning the plane. Returns the ray parameter t > 0 of the hit, or -1.0
// if the ray misses (or runs parallel to) the quad. O(1) work per ray,
// so the cost of drawing the cutting plane is independent of step count.
fn intersect_plane_quad(
    ray_origin: vec3<f32>,
    ray_dir: vec3<f32>,
    center: vec3<f32>,
    u: vec3<f32>,
    v: vec3<f32>,
    half_size: f32,
) -> f32 {
    let n = normalize(cross(u, v));
    let denom = dot(ray_dir, n);
    // Ray parallel to the plane: no single intersection point.
    if (abs(denom) < 1e-6) {
        return -1.0;
    }
    let t = dot(center - ray_origin, n) / denom;
    // Plane is behind the ray origin: skip.
    if (t < 0.0) {
        return -1.0;
    }
    let p = ray_origin + t * ray_dir;
    // Reject hits that fall outside the [-half, +half] x [-half, +half] quad.
    let a = dot(p - center, u);
    let b = dot(p - center, v);
    if (abs(a) > half_size || abs(b) > half_size) {
        return -1.0;
    }
    return t;
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
// Light 0 (Key):   slightly right of camera, main illumination
// Light 1 (Fill):  left and behind camera, fills shadows
// Light 2 (Rim):   from behind, highlights silhouette edges
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

// Ramps a HU value from dark to light bone color, saturating above ~1400 HU.
fn bone_base_color(hu: f32) -> vec3<f32> {
    let t = clamp((hu - 200.0) / 1200.0, 0.0, 1.0);
    let dark = vec3<f32>(0.85, 0.85, 0.85);
    let light = vec3<f32>(1.0, 1.0, 1.0);
    return mix(dark, light, t);
}

// Maps a HU value to an RGBA color/opacity using a 3-stop color ramp and a
// power-curve alpha. The edge_factor boosts opacity at high gradient regions
// (tissue boundaries) to make surfaces pop.
fn transfer_function(hu: f32, grad_mag: f32) -> vec4<f32> {
    let norm = apply_window_level(hu);
    if (norm <= 0.0) {
        return vec4<f32>(0.0);
    }
    let color_low = vec3<f32>(0.18, 0.14, 0.12);
    let color_mid = vec3<f32>(0.72, 0.55, 0.44);
    let color_high = vec3<f32>(0.98, 0.96, 0.92);
    var color = mix(color_low, color_mid, smoothstep(0.0, 0.5, norm));
    color = mix(color, color_high, smoothstep(0.5, 0.9, norm));

    var base_alpha = pow(norm, 2.5);
    let edge_factor = 1.0 + 1.5 * smoothstep(0.01, 0.1, grad_mag);
    let final_alpha = clamp(base_alpha * edge_factor, 0.0, 1.0);

    return vec4<f32>(color, final_alpha);
}

// Rodrigues rotation
fn rotate_vec_around_axis(v: vec3<f32>, axis: vec3<f32>, angle: f32) -> vec3<f32> {
    let cos_a = cos(angle);
    let sin_a = sin(angle);
    return v * cos_a + cross(axis, v) * sin_a + axis * dot(axis, v) * (1.0 - cos_a);
}

fn build_basis(n: vec3<f32>) -> mat3x3<f32> {
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let right = vec3<f32>(1.0, 0.0, 0.0);
    let a = select(up, right, abs(n.y) > 0.9);
    let u = normalize(cross(a, n));
    let v = cross(n, u);
    return mat3x3<f32>(u, v, n);
}

// -----------------------------------------------------------------------------
// Raymarching: Direct Volume Rendering (DVR)
// -----------------------------------------------------------------------------
struct DvrResult {
    color: vec4<f32>,
    first_hit_depth: f32,
    // World-space position of the first visible (alpha>0.08) sample.
    // Used by the segmentation overlay so the seg volume is sampled at
    // the same world location as the foreground surface.
    first_hit_pos: vec3<f32>,
    hit: f32,  // 1.0 if anything visible was hit, 0.0 otherwise
}
fn dvr_ray_march(ray_origin: vec3<f32>, ray_dir: vec3<f32>, t0: f32, t1: f32) -> DvrResult {
    let step_vox = 0.5; 
    let dt = step_vox / max(length(ray_dir * u_vol.vol_dims), 1e-6);
    let step_len = step_vox; 
    let max_steps = u32(max(u_vol.max_steps, 1.0));

    let mapped_opacity = pow(u_vol.opacity, 6.0);

    var accum_rgb = vec3<f32>(0.0);
    var accum_a = 0.0;
    var first_hit_t = t1;
    var first_hit_pos = vec3<f32>(0.0);
    var t = t0;

    let center = u_vol.level;
    let width = max(u_vol.window, 1e-6);
    let min_val = center - 0.5 - (width - 1.0) * 0.5;
    let any_oblique_visible = (u_vol.oblique_planes[0].visible > 0.5) ||
        (u_vol.oblique_planes[1].visible > 0.5) ||
        (u_vol.oblique_planes[2].visible > 0.5) ||
        (u_vol.oblique_planes[3].visible > 0.5);
    let any_oblique_visible_plane = (u_vol.oblique_planes[0].plane_alpha > 0.0) ||
        (u_vol.oblique_planes[1].plane_alpha > 0.0) ||
        (u_vol.oblique_planes[2].plane_alpha > 0.0) ||
        (u_vol.oblique_planes[3].plane_alpha > 0.0);

    for (var i = 0u; i < max_steps; i = i + 1u) {
        if (t > t1 || accum_a > 0.97) {
            break;
        }

        let pos = ray_origin + t * ray_dir;

        var tf = vec4<f32>(0.0);
        var is_needle = false;
        var is_ball = false;
        var n = vec3<f32>(0.0);

        if (u_vol.needle_enabled > 0.5) {
            for (var k: u32 = 0u; k < u_vol.needle_count; k = k + 1u) {
                let needle = u_vol.needles[k];
                let axis = normalize(needle.tip - needle.entry);
                let inside_shaft = point_inside_needle(pos, needle.entry, needle.tip, needle.radius);
                // let inside_head = point_inside_disc(pos, needle.entry, axis, needle.radius * 2.0, needle.radius * 2.0);

                // if (inside_shaft || inside_head) {
                if (inside_shaft) {
                    is_needle = true;
                    let to_p = pos - needle.entry;
                    let proj = dot(to_p, axis) * axis;
                    n = normalize(to_p - proj + vec3<f32>(1e-6));
                    let vdir = normalize(-ray_dir);
                    let base_color = needle.color.xyz;
                    let lit_color = compute_lighting(n, vdir, base_color);
                    let density = 4.0;
                    let alpha = 1.0 - exp(-density * step_len);
                    tf = vec4<f32>(lit_color, alpha);
                    break;
                }
            }
        }

        if (u_vol.ball_enabled > 0.5) {
            for (var k: u32 = 0u; k <4u; k = k + 1u) {
                let ball = u_vol.balls[k];
                let center = ball.xyz;
                let radius = ball.w;
                let to_center = pos - center;
                if (dot(to_center, to_center) <= radius * radius) {
                    is_ball = true;
                    n = normalize(to_center + vec3<f32>(1e-6));
                    let vdir = normalize(-ray_dir);
                    let base_color = vec3<f32>(1.0, 0.31, 0.0);
                    let lit_color = compute_lighting(n, vdir, base_color);
                    let alpha = 1.0 - exp(-4.0 * step_len);
                    tf = vec4<f32>(lit_color, alpha);
                    break;
                }
            }
        }

        if (!is_needle && !is_ball) {
            if (u_vol.needle_enabled > 1.5 && u_vol.needle_enabled < 2.5) {
                let needle = u_vol.needles[u_vol.needle_index];
                let v0 = needle.entry - needle.tip;
                let axis = normalize(needle.tip - needle.entry);
                let angle = u_vol.plane_rotation_angle;
                let ref_vec = rotate_vec_around_axis(vec3<f32>(1.0, 0.0, 0.0), axis, angle);
                let normal = normalize(cross(v0, ref_vec));
                let d = -dot(normal, needle.tip);
                if (dot(normal, pos) + d < 0.0) {
                    t += dt;
                    continue;
                }
            } else if (any_oblique_visible) {
                var rejected = false;
                for (var i: i32 = 0; i < 4; i = i + 1) {
                    let plane = u_vol.oblique_planes[i];
                    if (plane.visible < 0.5) {
                        continue;
                    }
                    let n_raw = plane.normal;
                    let n = select(normalize(n_raw), vec3<f32>(0.0, 0.0, 1.0), length(n_raw) < 1e-6);
                    let d = dot(n, pos - plane.center);
                    if (d < 0.0) {
                        rejected = true;
                        break;
                    }
                }
                if (rejected) {
                    t += dt;
                    continue;
                }
            }else {
                if (any(pos < u_vol.roi_min) || any(pos > u_vol.roi_max)) {
                    t += dt;
                    continue;
                }
            }

            let hu = sample_volume(pos);
            if (hu < min_val) {
                t += dt * 2.0;
                continue;
            }
            n = compute_normal(pos);
            let grad_mag = length(n);
            tf = transfer_function(hu, grad_mag);
        }

        if (tf.a > 0.005) {
            let vdir = normalize(-ray_dir);
            let lit_color = compute_lighting(n, vdir, tf.xyz);

            let opacity_scale = select(mapped_opacity, 1.0, is_needle || is_ball);
            let density = tf.a * opacity_scale * 3.0;
            let sample_alpha = 1.0 - exp(-density * step_len);

            accum_rgb += (1.0 - accum_a) * lit_color * sample_alpha;
            accum_a += (1.0 - accum_a) * sample_alpha;
        }

        if (first_hit_t >= t1 && accum_a > 0.08) {
            first_hit_t = t;
            first_hit_pos = pos;
        }

        t += dt;
    }

    if (u_vol.needle_enabled > 2.5) {
        let needle = u_vol.needles[u_vol.needle_index];
        let axis = normalize(needle.tip - needle.entry);
        let angle = u_vol.plane_rotation_angle;
        let ref_vec = rotate_vec_around_axis(vec3<f32>(1.0, 0.0, 0.0), axis, angle);
        let plane_center = needle.tip;
        let t_plane = intersect_plane_quad(
            ray_origin, ray_dir,
            plane_center, axis, ref_vec,
            0.5,  // half_size: square of side 1.0 (matches the unit-cube edge)
        );
        if (t_plane >= t0 && t_plane <= t1) {
            let plane_rgb = vec3<f32>(0.30, 0.85, 0.50); // soft green
            let plane_alpha = 0.40;
            accum_rgb += (1.0 - accum_a) * plane_rgb * plane_alpha;
            accum_a += (1.0 - accum_a) * plane_alpha;
        }
    }

    if (any_oblique_visible_plane) {
        let plane_colors = array<vec3<f32>, 4>(
            vec3<f32>(0.9, 0.2, 0.2),
            vec3<f32>(0.3, 0.85, 0.5),
            vec3<f32>(1.0, 0.85, 0.2),
            vec3<f32>(0.4, 0.7, 1.0),
        );
        for (var pi: i32 = 0; pi < 4; pi = pi + 1) {
            let plane = u_vol.oblique_planes[pi];
            if (plane.plane_alpha <= 0.0) {
                continue;
            }
            let n_raw = plane.normal;
            let n = select(normalize(n_raw), vec3<f32>(0.0, 0.0, 1.0), length(n_raw) < 1e-6);
            let c = plane.center;
            let basis = build_basis(n);
            let denom = dot(n, ray_dir);
            if (abs(denom) < 1e-6) {
                continue;
            }
            let t_plane = dot(n, c - ray_origin) / denom;
            if (t_plane < t0 || t_plane > t1) {
                continue;
            }
            let p = ray_origin + ray_dir * t_plane;
            let local = p - c;
            let u = dot(local, basis[0]);
            let v = dot(local, basis[1]);
            if (abs(u) > 2.0 || abs(v) > 2.0) {
                continue;
            }
            let plane_alpha = plane.plane_alpha;
            accum_rgb += (1.0 - accum_a) * plane_colors[pi] * plane_alpha;
            accum_a += (1.0 - accum_a) * plane_alpha;
        }
    }

    if (accum_a < 0.01) {
        return DvrResult(vec4<f32>(accum_rgb, accum_a), 1.0, vec3<f32>(0.0), 0.0);
    }
    let hit_pos = ray_origin + (first_hit_t + dt * 2.0) * ray_dir;
    let hit_pos_world = vec3<f32>(0.5) + (hit_pos - vec3<f32>(0.5)) * u_vol.volume_scale;
    let clip = u_vol.view_proj * vec4<f32>(hit_pos_world, 1.0);
    let ndc_z = clip.z / clip.w;
    let depth = ndc_z * 0.5 + 0.5;
    let first_hit_world = vec3<f32>(0.5) + (first_hit_pos - vec3<f32>(0.5)) * u_vol.volume_scale;
    return DvrResult(vec4<f32>(accum_rgb, accum_a), depth, first_hit_world, 1.0);
}


// -----------------------------------------------------------------------------
// Fragment shader entry point
// -----------------------------------------------------------------------------
struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let uv = in.tex_coords;

    // Build the ray in WORLD space from the shared camera
    let ndc_x = uv.x * 2.0 - 1.0;
    let ndc_y = 1.0 - uv.y * 2.0;
    let ndc_near = vec4(ndc_x, ndc_y, 0.0, 1.0);
    let ndc_far  = vec4(ndc_x, ndc_y, 1.0, 1.0);
    let near_world = u_vol.inv_view_proj * ndc_near;
    let far_world  = u_vol.inv_view_proj * ndc_far;
    let world_origin = near_world.xyz / near_world.w;
    let world_far = far_world.xyz / far_world.w;
    let world_dir = normalize(world_far - world_origin);

    // Convert the world-space ray into VOLUME (texture) space
    let inv_scale = vec3<f32>(
        1.0 / max(u_vol.volume_scale.x, 1e-6),
        1.0 / max(u_vol.volume_scale.y, 1e-6),
        1.0 / max(u_vol.volume_scale.z, 1e-6),
    );
    let tex_origin = (world_origin - vec3<f32>(0.5)) * inv_scale + vec3<f32>(0.5);
    let tex_dir = world_dir * inv_scale;

    // Clip ray to the [0,1]^3 volume AABB (texture space)
    let inter_vol = intersect_box(tex_origin, tex_dir, vec3<f32>(0.0), vec3<f32>(1.0));
    var t_start = inter_vol.x;
    var t_end = inter_vol.y;
    if (t_start >= t_end) {
        return FragmentOutput(vec4<f32>(0.0), 1.0);
    }

    let dims = u_vol.vol_dims;
    let ray_dir_vox = tex_dir * dims;
    let inv_len = 1.0 / max(length(ray_dir_vox), 1e-6);
    let dt = 0.5 * inv_len;
    t_start = t_start + hash(in.tex_coords) * dt;

    // Dispatch to DVR (ray/pos are in texture space)
    let dvr_res = dvr_ray_march(tex_origin, tex_dir, t_start, t_end);
    let base_color = dvr_res.color;
    let depth = dvr_res.first_hit_depth;

    return FragmentOutput(base_color, depth);
}
