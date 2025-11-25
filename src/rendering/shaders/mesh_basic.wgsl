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
    _padding1: f32,
    light_color: vec3<f32>,
    light_intensity: f32,
    ambient_color: vec3<f32>,
    ambient_intensity: f32,
    padding2: vec3<f32>,
    opacity: f32,
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
    let normal = normalize(in.v_normal);
    let light_dir = normalize(-lighting.light_direction);

    // Lambert diffuse lighting calculation
    let diffuse_factor = max(dot(normal, light_dir), 0.0);
    let diffuse = lighting.light_color * lighting.light_intensity * diffuse_factor;

    // Blinn-Phong specular
    let view_dir = normalize(vec3<f32>(0.0, 0.0, 1.0));
    let half_vec = normalize(light_dir + view_dir);
    let spec_factor = pow(max(dot(normal, half_vec), 0.0), 32.0);
    let specular = lighting.light_color * 0.45 * spec_factor;

    // Ambient lighting
    let ambient = lighting.ambient_color * lighting.ambient_intensity;

    // Rim light to enhance silhouette (low intensity)
    let rim = pow(1.0 - abs(dot(normal, view_dir)), 3.0) * 0.15;

    // Combine and apply gentle gamma correction
    let linear = in.v_color * (ambient + diffuse) + specular + vec3<f32>(rim, rim, rim) + vec3<f32>(0.1);
    let gamma = vec3<f32>(1.0 / 2.0);
    let final_color = pow(clamp(linear, vec3<f32>(0.0), vec3<f32>(1.0)), gamma);

    return vec4<f32>(final_color * lighting.opacity, lighting.opacity);
}
