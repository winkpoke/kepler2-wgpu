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
    window_scale: f32,
    window_offset: f32,
    opacity: f32,
    _padding2: f32,
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
    
    // Ambient lighting
    let ambient = lighting.ambient_color * lighting.ambient_intensity;
    
    // Combine lighting with vertex color (premultiplied alpha for correct ALPHA_BLENDING)
    let final_color = in.v_color * (ambient + diffuse);
    let alpha = lighting.opacity;
    let color_pm = final_color * alpha;
    return vec4<f32>(color_pm, alpha);
}
