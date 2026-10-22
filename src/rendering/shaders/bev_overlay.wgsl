struct View { detector:vec4<f32>,viewport:vec4<f32> }
@group(0) @binding(0) var<uniform> view:View;
struct Out { @builtin(position) pos:vec4<f32>,@location(0) color:vec4<f32>,@location(1) local:vec2<f32>,@location(2) style:vec4<f32> }
fn pixel(mm:vec2<f32>)->vec2<f32> {return (mm-view.detector.xy)/view.detector.zw*view.viewport.xy;}
@vertex fn vertex(@builtin(vertex_index) index:u32,@location(0) edge:vec4<f32>,@location(1) color:vec4<f32>,@location(2) style:vec4<f32>)->Out {
    let corners=array<vec2<f32>,6>(vec2(0.,-1.),vec2(1.,-1.),vec2(1.,1.),vec2(0.,-1.),vec2(1.,1.),vec2(0.,1.));
    let uv=corners[index];let a=pixel(edge.xy);let b=pixel(edge.zw);var p:vec2<f32>;var local=vec2(0.);
    if style.y>0.5 {p=mix(a,b,vec2(uv.x,(uv.y+1.)*0.5));}
    else {let delta=b-a;let len=max(length(delta),0.0001);let normal=vec2(-delta.y,delta.x)/len;let half_width=style.x*0.5+0.75;p=mix(a,b,uv.x)+normal*uv.y*half_width;local=vec2(uv.x*len,uv.y*half_width);}
    var out:Out;out.pos=vec4(p/view.viewport.xy*vec2(2.,-2.)+vec2(-1.,1.),0.,1.);out.color=color;out.local=local;out.style=style;return out;
}
@fragment fn fragment(in:Out)->@location(0) vec4<f32> {
    var alpha=in.color.a;
    if in.style.y<0.5 {if in.style.z>0. {if fract(in.local.x/in.style.z)>0.5 {discard;}}alpha*=1.-smoothstep(in.style.x*0.5-0.5,in.style.x*0.5+0.75,abs(in.local.y));}
    return vec4(in.color.rgb*alpha,alpha);
}
