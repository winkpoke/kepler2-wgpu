// Bind group 0: Transform uniforms
@group(0) @binding(0)
var<uniform> uni: Uniforms;

// Bind group 1: Lighting uniforms
@group(1) @binding(0)
var<uniform> lighting: BasicLightingUniforms;

struct Uniforms {
    model_view_proj: mat4x4<f32>,
};

struct BasicLightingUniforms {
    light_direction: vec3<f32>,
    opacity: f32,
    light_color: vec3<f32>,
    light_intensity: f32,
    ambient_color: vec3<f32>,
    ambient_intensity: f32,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) v_color: vec3<f32>,
    @location(1) v_normal: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = uni.model_view_proj * vec4<f32>(in.position, 1.0);
    out.v_color = in.color;
    out.v_normal = normalize(in.normal);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.v_normal);
    let l = normalize(-lighting.light_direction);
    let d = abs(dot(n, l));
    let factor = clamp(0.8 + 0.2 * d, 0.0, 1.0);
    let c = in.v_color * factor;
    return vec4<f32>(c, lighting.opacity);
}
