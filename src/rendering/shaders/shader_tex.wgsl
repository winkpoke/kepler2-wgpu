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
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    // Apply rotation (you may want to adjust this)
    let u_rotation_z = u_uniform.rotation_angle_z;
    let rotation_matrix_z = mat4x4<f32>(
        cos(u_rotation_z), sin(u_rotation_z), 0.0, 0.0,
       -sin(u_rotation_z), cos(u_rotation_z), 0.0, 0.0,
        0.0,               0.0,                1.0, 0.0,
        0.0,               0.0,                0.0, 1.0
    );
    let u_rotation_y = u_uniform.rotation_angle_y;
    let rotation_matrix_y = mat4x4<f32>(
        cos(u_rotation_y), 0.0, -sin(u_rotation_y), 0.0,
        0.0,               1.0,  0.0,               0.0,
        sin(u_rotation_y), 0.0,  cos(u_rotation_y), 0.0,
        0.0,               0.0,  0.0,               1.0
    );

    let scale_matrix = mat4x4<f32>(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    );

    // Set the output
    out.tex_coords = model.tex_coords;
    // out.clip_position = vec4<f32>(model.position, 1.0);
    out.clip_position = rotation_matrix_z * rotation_matrix_y * scale_matrix * vec4<f32>(model.position, 1.0);
    out.clip_position.z += 0.5;
    return out;
}
// Fragment shader

@group(0) @binding(0)
// var t_diffuse: texture_2d<f32>;
var t_diffuse: texture_3d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

struct UniformsFrag {
    window: f32,
    level: f32,
    slice: f32,
    is_packed_rg8: f32,
    bias: f32,
    is_dual_mode: f32,
    slice2: f32,
    aliasing: u32, 
    mat: mat4x4<f32>,
    mat2: mat4x4<f32>,
}

@group(2) @binding(0)
var<uniform> u_uniform_frag: UniformsFrag;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var depth = u_uniform_frag.slice;
    var local_x = in.tex_coords.x;
    var current_mat = u_uniform_frag.mat;
    
    // Intersection line drawing
    var draw_line = false;
    
    if (u_uniform_frag.is_dual_mode > 0.5) {
        if (in.tex_coords.x < 0.498) {
            // Left view: use mat1
            local_x = in.tex_coords.x * 2.0;
            current_mat = u_uniform_frag.mat;
            depth = 0.0;
            
            // Calculate intersection with plane 2
            let p1 = (current_mat * vec4<f32>(local_x, in.tex_coords.y, depth, 1.0)).xyz;
            let n2 = normalize(vec3<f32>(u_uniform_frag.mat2[2][0], u_uniform_frag.mat2[2][1], u_uniform_frag.mat2[2][2]));
            let p2 = (u_uniform_frag.mat2 * vec4<f32>(0.5, 0.5, 0.0, 1.0)).xyz;
            let dist = dot(p1 - p2, n2);
            if (abs(dist) < 0.003) {
                draw_line = true;
            }
            
        } else if (in.tex_coords.x > 0.502) {
            // Right view: use mat2
            local_x = (in.tex_coords.x - 0.5) * 2.0;
            current_mat = u_uniform_frag.mat2;
            depth = 0.0;
            
            // Calculate intersection with plane 1
            let p2 = (current_mat * vec4<f32>(local_x, in.tex_coords.y, depth, 1.0)).xyz;
            let n1 = normalize(vec3<f32>(u_uniform_frag.mat[2][0], u_uniform_frag.mat[2][1], u_uniform_frag.mat[2][2]));
            let p1 = (u_uniform_frag.mat * vec4<f32>(0.5, 0.5, 0.0, 1.0)).xyz;
            let dist = dot(p2 - p1, n1);
            if (abs(dist) < 0.003) {
                draw_line = true;
            }
            
        } else {
            // Divider
            return vec4<f32>(0.2, 0.2, 0.2, 1.0);
        }
    }

    let tex_coords_3d = (current_mat * vec4<f32>(local_x, in.tex_coords.y, depth, 1.0)).xyz;

    // Component-wise comparison for out-of-bounds check
    let out_of_bounds = any(tex_coords_3d < vec3<f32>(0.0)) || any(tex_coords_3d > vec3<f32>(1.0));

    // If the texture coordinates are out of bounds, return black
    if out_of_bounds {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    // Sample the texture using the 3D coordinates
    // sampled_value = textureSample(t_diffuse, s_diffuse, tex_coords_3d);
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
    
    var final_color = vec3<f32>(v);
    if (draw_line) {
        // Red indicator line
        final_color = vec3<f32>(1.0, 0.0, 0.0);
    }

    // Return the final computed color
    return vec4<f32>(final_color, 1.0);
}