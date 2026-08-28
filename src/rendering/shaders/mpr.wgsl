// Vertex shader
struct Uniforms {
    rotation_angle_y: f32,
    rotation_angle_z: f32,
    _padding1: f32,
    _padding2: f32,
};

@group(1) @binding(0)
var<uniform> u_uniform: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let cz = cos(u_uniform.rotation_angle_z);
    let sz = sin(u_uniform.rotation_angle_z);

    let cy = cos(u_uniform.rotation_angle_y);
    let sy = sin(u_uniform.rotation_angle_y);

    let rotation_matrix_z = mat4x4<f32>(
        cz,  sz, 0.0, 0.0,
       -sz,  cz, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0
    );

    let rotation_matrix_y = mat4x4<f32>(
        cy, 0.0, -sy, 0.0,
        0.0, 1.0,  0.0, 0.0,
        sy, 0.0,  cy, 0.0,
        0.0, 0.0,  0.0, 1.0
    );

    let scale_matrix = mat4x4<f32>(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    );

    // Set the output
    out.tex_coords = model.tex_coords;
    out.clip_position = rotation_matrix_z * rotation_matrix_y * scale_matrix * vec4<f32>(model.position, 1.0);
    out.clip_position.z += 0.5;
    return out;
}

// Fragment shader
@group(0) @binding(0)
var t_diffuse: texture_3d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

// Segmentation overlay: R8Uint 3D texture sharing the CT volume's UV space.
// label == 0 means background, label > 0 means a foreground anatomical region.
// The texture is bound to the same bind group (group 0) as the volume
// texture; bindings 2/3 are dedicated to the segmentation.
@group(0) @binding(2)
var t_segmentation: texture_3d<u32>;
@group(0) @binding(3)
var s_segmentation: sampler;

struct UniformsFrag {
    window: f32,
    level: f32,
    slice: f32,
    is_packed_rg8: f32,
    bias: f32,
    is_dual_mode: f32,
    seg_jet: f32,
    aliasing: u32, 
    mat: mat4x4<f32>,
    needle_count: u32,
    needle_enabled: f32,
    seg_enabled: f32,
    seg_alpha: f32,
    needles: array<NeedleUniform, 32>,
    label_colors: array<vec4<f32>, 8>,
    label_visibility: array<vec4<f32>, 8>,
}

@group(2) @binding(0)
var<uniform> u_uniform_frag: UniformsFrag;

// 3D point-to-segment distance in volume-UV space
fn distance_point_to_segment_3d(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>) -> f32 {
    let ab = b - a;
    let len_sq = dot(ab, ab);
    if (len_sq < 1e-10) {
        return distance(p, a);
    }
    let t = clamp(dot(p - a, ab) / len_sq, 0.0, 1.0);
    let foot = a + t * ab;
    return distance(p, foot);
}

fn jet(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);
    let r = clamp(1.5 - abs(4.0 * x - 3.0), 0.0, 1.0);
    let g = clamp(1.5 - abs(4.0 * x - 2.0), 0.0, 1.0);
    let b = clamp(1.5 - abs(4.0 * x - 1.0), 0.0, 1.0);
    return vec3<f32>(r, g, b);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var depth = u_uniform_frag.slice;
    var local_x = in.tex_coords.x;
    var current_mat = u_uniform_frag.mat;
    
    let tex_coords_3d = (current_mat * vec4<f32>(local_x, in.tex_coords.y, depth, 1.0)).xyz;

    // Component-wise comparison for out-of-bounds check
    let out_of_bounds = any(tex_coords_3d < vec3<f32>(0.0)) || any(tex_coords_3d > vec3<f32>(1.0));

    // If the texture coordinates are out of bounds, return black
    if out_of_bounds {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    // Sample the texture using the 3D coordinates
    var sampled_value: vec4<f32>;
    if (u_uniform_frag.aliasing == 0) {
        sampled_value = textureSample(t_diffuse, s_diffuse, tex_coords_3d);
    } else {
        let tex_size = vec3<f32>(textureDimensions(t_diffuse));
        let coord = vec3<i32>(tex_coords_3d * tex_size);
        sampled_value = textureLoad(t_diffuse, coord, 0);
    }

    // Conditionally decode depending on texture format
    var value: f32;
    if (u_uniform_frag.is_packed_rg8 > 0.5) {
        let low = sampled_value.r * 255.0;
        let high = sampled_value.g * 255.0;
        let u16_val = low + high * 256.0;
        value = u16_val - u_uniform_frag.bias;
    } else {
        value = sampled_value.r;
    }

    // DICOM PS3.3 C.11.2 Window/Level mapping
    let center = u_uniform_frag.level;
    let width = u_uniform_frag.window;
    var v: f32;
    if (value <= (center - 0.5 - (width - 1.0) / 2.0)) {
        v = 0.0;    
    } else if (value > (center - 0.5 + (width - 1.0) / 2.0)) {
        v = 1.0;
    } else {
        v = ((value - (center - 0.5)) / (width - 1.0)) + 0.5;
    }
    v = clamp(v, 0.0, 1.0);
    
    let final_color = vec3<f32>(v);

    // Segmentation overlay
    if (u_uniform_frag.seg_enabled > 0.5) {
        let seg_tex_size = vec3<f32>(textureDimensions(t_segmentation));
        let seg_coord = vec3<i32>(clamp(tex_coords_3d, vec3<f32>(0.0), vec3<f32>(1.0)) * seg_tex_size);
        let seg_label = textureLoad(t_segmentation, seg_coord, 0).r;
        if (seg_label > 0u) {
            if (u_uniform_frag.seg_jet > 0.5) {
                let t = f32(seg_label) / 255.0;
                let a = clamp(u_uniform_frag.seg_alpha, 0.0, 1.0);
                return vec4<f32>(mix(final_color, jet(t), a), 1.0);
            } else {
                let idx = min(seg_label, 8u);
                if (u_uniform_frag.label_visibility[idx].x > 0.5) {
                    let seg_overlay = u_uniform_frag.label_colors[idx].rgb;
                    let a = clamp(u_uniform_frag.seg_alpha, 0.0, 1.0);
                    return vec4<f32>(mix(final_color, seg_overlay, a), 1.0);
                }
            }
        }
    }

    // 3D needle intersection on the slice
    if (u_uniform_frag.needle_enabled > 0.5 && u_uniform_frag.needle_count > 0u) {
        var best_dist = 1e20;
        var best_color = vec3<f32>(0.0);
        var best_alpha: f32 = 0.0;

        for (var k: u32 = 0u; k < u_uniform_frag.needle_count; k = k + 1u) {
            let needle = u_uniform_frag.needles[k];
            let d = distance_point_to_segment_3d(tex_coords_3d, needle.entry, needle.tip);
            if (d < needle.radius && d < best_dist) {
                best_dist = d;
                best_color = needle.color.rgb;
                // Soft edge falloff for anti-aliased look
                let edge = clamp((needle.radius - d) / max(needle.radius, 1e-6), 0.0, 1.0);
                best_alpha = 0.55 + 0.45 * edge;
            }
        }

        if (best_alpha > 0.0) {
            // Composite needle over the slice using straight alpha
            return vec4<f32>(mix(final_color, best_color, best_alpha), 1.0);
        }
    }

    // Return the final computed color
    return vec4<f32>(final_color, 1.0);
}