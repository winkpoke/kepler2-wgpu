struct VSOut {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> VSOut {
    return VSOut(vec4<f32>(position, 1.0));
}

@fragment
fn fs_main() -> @builtin(frag_depth) f32 {
    // Write a constant depth for now; real implementation will compute
    return 0.5;
}