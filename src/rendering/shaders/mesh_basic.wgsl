// Bind group 0: Transform uniforms
@group(0) @binding(0)
var<uniform> uni: Uniforms;

// Bind group 1: Lighting uniforms
@group(1) @binding(0)
var<uniform> lighting: BasicLightingUniforms;

// =============================================================================
// All mesh geometry lives in the shared `[0, 1]^3` UV/world space. The model
// matrix is the identity in this space, so `model_view_proj == view_proj` and
// the mesh's `frag_depth` is directly comparable with the DVR volume's
// `frag_depth` (both are written via the same matrix).
// =============================================================================
struct Uniforms {
    // UV-space (`[0, 1]^3`) -> clip space. Identical to the matrix used
    // by the DVR volume shader, so per-fragment depth values line up.
    model_view_proj: mat4x4<f32>,
    // Slice plane (in the same `[0, 1]^3` UV/world space) equation: normal · x + d = 0
    plane_normal: vec3<f32>,
    plane_d: f32,
    slice_thickness: f32,
    slice_enabled: f32,
    _pad: vec2<f32>,
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
    // Vertex position in the shared `[0, 1]^3` UV/world space
    @location(2) v_uv_pos: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = uni.model_view_proj * vec4<f32>(in.position, 1.0);
    out.v_color = in.color;
    out.v_normal = normalize(in.normal);
    out.v_uv_pos = in.position;
    return out;
}

// =============================================================================
// Fragment output: emit the lit color plus a per-fragment depth so the mesh
// shares the render pass depth buffer with the DVR volume.
//
//   * color:  RGBA lit color, alpha-blended by the blend state
//   * depth:  perspective-divided clip-space Z, remapped to [0, 1]
//
// Writing the depth is what lets the volume occlude the mesh when the
// volume's first hit is in front of the mesh surface, and lets the mesh
// occlude the volume when the mesh surface is in front of the volume.
// =============================================================================
struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    // Slice clipping: keep only fragments inside the slab around the plane.
    if (uni.slice_enabled > 0.5) {
        let dist = abs(dot(in.v_uv_pos, uni.plane_normal) + uni.plane_d);
        if (dist > uni.slice_thickness * 0.5) {
            discard;
        }
    }

    let n = normalize(in.v_normal);
    let l = normalize(-lighting.light_direction);
    let d = abs(dot(n, l));
    let factor = clamp(0.8 + 0.2 * d, 0.0, 1.0);
    let c = in.v_color * factor;
    let ndc_z = in.position.z / in.position.w;
    let depth = ndc_z * 0.5 + 0.5;

    return FragmentOutput(vec4<f32>(c, lighting.opacity), depth);
}
