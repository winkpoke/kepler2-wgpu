struct View {
    patient_to_clip: mat4x4<f32>,
    // viewport width/height in physical pixels, half stroke width, opacity
    style: vec4<f32>,
};
@group(0) @binding(0) var<uniform> view: View;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) segment_length: f32,
    @location(2) @interpolate(flat) color: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32,
           @location(0) start: vec3<f32>,
           @location(1) end: vec3<f32>,
           @location(2) color: vec4<f32>) -> VertexOut {
    let a = (view.patient_to_clip * vec4<f32>(start, 1.0)).xy;
    let b = (view.patient_to_clip * vec4<f32>(end, 1.0)).xy;
    let delta = (b - a) * view.style.xy * 0.5;
    let len = length(delta);
    let direction = select(vec2<f32>(1.0, 0.0), delta / max(len, 0.00001), len > 0.00001);
    let perpendicular = vec2<f32>(-direction.y, direction.x);
    // Arithmetic corners also validate with the repository's older standalone Naga.
    let corner = vec2<f32>(f32(vertex & 1u), select(-1.0, 1.0, vertex >= 2u));
    let radius = view.style.z + 1.0;
    let along = mix(-radius, len + radius, corner.x);
    let offset = direction * along + perpendicular * corner.y * radius;
    var out: VertexOut;
    out.position = vec4<f32>(a + offset * 2.0 / view.style.xy, 0.0, 1.0);
    out.local = vec2<f32>(along, corner.y * radius);
    out.segment_length = len;
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let cap = max(max(-in.local.x, in.local.x - in.segment_length), 0.0);
    let distance = length(vec2<f32>(cap, in.local.y)) - view.style.z;
    let alpha = clamp(0.5 - distance, 0.0, 1.0) * in.color.a * view.style.w;
    return vec4<f32>(in.color.rgb * alpha, alpha);
}
