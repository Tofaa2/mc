#import bevy_pbr::{
    forward_io::{Vertex, VertexOutput, FragmentOutput},
    mesh_functions,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}

@group(2) @binding(100) var<uniform> sky_tint: vec4<f32>;

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let model = mesh_functions::get_world_from_local(v.instance_index);
    var p = mesh_functions::mesh_position_local_to_world(model, vec4<f32>(v.position, 1.0));
    p.y += sin(p.x * 1.2 + globals.time * 1.1) * 0.045
         + sin(p.z * 0.85 - globals.time * 0.8) * 0.035;
    out.world_position = p;
    out.world_normal = vec3<f32>(0.0, 1.0, 0.0);
    out.position = position_world_to_clip(p.xyz);
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = v.instance_index;
#endif
    return out;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) front: bool) -> FragmentOutput {
    var pbr = pbr_input_from_standard_material(in, front);
    let p = in.world_position.xyz;
    let t = globals.time;
    pbr.N = normalize(vec3<f32>(
        -0.054 * cos(p.x * 1.2 + t * 1.1) - 0.045 * cos(p.x * 7.0 + p.z * 4.0 + t * 2.0),
        1.0,
        -0.030 * cos(p.z * 0.85 - t * 0.8) - 0.035 * cos(p.z * 8.0 - p.x * 3.0 - t * 1.7)));
    let fresnel = pow(1.0 - max(dot(pbr.N, pbr.V), 0.0), 4.0);
    let ripple = sin(p.x * 3.0 + p.z * 5.0 + t * 1.5 + sin(p.x * 2.0 - t));
    let glint = smoothstep(0.94, 1.0, ripple) * 0.055;
    pbr.material.base_color = vec4<f32>(mix(vec3<f32>(0.06, 0.34, 0.40), sky_tint.rgb, fresnel * 0.7) + vec3<f32>(glint), 0.78 + fresnel * 0.2);
    pbr.material.emissive = vec4<f32>(sky_tint.rgb * fresnel * 0.18, 0.0);
    var out: FragmentOutput;
    out.color = main_pass_post_lighting_processing(pbr, apply_pbr_lighting(pbr));
    return out;
}
