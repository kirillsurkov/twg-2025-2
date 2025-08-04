#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    mesh_view_bindings::globals,
    pbr_functions::alpha_discard,
    pbr_functions as fns,
}

#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}

@group(2) @binding(100) var<uniform> bounds: vec4<f32>;
@group(2) @binding(101) var lightmap: texture_2d<f32>;
@group(2) @binding(102) var lightmap_sampler: sampler;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);

    let bounds_min = bounds.xy;
    let bounds_max = bounds.zw;
    let rel_pos = (in.world_position.xz - bounds_min) / (bounds_max - bounds_min);
    out.color *= textureSample(lightmap, lightmap_sampler, rel_pos).r;

    return out;
}
