// Analytic intersection of a jaw-defined divergent beam with the displayed CT plane.
struct Uniforms {
    origin: vec4<f32>, right: vec4<f32>, down: vec4<f32>,
    beam_x: vec4<f32>, beam_y: vec4<f32>, beam_z: vec4<f32>,
    jaws: vec4<f32>, uv: vec4<f32>, screen: vec4<f32>, style: vec4<f32>,
}

struct ApertureVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) span: f32,
}
@vertex fn aperture_vertex(@builtin(vertex_index) index: u32, @location(0) endpoints: vec4<f32>) -> ApertureVertexOutput {
    let corners=array<vec2<f32>,6>(vec2<f32>(0.,-1.),vec2<f32>(1.,-1.),vec2<f32>(0.,1.),
        vec2<f32>(0.,1.),vec2<f32>(1.,-1.),vec2<f32>(1.,1.));
    let a=(endpoints.xy-u.uv.xy)/u.uv.zw*u.screen.xy;
    let b=(endpoints.zw-u.uv.xy)/u.uv.zw*u.screen.xy;
    let span=max(length(b-a),0.00001);
    let direction=(b-a)/span;
    let normal=vec2<f32>(-direction.y,direction.x);
    let corner=corners[index];
    let along=mix(-2.,span+2.,corner.x);
    let across=corner.y*2.;
    let pixel=a+direction*along+normal*across;
    var out:ApertureVertexOutput;
    out.position=vec4<f32>(pixel/u.screen.xy*vec2<f32>(2.,-2.)+vec2<f32>(-1.,1.),0.,1.);
    out.local=vec2<f32>(along,across);
    out.span=span;
    return out;
}
@fragment fn aperture_fragment(in:ApertureVertexOutput) -> @location(0) vec4<f32> {
    let cap=max(max(-in.local.x,in.local.x-in.span),0.);
    let distance=length(vec2<f32>(cap,in.local.y));
    let alpha=(1.-smoothstep(0.6,1.6,distance))*u.style.y;
    return vec4<f32>(mix(vec3<f32>(0.6,0.69,0.78),vec3<f32>(0.1,0.85,1.),u.style.w)*alpha,alpha);
}
@group(0) @binding(0) var<uniform> u: Uniforms;
struct VertexOutput { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vertex(@builtin(vertex_index) index: u32) -> VertexOutput {
    let xy = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: VertexOutput;
    out.position = vec4<f32>(xy * vec2<f32>(2., -2.) + vec2<f32>(-1., 1.), 0., 1.);
    out.uv = xy;
    return out;
}
fn uv_of(point: vec3<f32>) -> vec2<f32> {
    let p = point - u.origin.xyz;
    return vec2<f32>(dot(p, u.right.xyz) / dot(u.right.xyz, u.right.xyz), dot(p, u.down.xyz) / dot(u.down.xyz, u.down.xyz));
}
fn pixel_delta(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> { return (a-b) / u.uv.zw * u.screen.xy; }
@fragment fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = u.uv.xy + in.uv * u.uv.zw;
    let p = u.origin.xyz + uv.x * u.right.xyz + uv.y * u.down.xyz;
    let depth = u.beam_z.w - dot(p, u.beam_z.xyz);
    let factor = depth / u.beam_z.w;
    let x = dot(p, u.beam_x.xyz);
    let y = dot(p, u.beam_y.xyz);
    let sides = vec4<f32>(x-u.jaws.x*factor, u.jaws.y*factor-x, y-u.jaws.z*factor, u.jaws.w*factor-y);
    let field = min(min(sides.x, sides.y), min(sides.z, sides.w));
    // Derivatives are evaluated uniformly, including outside the geometric envelope.
    let edge = abs(field) / max(fwidth(field), 0.000001);
    var coverage = (1.-smoothstep(0.75, 1.5, edge)) * u.screen.z * u.style.z * select(0., 1., depth > 0.);
    var color = vec3<f32>(0.1, 0.85, 1.);
    let normal = normalize(cross(u.right.xyz, u.down.xyz));
    let distance = -dot(u.origin.xyz, normal);
    let iso_delta = pixel_delta(uv, uv_of(vec3<f32>(0.)));
    let on_slice = abs(distance) <= 0.01;
    var iso = 1.-smoothstep(0.8, 1.6, abs(length(iso_delta)-6.));
    if !on_slice {
        iso *= u.style.x * select(0., 1., sin(atan2(iso_delta.y, iso_delta.x)*6.) > 0.);
    }
    let forward = -u.beam_z.xyz;
    let denominator = dot(forward, normal);
    var axis = 0.;
    if abs(denominator) < 0.000001 {
        if (on_slice || u.style.x > 0.) && depth > 0. {
            let direction = normalize(vec2<f32>(dot(forward, u.right.xyz)/dot(u.right.xyz,u.right.xyz), dot(forward,u.down.xyz)/dot(u.down.xyz,u.down.xyz)) / u.uv.zw * u.screen.xy);
            let d = abs(iso_delta.x*direction.y - iso_delta.y*direction.x);
            let along = dot(iso_delta,direction);
            let arrow = abs(along+25.);
            let arrow_edge = abs(d-arrow*0.55);
            let arrow_coverage = (1.-smoothstep(0.6,1.4,arrow_edge))*select(0.,1.,along >= -35. && along <= -25.);
            axis = max(1.-smoothstep(0.6+u.style.w*0.5,1.4+u.style.w*0.5,d),arrow_coverage);
            if !on_slice {axis *= select(0.,1.,sin(along*0.5)>0.);}
        }
    } else {
        let source = u.beam_z.xyz*u.beam_z.w;
        let t = dot(u.origin.xyz-source, normal)/denominator;
        if t >= 0. {
            let point = source + t*forward;
            let delta = abs(pixel_delta(uv, uv_of(point)));
            axis = (1.-smoothstep(0.6,1.4,min(delta.x,delta.y))) * (1.-smoothstep(4.,5.,max(delta.x,delta.y)));
        }
    }
    axis *= u.screen.w;
    if axis > coverage { color = mix(vec3<f32>(0.6,0.69,0.78),vec3<f32>(1.,0.65,0.25),u.style.w); coverage = axis; }
    if iso > coverage { color = vec3<f32>(1.); coverage = iso; }
    let alpha = coverage * u.style.y;
    return vec4<f32>(color*alpha, alpha);
}
