// =============================================================================
// Mesh Volume Rendering Shader
// =============================================================================
// Renders a 3D medical volume (CT / MR) for a single fullscreen quad view.
//
// Two rendering modes are supported, selected by `u_vol.preset`:
//   * preset < 0.5  → ISO surface raymarching (binary isosurface, 4-iter bisection)
//   * preset >= 0.5 → DVR (front-to-back alpha compositing with transfer function)
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
//   group(0): volume texture (3D) + sampler
//   group(1): MeshUniforms uniform buffer
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

struct NeedleUniform {
    entry : vec3<f32>,
    radius : f32,
    tip : vec3<f32>,
    id : u32,
    color : vec4<f32>,
};

struct ObliquePlane {
    center : vec3<f32>,
    visible : f32,
    normal : vec3<f32>,
    plane_alpha: f32,
};

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
    needle_count : u32,
    needle_enabled: f32,
    needle_index: u32,
    plane_rotation_angle: f32,
    oblique_planes: array<ObliquePlane, 4>,
    needles : array<NeedleUniform, 32>,
}
@group(1) @binding(0)
var<uniform> u_vol: MeshUniforms;


// -----------------------------------------------------------------------------
// Geometry helpers
// -----------------------------------------------------------------------------

// Returns true if `p` lies inside a finite-thickness disc centred at `center`,
// with face-normal `axis` and radius `radius`. Thickness is measured as
// `thickness` along `axis` (i.e. the disc is a cylinder cap with `thickness` height).
fn point_inside_disc(p: vec3<f32>, center: vec3<f32>, axis: vec3<f32>, radius: f32, thickness: f32) -> bool {
    let v = p - center;
    let h = dot(v, axis);
    if (abs(h) > thickness * 0.5) {
        return false;
    }
    let radial = v - h * axis;
    return dot(radial, radial) <= radius * radius;
}

// Returns true if `p` lies inside a finite cylinder between `entry` and `tip`
// with the given `radius`. Treats degenerate (entry≈tip) needles as empty.
fn point_inside_needle(p: vec3<f32>, entry: vec3<f32>, tip: vec3<f32>, radius: f32) -> bool {
    let axis = tip - entry;
    let len = length(axis);
    if (len < 0.00001) {
        return false;
    }
    let dir = axis / len;
    let v = p - entry;
    let t = dot(v, dir);
    if (t < 0.0 || t > len) {
        return false;
    }
    let closest = entry + dir * t;
    let dist = distance(p, closest);
    return dist < radius;
}

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
// Raymarching: ISO surface
// -----------------------------------------------------------------------------
//
// Walks the ray in voxel units (1.0 voxel step) and detects a sign change in
// (sample - iso). When detected, performs a 4-iteration bisection to refine
// the surface hit, then shades it with Phong lighting and returns an
// alpha-blended color.
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
        t += dt;
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


// -----------------------------------------------------------------------------
// Raymarching: Direct Volume Rendering (DVR)
// -----------------------------------------------------------------------------
struct DvrResult {
    color: vec4<f32>,
    first_hit_depth: f32,
}
fn dvr_ray_march(ray_origin: vec3<f32>, ray_dir: vec3<f32>, t0: f32, t1: f32) -> DvrResult {
    let step_vox = 0.5; 
    let dt = step_vox / max(length(ray_dir * u_vol.vol_dims), 1e-6);
    let step_len = step_vox; 
    let max_steps = u32(max(u_vol.max_steps, 1.0));

    let mapped_opacity = pow(u_vol.opacity_multiplier, 6.0);

    var accum_rgb = vec3<f32>(0.0);
    var accum_a = 0.0;
    var first_hit_t = t1;
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
        var n = vec3<f32>(0.0);

        if (u_vol.needle_enabled > 0.5) {
            for (var k: u32 = 0u; k < u_vol.needle_count; k = k + 1u) {
                let needle = u_vol.needles[k];
                let axis = normalize(needle.tip - needle.entry);
                let inside_shaft = point_inside_needle(pos, needle.entry, needle.tip, needle.radius);
                let inside_head = point_inside_disc(pos, needle.entry, axis, needle.radius * 2.0, needle.radius * 2.0);

                if (inside_shaft || inside_head) {
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

        if (!is_needle) {
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

            let opacity_scale = select(mapped_opacity, 1.0, is_needle);
            let density = tf.a * opacity_scale * 3.0;
            let sample_alpha = 1.0 - exp(-density * step_len);

            accum_rgb += (1.0 - accum_a) * lit_color * sample_alpha;
            accum_a += (1.0 - accum_a) * sample_alpha;
        }

        if (first_hit_t >= t1 && accum_a > 0.08) {
            first_hit_t = t;
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
        return DvrResult(vec4<f32>(accum_rgb, accum_a), 1.0);
    }
    let hit_pos = ray_origin + (first_hit_t + dt * 2.0) * ray_dir;
    let ndc_z = (u_vol.rotation * vec4<f32>(hit_pos, 1.0)).z;
    let norm_depth = clamp((ndc_z + 0.5) / 2.0, 0.0, 1.0);
    return DvrResult(vec4<f32>(accum_rgb, accum_a), norm_depth);
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
    // Screen-space UV with aspect ratio, scale, and pan
    let scale = max(u_vol.scale * 1.5, 0.0001);
    var uv_centered = in.tex_coords - vec2<f32>(0.5, 0.5);

    // if (u_vol.aspect_ratio > 1.0) {
    //     uv_centered.x = uv_centered.x * u_vol.aspect_ratio;
    // } else if (u_vol.aspect_ratio < 1.0 && u_vol.aspect_ratio > 0.0) {
    //     uv_centered.y = uv_centered.y / u_vol.aspect_ratio;
    // }

    let uv = (uv_centered * scale) + vec2<f32>(0.5, 0.5) + vec2<f32>(u_vol.pan_x, u_vol.pan_y);
    // if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
    //     return FragmentOutput(vec4<f32>(0.0, 0.0, 0.0, 1.0), 1.0);
    // }

    // Build the ray in volume space
    let center = vec3<f32>(0.5, 0.5, 0.5);
    let base_ray_origin = vec3<f32>(uv.x, 1.0 - uv.y, -0.5);

    let ray_origin = (u_vol.rotation * vec4<f32>(base_ray_origin - center, 1.0)).xyz + center;
    let ray_dir = normalize((u_vol.rotation * vec4<f32>(0.0, 0.0, 1.0, 0.0)).xyz);

    // Clip ray to the [0,1]^3 AABB
    let inter_vol = intersect_box(ray_origin, ray_dir, vec3<f32>(0.0), vec3<f32>(1.0));
    var t_start = inter_vol.x;
    var t_end = inter_vol.y;
    if (t_start >= t_end) {
        return FragmentOutput(vec4<f32>(0.0), 1.0);
    }

    let dims = u_vol.vol_dims;
    let ray_dir_vox = ray_dir * dims;
    let inv_len = 1.0 / max(length(ray_dir_vox), 1e-6);
    let dt = 0.5 * inv_len;
    t_start = t_start + hash(in.tex_coords) * dt;

    // Dispatch to ISO or DVR
    if (u_vol.preset < 0.5) {
        let iso = get_iso_threshold();
        return FragmentOutput(
            iso_ray_march(ray_origin, ray_dir, t_start, t_end, iso),
            0.5,
        );
    } else {
        let result = dvr_ray_march(ray_origin, ray_dir, t_start, t_end);
        return FragmentOutput(result.color, result.first_hit_depth);
    }
}
