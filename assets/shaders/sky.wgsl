#import bevy_pbr::{forward_io::VertexOutput, mesh_view_bindings::view}
@group(2) @binding(0) var<uniform> sun: vec4<f32>;

fn hash(p: vec3<f32>) -> f32 { return fract(sin(dot(p,vec3<f32>(127.1,311.7,74.7)))*43758.5453); }

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let d = normalize(in.world_position.xyz-view.world_position);
    let day = clamp(sun.y*3.0+0.25,0.0,1.0);
    let dusk = clamp(1.0-abs(sun.y)*5.0,0.0,1.0);
    let horizon = pow(1.0-abs(d.y),4.0);
    var color = mix(vec3<f32>(0.004,0.006,0.02),vec3<f32>(0.15,0.35,0.68),day);
    color = mix(color,mix(vec3<f32>(0.02,0.025,0.065),vec3<f32>(0.65,0.72,0.86),day),horizon);
    color += vec3<f32>(0.65,0.15,0.23)*horizon*dusk;
    let sun_disc = smoothstep(0.9990,0.9996,dot(d,sun.xyz));
    let halo = pow(max(dot(d,sun.xyz),0.0),96.0);
    color += vec3<f32>(4.0,2.8,1.5)*sun_disc + vec3<f32>(0.7,0.3,0.15)*halo;
    color += vec3<f32>(0.85,0.95,1.3)*smoothstep(0.9990,0.9996,dot(d,-sun.xyz))*(1.0-day);
    let cell = floor(d*320.0);
    let star = step(0.996,hash(cell))*pow(max(0.0,1.0-length(fract(d*320.0)-0.5)*2.0),3.0);
    color += vec3<f32>(1.0,0.85,0.95)*star*(1.0-day)*smoothstep(0.0,0.25,d.y)*3.0;
    // Soft, slowly drifting cloud bands above the horizon.
    let p = d.xz/max(d.y,0.12)*2.0 + vec2<f32>(sun.w*0.003,0.0);
    let cloud = sin(p.x+sin(p.y*1.3))*0.5 + sin(p.y*1.7+p.x*0.4)*0.3 + sin(p.x*3.1-p.y*2.0)*0.15;
    let cover = smoothstep(0.2,0.65,cloud)*smoothstep(0.05,0.3,d.y)*0.65;
    color = mix(color,mix(vec3<f32>(0.04,0.05,0.10),vec3<f32>(0.93,0.82,0.88),day),cover);
    return vec4<f32>(color,1.0);
}
