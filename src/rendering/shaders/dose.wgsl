struct Uniforms {
    // Clipped viewport UV to dose column index, row index and normal offset in mm.
    matrix: mat4x4<f32>,
    style: vec4<f32>, // lower, upper, cutoff, opacity
    shape: vec4<u32>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var grid: texture_3d<f32>;
@group(0) @binding(2) var<storage, read> offsets: array<f32>;
struct Vertex { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) i: u32) -> Vertex {
    let x = f32(i & 1u) * 4.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    var v: Vertex;
    v.position = vec4<f32>(x,y,0.0,1.0);
    v.uv = vec2<f32>((x+1.0)*0.5,(1.0-y)*0.5);
    return v;
}
fn at(x: u32,y: u32,z: u32) -> f32 { return textureLoad(grid,vec3<i32>(i32(x),i32(y),i32(z)),0).r; }
fn layer(p: vec2<f32>, z: u32) -> f32 {
    let a = vec2<u32>(floor(p));
    let b = min(a+vec2<u32>(1u),u.shape.xy-vec2<u32>(1u));
    let t = fract(p);
    return mix(mix(at(a.x,a.y,z),at(b.x,a.y,z),t.x),mix(at(a.x,b.y,z),at(b.x,b.y,z),t.x),t.y);
}
// Fixed perceptual sequence, shared numerically with the viewer legend.
fn color(t: f32) -> vec3<f32> {
    let a=vec3<f32>(0.05,0.1,0.8); let b=vec3<f32>(0.0,0.8,0.9);
    let c=vec3<f32>(1.0,0.9,0.0); let d=vec3<f32>(0.95,0.05,0.0);
    if t < 0.33333333 { return mix(a,b,t*3.0); }
    if t < 0.66666667 { return mix(b,c,t*3.0-1.0); }
    return mix(c,d,t*3.0-2.0);
}
@fragment fn fs_main(v: Vertex) -> @location(0) vec4<f32> {
    let p=(u.matrix*vec4<f32>(v.uv,0.0,1.0)).xyz;
    if any(p.xy<vec2<f32>(0.0)) || any(p.xy>vec2<f32>(u.shape.xy-vec2<u32>(1u))) || p.z<offsets[0] || p.z>offsets[u.shape.z-1u] { return vec4<f32>(0.0); }
    var lo=0u; var hi=u.shape.z-1u;
    loop {
        if hi-lo<=1u { break; }
        let mid=(lo+hi)/2u;
        if offsets[mid]<=p.z { lo=mid; } else { hi=mid; }
    }
    let t=(p.z-offsets[lo])/(offsets[hi]-offsets[lo]);
    let dose=mix(layer(p.xy,lo),layer(p.xy,hi),t);
    if dose<u.style.z { return vec4<f32>(0.0); }
    let value=clamp((dose-u.style.x)/(u.style.y-u.style.x),0.0,1.0);
    return vec4<f32>(color(value)*u.style.w,u.style.w);
}
