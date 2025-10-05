struct VSOut {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> VSOut {
    // Pass-through projected to clip space (for now assume already in clip space range)
    // Map [-1,1] positions directly; add w=1.
    return VSOut(vec4<f32>(position, 1.0));
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // Solid color for minimal pipeline
    return vec4<f32>(0.8, 0.4, 1.0, 1.0);
}