// Qualitative divergent CT projection. Integrated quantity: relative-density mm.
struct Parameters {
    source: vec4<f32>,
    origin: vec4<f32>,
    right: vec4<f32>,
    down: vec4<f32>,
    detector: vec4<f32>,
    quality: vec4<f32>,
    display: vec4<f32>,
}
@group(0) @binding(0) var<uniform> p: Parameters;
@group(0) @binding(1) var volume: texture_3d<f32>;
@group(1) @binding(0) var integrated: texture_2d<f32>;

struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
@vertex fn vertex(@builtin(vertex_index) index: u32) -> Vertex {
    let positions = array<vec2<f32>,3>(vec2(-1.,1.),vec2(-1.,-3.),vec2(3.,1.));
    var out: Vertex;
    let q=positions[index];
    out.position=vec4(q,0.,1.);
    out.uv=vec2((q.x+1.)*0.5,(1.-q.y)*0.5);
    return out;
}

fn scalar(index: vec3<i32>) -> f32 {
    let limits=vec3<i32>(textureDimensions(volume))-vec3(1);
    let value=textureLoad(volume,clamp(index,vec3(0),limits),0);
    let decoded=decode_volume_sample(value,p.right.w,p.down.w);
    return select(decoded,round(decoded),p.right.w>0.5);
}

fn sample_hu(uvw: vec3<f32>) -> f32 {
    let voxel=uvw*vec3<f32>(textureDimensions(volume))-vec3(0.5);
    let base=vec3<i32>(floor(voxel));
    let f=fract(voxel);
    let a=mix(scalar(base),scalar(base+vec3(1,0,0)),f.x);
    let b=mix(scalar(base+vec3(0,1,0)),scalar(base+vec3(1,1,0)),f.x);
    let c=mix(scalar(base+vec3(0,0,1)),scalar(base+vec3(1,0,1)),f.x);
    let d=mix(scalar(base+vec3(0,1,1)),scalar(base+vec3(1,1,1)),f.x);
    return mix(mix(a,b,f.y),mix(c,d,f.y),f.z);
}

@fragment fn integrate(in: Vertex) -> @location(0) f32 {
    let detector=p.origin.xyz+p.right.xyz*in.uv.x+p.down.xyz*in.uv.y;
    let direction=detector-p.source.xyz;
    var near=0.;
    var far=1e9;
    for(var axis=0u;axis<3u;axis++) {
        if abs(direction[axis])<1e-10 {
            if p.source[axis]<0. || p.source[axis]>1. { return 0.; }
        } else {
            let a=-p.source[axis]/direction[axis];
            let b=(1.-p.source[axis])/direction[axis];
            near=max(near,min(a,b));
            far=min(far,max(a,b));
        }
    }
    if far<=near { return 0.; }
    let detector_mm=p.detector.xy+p.detector.zw*in.uv;
    let ray_mm=sqrt(dot(detector_mm,detector_mm)+p.source.w*p.source.w);
    let length_mm=(far-near)*ray_mm;
    // If the budget is reached, increase step length; never truncate the ray.
    let count=u32(clamp(ceil(length_mm/p.origin.w),1.,p.quality.x));
    let dt=(far-near)/f32(count);
    var total=0.;
    for(var i=0u;i<count;i++) {
        let t=near+(f32(i)+0.5)*dt;
        let hu=sample_hu(p.source.xyz+direction*t);
        total+=max(0.,(hu+1000.)/1000.);
    }
    return total*(length_mm/f32(count));
}

@fragment fn display(in: Vertex) -> @location(0) vec4<f32> {
    let size=vec2<i32>(textureDimensions(integrated));
    let pixel=clamp(vec2<i32>(in.position.xy),vec2(0),size-vec2(1));
    let length_mm=textureLoad(integrated,pixel,0).r;
    let gray=1.-exp(-length_mm*p.display.x);
    return vec4(vec3(gray),1.);
}
