use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    math::bounding::Aabb3d,
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, Extent3d, ShaderRef, TextureDimension, TextureFormat},
    utils::Parallel,
};
use bevy_heightmap::mesh_builder::MeshBuilder;
use imageproc::image::{GenericImageView, imageops::FilterType};

use crate::{
    level::{BiomePixel, Level},
    player::Player,
};

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, TerrainMaterial>,
        >::default());
        app.add_systems(Update, init.run_if(resource_added::<Level>));
        app.add_systems(Update, init_chunks.after(init));
        app.add_systems(Update, update_lightmap.after(init));
        app.add_systems(Update, physics.after(init));
    }
}

#[derive(Component)]
struct Chunk;

impl Chunk {
    const MESH_SIZE: u32 = 128;
}

#[derive(Resource)]
struct Textures {
    albedo: Handle<Image>,
    roughness: Handle<Image>,
    normal: Handle<Image>,
}

#[derive(Resource)]
struct DynamicLightmap(Handle<Image>);

impl DynamicLightmap {
    fn new(images: &mut Assets<Image>, size: UVec2) -> Self {
        let image = Image::new_fill(
            Extent3d {
                width: size.x,
                height: size.y,
                ..Default::default()
            },
            TextureDimension::D2,
            &[0],
            TextureFormat::R32Float,
            RenderAssetUsages::all(),
        );

        Self(images.add(image))
    }
}

impl Textures {
    fn new(images: &mut Assets<Image>, names: &[&str]) -> Self {
        Self {
            albedo: images.add(Self::load_as_array(
                "albedo",
                TextureFormat::Rgba8UnormSrgb,
                names,
            )),
            roughness: images.add(Self::load_as_array(
                "roughness",
                TextureFormat::R8Unorm,
                names,
            )),
            normal: images.add(Self::load_as_array(
                "normal",
                TextureFormat::Rgba8Unorm,
                names,
            )),
        }
    }

    fn load_as_array(part: &str, texture_format: TextureFormat, names: &[&str]) -> Image {
        let base_path = "./assets/textures";
        let mut data = vec![];
        let mut size = (0, 0);
        let mut mip_levels = 0;
        let mut count = 0;

        for name in names {
            let path = format!("{base_path}/{name}/{part}.png");
            let image = imageproc::image::open(&path).unwrap();
            let cur_size = image.dimensions();
            if count == 0 {
                size = cur_size;
                mip_levels = (size.0.min(size.1) as f32).log2() as u32 + 1;
            } else if size != cur_size {
                panic!(
                    "Textures should be the same size. {path} differs. {cur_size:?} != {size:?}",
                );
            }
            for mip in 0..mip_levels {
                let path = format!("{base_path}/{name}/{part}.{mip}.png");
                let image = match (mip, imageproc::image::open(&path)) {
                    (0, _) => image.clone(),
                    (_, Ok(image)) => image,
                    (_, _) => {
                        let image = image.resize_exact(
                            image.width() / 2u32.pow(mip),
                            image.height() / 2u32.pow(mip),
                            FilterType::Lanczos3,
                        );
                        image.save(path.clone()).unwrap();
                        println!("Created mip {path}");
                        image
                    }
                };
                data.extend(match texture_format.components() {
                    1 => image.into_luma8().into_vec(),
                    4 => image.into_rgba8().into_vec(),
                    channels @ _ => panic!("Unsupported channels: {channels} {path}"),
                });
            }
            count += 1;
        }

        let mut image = Image::new(
            Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: count,
            },
            TextureDimension::D2,
            data,
            texture_format,
            RenderAssetUsages::RENDER_WORLD,
        );

        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            min_filter: ImageFilterMode::Linear,
            mag_filter: ImageFilterMode::Linear,
            mipmap_filter: ImageFilterMode::Linear,
            ..Default::default()
        });

        image.texture_descriptor.mip_level_count = mip_levels;

        image
    }
}

#[derive(Component)]
pub struct Physics {
    pub radius: f32,
    pub speed: f32,
    pub hitbox: Aabb3d,
    pub move_vec: Vec2,
    pub look_to: Dir2,
    pub ignore_overlap: bool,
}

impl Physics {
    pub fn new(radius: f32, speed: f32, hitbox: Aabb3d) -> Self {
        Self {
            radius,
            speed,
            hitbox,
            move_vec: Vec2::ZERO,
            look_to: Dir2::NEG_Y,
            ignore_overlap: false,
        }
    }
}

fn init(mut commands: Commands, level: Res<Level>, mut images: ResMut<Assets<Image>>) {
    let texture_size = level.texture_size().as_uvec2();
    let chunk_size = UVec2::splat(Chunk::MESH_SIZE);
    let chunks_count = (texture_size / chunk_size) + (texture_size % chunk_size).min(UVec2::ONE);
    let starting_point = level.bounds().min;
    let scale = chunk_size.as_vec2() * level.bounds().size() / texture_size.as_vec2();

    commands.insert_resource(Textures::new(
        &mut *images,
        &["bricks", "dirt", "grass", "guts", "stone", "tiles"],
    ));

    commands.insert_resource(DynamicLightmap::new(&mut *images, texture_size / 8));

    let mut terrain = commands.spawn((
        Name::new("Terrain"),
        Transform::default(),
        Visibility::default(),
    ));

    for y in 0..chunks_count.y {
        for x in 0..chunks_count.x {
            let pos = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * scale + starting_point;
            terrain.with_child((
                Chunk,
                Name::new(format!("Chunk ({x} {y})")),
                Transform::from_translation(pos.extend(0.0).xzy())
                    .with_scale(scale.extend(1.0).xzy()),
                Visibility::default(),
            ));
        }
    }
}

fn init_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>,
    level: Res<Level>,
    textures: Res<Textures>,
    lightmap: Res<DynamicLightmap>,
    chunks: Query<(Entity, &Transform), Added<Chunk>>,
) {
    let mut meshes_queue = Parallel::<Vec<(Entity, Mesh, Image)>>::default();

    chunks.par_iter().for_each_init(
        || meshes_queue.borrow_local_mut(),
        |meshes, (entity, transform)| {
            let chunk_pos = transform.translation.xz();
            let chunk_scale = Chunk::MESH_SIZE as f32 * level.pixel_size();

            let mut builder = MeshBuilder::grid(UVec2::splat(Chunk::MESH_SIZE), &|mut pos| {
                pos.y = -pos.y;
                level.height(chunk_pos + pos * chunk_scale).max(0.0)
            });

            for [_, y, z] in &mut builder.positions {
                (*y, *z) = (*z, -*y);
            }

            let chunk_texture_size = (Chunk::MESH_SIZE * Chunk::MESH_SIZE) as usize;

            let biomes_total = BiomePixel::END_BIOME - BiomePixel::START_BIOME;
            let mut biome_mask = vec![0; chunk_texture_size * biomes_total];
            for y in 0..Chunk::MESH_SIZE {
                for x in 0..Chunk::MESH_SIZE {
                    let pos = UVec2::new(x, y).as_vec2() / (Chunk::MESH_SIZE - 1) as f32 - 0.5;
                    let biome = level.biome(chunk_pos + pos * chunk_scale).0;
                    let base_offset = (x + y * Chunk::MESH_SIZE) as usize;
                    for biome_idx in 0..biomes_total {
                        biome_mask[base_offset + chunk_texture_size * biome_idx] = (255.0
                            * biome[BiomePixel::START_BIOME + biome_idx].clamp(0.0, 1.0))
                            as u8;
                    }
                }
            }
            let biome_mask = Image::new(
                Extent3d {
                    width: Chunk::MESH_SIZE,
                    height: Chunk::MESH_SIZE,
                    depth_or_array_layers: biomes_total as u32,
                },
                TextureDimension::D2,
                biome_mask,
                TextureFormat::R8Unorm,
                RenderAssetUsages::RENDER_WORLD,
            );

            let mut mesh = builder.build();
            mesh.generate_tangents().unwrap();

            meshes.push((entity, mesh, biome_mask));
        },
    );

    for (entity, mesh, biome_mask) in meshes_queue.drain() {
        commands.entity(entity).insert((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(ExtendedMaterial {
                base: StandardMaterial::default(),
                extension: TerrainMaterial {
                    bounds: {
                        let bounds = level.bounds();
                        Vec4::new(bounds.min.x, bounds.min.y, bounds.max.x, bounds.max.y)
                    },
                    lightmap: lightmap.0.clone_weak(),
                    biome_mask: images.add(biome_mask),
                    albedo: textures.albedo.clone_weak(),
                    roughness: textures.roughness.clone_weak(),
                    normal: textures.normal.clone_weak(),
                },
            })),
        ));
    }
}

fn update_lightmap(
    level: Res<Level>,
    lightmap: ResMut<DynamicLightmap>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>,
    player: Single<&Transform, With<Player>>,
    time: Res<Time>,
) {
    let Some(lightmap) = images.get_mut(&lightmap.0) else {
        return;
    };

    for _ in materials.iter_mut() {}

    let radius = Vec2::splat(32.0) * lightmap.size_f32() / level.bounds().size();

    let pos = (player.translation.xz() - level.bounds().min) / level.bounds().size();
    let pos = pos.clamp(Vec2::ZERO, Vec2::ONE);
    let pos = (lightmap.size_f32() * pos).as_ivec2();

    for x in (pos.x - radius.x as i32)..=(pos.x + radius.x as i32) {
        for y in (pos.y - radius.y as i32)..=(pos.y + radius.y as i32) {
            let Ok(cur_val) = lightmap.get_color_at(x as u32, y as u32) else {
                continue;
            };

            let rel_pos = (IVec2::new(x, y) - pos).as_vec2();
            let dist2 = (rel_pos.powf(2.0) / radius.powf(2.0)).element_sum();

            if dist2 <= 1.0 {
                let cur_val = cur_val.to_linear().red.clamp(0.0, 1.0);
                let target_val = 1.0 - dist2.powf(1.0 / 2.0);
                let val = (cur_val + time.delta_secs() * 0.5).min(target_val.max(cur_val));
                lightmap
                    .set_color_at(x as u32, y as u32, Color::linear_rgba(val, val, val, 1.0))
                    .unwrap();
            }
        }
    }
}

fn physics(
    level: Res<Level>,
    time: Res<Time>,
    queries: Query<(Entity, &Physics)>,
    mut transforms: Query<&mut Transform>,
) {
    for (entity, physics) in queries {
        let speed = physics.move_vec.length().min(1.0) * physics.speed;
        let move_vec = physics.move_vec.normalize_or_zero();

        let pos_3d = transforms.get(entity).unwrap().translation;
        let mut desired_pos = pos_3d.xz() + move_vec * time.delta_secs() * speed;

        if !physics.ignore_overlap {
            if let Some((entity, _)) = level
                .nearest_creatures(2, desired_pos.extend(pos_3d.y).xzy())
                .into_iter()
                .nth(1)
            {
                let other_pos = transforms.get(entity).unwrap().translation.xz();
                let other_radius = queries.get(entity).unwrap().1.radius;
                let direction = desired_pos - other_pos;
                let penetration = (physics.radius + other_radius) - direction.length();
                desired_pos += direction.normalize_or_zero() * penetration.max(0.0) * 2.0;
            }
        }

        let penetration = physics.radius + level.height(desired_pos);
        desired_pos += level.normal_2d(desired_pos) * penetration.max(0.0);

        let mut transform = transforms.get_mut(entity).unwrap();
        transform.translation.x = desired_pos.x;
        transform.translation.z = desired_pos.y;

        transform.look_to(physics.look_to.extend(0.0).xzy(), Vec3::Y);
    }
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
struct TerrainMaterial {
    #[uniform(100)]
    bounds: Vec4,
    #[texture(101)]
    #[sampler(102)]
    lightmap: Handle<Image>,
    #[texture(103, dimension = "2d_array")]
    #[sampler(104)]
    biome_mask: Handle<Image>,
    #[texture(105, dimension = "2d_array")]
    #[sampler(106)]
    albedo: Handle<Image>,
    #[texture(107, dimension = "2d_array")]
    #[sampler(108)]
    roughness: Handle<Image>,
    #[texture(109, dimension = "2d_array")]
    #[sampler(110)]
    normal: Handle<Image>,
}

impl MaterialExtension for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain.wgsl".into()
    }
}
