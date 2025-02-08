// Vertex shader

struct Uniforms {
    rotation_angle_y: f32,
    rotation_angle_z: f32,
    _padding0: f32,
    _padding1: f32,
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

    let scale = mat4x4<f32>(
        2.0, 0.0, 0.0, 0.0,
        0.0, 2.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    );

    // Set the output
    out.tex_coords = model.tex_coords;
    // out.clip_position = vec4<f32>(model.position, 1.0);
    out.clip_position = rotation_matrix_z * rotation_matrix_y * scale * vec4<f32>(model.position, 1.0);
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
    _padding: f32,
	mat: mat4x4<f32>,
}

@group(2) @binding(0)
var<uniform> u_uniform_frag: UniformsFrag;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Ensure that the depth is correctly set and within valid bounds
    let depth = u_uniform_frag.slice;  // Ensure this is set correctly
    // let tex_coords_3d = (u_uniform_frag.mat * vec4<f32>(in.tex_coords.x, in.tex_coords.y, depth * 0.2 + 0.5, 1)).xyz;
    let tex_coords_3d = (u_uniform_frag.mat * vec4<f32>(in.tex_coords.x, in.tex_coords.y, depth, 1)).xyz;

    // Component-wise comparison for out-of-bounds check
    let out_of_bounds = any(tex_coords_3d.x < 0.0 || tex_coords_3d.x > 1.0 ||
                             tex_coords_3d.y < 0.0 || tex_coords_3d.y > 1.0 ||
                             tex_coords_3d.z < 0.0 || tex_coords_3d.z > 1.0);


    // If the texture coordinates are out of bounds, return black
    if out_of_bounds {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);  // Black color for out-of-bound coordinates
    }

    // Sample the texture using the 3D coordinates
    let sampled_value: vec4<f32> = textureSample(t_diffuse, s_diffuse, tex_coords_3d);

    // Unpack the two unsigned bytes back to a signed int16 (if needed)
    let unpacked_value: f32 = (sampled_value.g * 256.0 + sampled_value.r) * 255.0;

    // Compute the final value with clamping based on window and level
    let v: f32 = clamp((unpacked_value - (u_uniform_frag.level - u_uniform_frag.window / 2.0)) / u_uniform_frag.window, 0.0, 1.0);

    // Return the final computed color
    return vec4<f32>(vec3<f32>(v), 1.0);
}

// @group(2) @binding(0)
// var<uniform> u_uniform_frag: UniformsFrag;

// @fragment
// fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
//     // // Create a 3D coordinate using in.tex_coords and the slice
//     let depth = u_uniform_frag.slice; // Ensure this is set correctly
//     // let tex_coords_3d = vec3<f32>(in.tex_coords, depth);
//     // let tex_coords_3d = vec3<f32>(in.tex_coords.x, depth * 0.2 + 0.5, 2.04-in.tex_coords.y * 3.08);

//     let tex_coords_3d = (u_uniform_frag.mat * vec4<f32>(in.tex_coords.x, in.tex_coords.y, depth * 0.2 + 0.5, 1)).xyz;

//     let sampled_value: vec4<f32> = textureSample(t_diffuse, s_diffuse, tex_coords_3d);

//     // Unpack the two unsigned bytes back to a signed int16
//     let unpacked_value: f32 = (sampled_value.g * 256.0 + sampled_value.r) * 255.0;

//     let v: f32 = clamp((unpacked_value - (u_uniform_frag.level - u_uniform_frag.window / 2.0)) / u_uniform_frag.window, 0.0, 1.0);

//     return vec4<f32>(vec3<f32>(v), 1.0);
//     // return textureSample(t_diffuse, s_diffuse, tex_coords_3d);
// }
