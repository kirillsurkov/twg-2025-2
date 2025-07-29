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
@group(2) @binding(103) var biome_mask: texture_2d_array<f32>;
@group(2) @binding(104) var biome_mask_sampler: sampler;
@group(2) @binding(105) var albedo: texture_2d_array<f32>;
@group(2) @binding(106) var albedo_sampler: sampler;
@group(2) @binding(107) var roughness: texture_2d_array<f32>;
@group(2) @binding(108) var roughness_sampler: sampler;
@group(2) @binding(109) var normal: texture_2d_array<f32>;
@group(2) @binding(110) var normal_sampler: sampler;

struct Biome {
    safe: f32,
    home: f32,
    forest: f32,
    cave: f32,
    ice: f32,
    temple: f32,
    boss: f32,
}

fn sample_biome(uv: vec2<f32>) -> Biome {
    var out: Biome;
    out.safe   = textureSample(biome_mask, biome_mask_sampler, uv, 0).r;
    out.home   = textureSample(biome_mask, biome_mask_sampler, uv, 1).r;
    out.forest = textureSample(biome_mask, biome_mask_sampler, uv, 2).r;
    out.cave   = textureSample(biome_mask, biome_mask_sampler, uv, 3).r;
    out.ice    = textureSample(biome_mask, biome_mask_sampler, uv, 4).r;
    out.temple = textureSample(biome_mask, biome_mask_sampler, uv, 5).r;
    out.boss   = textureSample(biome_mask, biome_mask_sampler, uv, 6).r;
    return out;
}

const BRICKS: u32 = 0;
const DIRT:   u32 = 1;
const GRASS:  u32 = 2;
const GUTS:   u32 = 3;
const STONE:  u32 = 4;
const TILES:  u32 = 5;

struct PbrPixel {
    albedo: vec4<f32>,
    roughness: f32,
    normal: vec3<f32>,
}

fn pbr_new() -> PbrPixel {
    var out: PbrPixel;
    out.albedo = vec4(0.0);
    out.roughness = 0.0;
    out.normal = vec3(0.0);
    return out;
}

fn pbr_add(left: PbrPixel, right: PbrPixel) -> PbrPixel {
    var out: PbrPixel;
    out.albedo = left.albedo + right.albedo;
    out.roughness = left.roughness + right.roughness;
    out.normal = left.normal + right.normal;
    return out;
}

fn pbr_mul(left: PbrPixel, right: f32) -> PbrPixel {
    var out: PbrPixel;
    out.albedo = left.albedo * right;
    out.roughness = left.roughness * right;
    out.normal = left.normal * right;
    return out;
}

fn mix_pbr(p1: PbrPixel, p2: PbrPixel, x: f32) -> PbrPixel {
    var out: PbrPixel;
    out.albedo = mix(p1.albedo, p2.albedo, x);
    out.roughness = mix(p1.roughness, p2.roughness, x);
    out.normal = mix(p1.normal, p2.normal, x);
    return out;
}

fn boxmap(t: texture_2d_array<f32>, s: sampler, layer: u32, p: vec3<f32>, n: vec3<f32>, k: f32) -> vec4<f32> {
	let x = textureSample(t, s, p.yz, layer);
	let y = textureSample(t, s, p.zx, layer);
	let z = textureSample(t, s, p.xy, layer);
    let m = pow(abs(n), vec3(k));
	return (x*m.x + y*m.y + z*m.z) / (m.x + m.y + m.z);
}

fn sample_texture(texture: u32, p: vec3<f32>, n: vec3<f32>) -> PbrPixel {
    var out: PbrPixel;
    out.albedo = boxmap(albedo, albedo_sampler, texture, p, n, 8.0);
    out.roughness = boxmap(roughness, roughness_sampler, texture, p, n, 8.0).r;
    out.normal = boxmap(normal, normal_sampler, texture, p, n, 8.0).xyz;
    return out;
}

fn grad3(height: f32,
    p1: PbrPixel, h1: f32,
    p2: PbrPixel, h2: f32,
    p3: PbrPixel, h3: f32,
) -> PbrPixel {
    let w1 = 1.0 - smoothstep(h1, h2, height);
    let w2 = smoothstep(h1, h2, height) * (1.0 - smoothstep(h2, h3, height));
    let w3 = smoothstep(h2, h3, height);

    var out: PbrPixel;
    out.albedo = p1.albedo * w1 + p2.albedo * w2 + p3.albedo * w3;
    out.roughness = p1.roughness * w1 + p2.roughness * w2 + p3.roughness * w3;
    out.normal = normalize(p1.normal * w1 + p2.normal * w2 + p3.normal * w3);

    return out;
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let height = in.world_position.y;
    let pos = in.world_position.xyz / 8.0;

    let time = globals.time / 10.0;
    let shift = vec3(cos(time), 0.0, sin(time));

    let bricks = sample_texture(BRICKS, pos, in.world_normal);
    let dirt   = sample_texture(DIRT,   pos, in.world_normal);
    let grass  = sample_texture(GRASS,  pos, in.world_normal);
    let guts   = sample_texture(GUTS,   pos + shift, in.world_normal);
    let stone  = sample_texture(STONE,  pos, in.world_normal);
    let tiles  = sample_texture(TILES,  pos, in.world_normal);

    let home   = grad3(height, tiles, 0.0, bricks, 0.1, grass, 3.0);
    let safe   = grad3(height, tiles, 0.0, grass, 0.1, stone, 20.0);
    let forest = grad3(height, dirt, 0.0, grass, 0.1, stone, 20.0);
    let cave   = grad3(height, guts, 0.0, guts, 0.1, stone, 20.0);

    let biome = sample_biome(in.uv);

    var pixel = pbr_new();
    pixel = pbr_add(pixel, pbr_mul(home, biome.home));
    pixel = pbr_add(pixel, pbr_mul(safe, biome.safe));
    pixel = pbr_add(pixel, pbr_mul(forest, biome.forest));
    pixel = pbr_add(pixel, pbr_mul(cave, biome.cave));

    pbr_input.material.base_color = pixel.albedo;
    pbr_input.material.perceptual_roughness = pixel.roughness;

    let TBN = fns::calculate_tbn_mikktspace(in.world_normal, in.world_tangent);
    pbr_input.N = fns::apply_normal_mapping(
        pbr_input.material.flags,
        TBN,
        true,
        is_front,
        pixel.normal,
    );

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
