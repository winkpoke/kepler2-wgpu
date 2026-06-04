struct VSOut {
    @builtin(position) pos: vec4<f32>
}

@group(0) @binding(0)
var<uniform> rotation: mat4x4<f32>;

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> VSOut {
    var out: VSOut;
    out.pos = rotation * vec4<f32>(position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 0.0, 1.0);
}
