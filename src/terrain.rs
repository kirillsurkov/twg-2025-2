use std::{
    f32::consts::SQRT_2,
    ops::{Add, Mul},
};

use bevy::{
    asset::RenderAssetUsages,
    pbr::Lightmap,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    utils::Parallel,
};
use bevy_heightmap::mesh_builder::MeshBuilder;
use imageproc::image::{DynamicImage, GrayImage, Luma, Rgb, RgbImage, imageops::FilterType};

use crate::{
    level::{BiomePixel, Level},
    player::Player,
};

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, setup.run_if(resource_added::<Level>));
        app.add_systems(Update, init_chunks.after(setup));
        app.add_systems(
            Update,
            update_lightmap.run_if(resource_exists::<Level>.and(resource_exists::<Textures>)),
        );
    }
}

#[derive(Component)]
struct Chunk(u32, u32);

impl Chunk {
    const MESH_SIZE: u32 = 48;
    const TEXTURE_SIZE: u32 = 128;
}

struct PbrPixel {
    albedo: Rgb<u8>,
    roughness: Luma<u8>,
    normal: Rgb<u8>,
}

impl Mul<f32> for PbrPixel {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            albedo: Rgb([
                (rhs * self.albedo[0] as f32).clamp(0.0, 255.0) as u8,
                (rhs * self.albedo[1] as f32).clamp(0.0, 255.0) as u8,
                (rhs * self.albedo[2] as f32).clamp(0.0, 255.0) as u8,
            ]),
            roughness: Luma([(rhs * self.roughness[0] as f32).clamp(0.0, 255.0) as u8]),
            normal: Rgb([
                (rhs * self.normal[0] as f32).clamp(0.0, 255.0) as u8,
                (rhs * self.normal[1] as f32).clamp(0.0, 255.0) as u8,
                (rhs * self.normal[2] as f32).clamp(0.0, 255.0) as u8,
            ]),
        }
    }
}

impl Mul<PbrPixel> for f32 {
    type Output = PbrPixel;

    fn mul(self, rhs: PbrPixel) -> Self::Output {
        rhs * self
    }
}

impl Add<PbrPixel> for PbrPixel {
    type Output = Self;

    fn add(self, rhs: PbrPixel) -> Self::Output {
        Self {
            albedo: Rgb([
                (rhs.albedo[0] as u32 + self.albedo[0] as u32).clamp(0, 255) as u8,
                (rhs.albedo[1] as u32 + self.albedo[1] as u32).clamp(0, 255) as u8,
                (rhs.albedo[2] as u32 + self.albedo[2] as u32).clamp(0, 255) as u8,
            ]),
            roughness: Luma([
                (rhs.roughness[0] as u32 + self.roughness[0] as u32).clamp(0, 255) as u8,
            ]),
            normal: Rgb([
                (rhs.normal[0] as u32 + self.normal[0] as u32).clamp(0, 255) as u8,
                (rhs.normal[1] as u32 + self.normal[1] as u32).clamp(0, 255) as u8,
                (rhs.normal[2] as u32 + self.normal[2] as u32).clamp(0, 255) as u8,
            ]),
        }
    }
}

struct PbrTexture {
    albedo: RgbImage,
    roughness: GrayImage,
    normal: RgbImage,
}

impl PbrTexture {
    fn new() -> Self {
        Self {
            albedo: RgbImage::new(Chunk::TEXTURE_SIZE, Chunk::TEXTURE_SIZE),
            roughness: GrayImage::new(Chunk::TEXTURE_SIZE, Chunk::TEXTURE_SIZE),
            normal: RgbImage::new(Chunk::TEXTURE_SIZE, Chunk::TEXTURE_SIZE),
        }
    }

    fn load(name: &str) -> Self {
        let base_path = "./assets/textures";
        Self {
            albedo: Self::load_texture(format!("{base_path}/{name}/albedo.png")).into(),
            roughness: Self::load_texture(format!("{base_path}/{name}/roughness.png")).into(),
            normal: Self::load_texture(format!("{base_path}/{name}/normal.png")).into(),
        }
    }

    fn load_texture<S: AsRef<str>>(path: S) -> DynamicImage {
        imageproc::image::open(path.as_ref()).unwrap().resize_exact(
            Chunk::TEXTURE_SIZE,
            Chunk::TEXTURE_SIZE,
            FilterType::Gaussian,
        )
    }

    fn get_pixel(&self, x: u32, y: u32) -> PbrPixel {
        PbrPixel {
            albedo: *self.albedo.get_pixel(x, y),
            roughness: *self.roughness.get_pixel(x, y),
            normal: *self.normal.get_pixel(x, y),
        }
    }

    fn put_pixel(&mut self, x: u32, y: u32, pixel: PbrPixel) {
        self.albedo.put_pixel(x, y, pixel.albedo);
        self.roughness.put_pixel(x, y, pixel.roughness);
        self.normal.put_pixel(x, y, pixel.normal);
    }

    fn gradient(x: u32, y: u32, value: f32, textures: &[(&Self, f32)]) -> PbrPixel {
        let index = match textures.binary_search_by(|(_, f)| f.partial_cmp(&value).unwrap()) {
            Ok(index) => index,
            Err(0) => 0,
            Err(index) if index >= textures.len() => textures.len() - 2,
            Err(index) => index - 1,
        };

        let start = &textures[index];
        let end = &textures[index + 1];
        let range = end.1 - start.1;
        let current = if range <= f32::EPSILON {
            0.0
        } else {
            ((value - start.1) / range).clamp(0.0, 1.0)
        };

        let start = start.0.get_pixel(x, y);
        let end = end.0.get_pixel(x, y);

        start * (1.0 - current) + end * current
    }
}

#[derive(Resource)]
struct Textures {
    grass: PbrTexture,
    dirt: PbrTexture,
    stone: PbrTexture,
    tiles: PbrTexture,
    bricks: PbrTexture,
    guts: PbrTexture,
    light_map: RgbImage,
}

impl Textures {
    fn new(light_map: RgbImage) -> Self {
        Self {
            grass: PbrTexture::load("grass"),
            dirt: PbrTexture::load("dirt"),
            stone: PbrTexture::load("stone"),
            tiles: PbrTexture::load("tiles"),
            bricks: PbrTexture::load("bricks"),
            guts: PbrTexture::load("guts"),
            light_map,
        }
    }
}

fn setup(mut commands: Commands, level: Res<Level>, mut images: ResMut<Assets<Image>>) {
    let texture_size = UVec2::from(level.height_map().dimensions());
    let chunk_size = UVec2::splat(Chunk::MESH_SIZE);
    let chunks_count = (texture_size / chunk_size) + (texture_size % chunk_size).min(UVec2::ONE);
    let starting_point = level.bounds().min;
    let scale = chunk_size.as_vec2() * level.bounds().size() / texture_size.as_vec2();

    commands.insert_resource(Textures::new(RgbImage::new(chunks_count.x, chunks_count.y)));

    let mut terrain = commands.spawn((
        Name::new("Terrain"),
        Transform::default(),
        Visibility::default(),
    ));

    for y in 0..chunks_count.y {
        for x in 0..chunks_count.x {
            let pos = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * scale + starting_point;
            let uv_rect = Rect {
                min: UVec2::new(x, y).as_vec2() / chunks_count.as_vec2(),
                max: UVec2::new(x + 1, y + 1).as_vec2() / chunks_count.as_vec2(),
            };
            terrain.with_child((
                Name::new(format!("Chunk ({x} {y})")),
                Chunk(x, y),
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
    mut materials: ResMut<Assets<StandardMaterial>>,
    level: Res<Level>,
    textures: Res<Textures>,
    chunks: Query<(Entity, &Chunk), Added<Chunk>>,
) {
    let mut meshes_queue = Parallel::<Vec<(Entity, Mesh, PbrTexture)>>::default();

    chunks.par_iter().for_each_init(
        || meshes_queue.borrow_local_mut(),
        |meshes, (entity, Chunk(chunk_x, chunk_y))| {
            let UVec2 {
                x: chunk_x,
                y: chunk_y,
            } = UVec2::new(*chunk_x, *chunk_y) * Chunk::MESH_SIZE;

            let height_map = level.height_map();
            let biome_map = level.biome_map();
            let texture_size = UVec2::from(height_map.dimensions());

            let mut builder =
                MeshBuilder::grid(UVec2::splat(Chunk::MESH_SIZE), &|Vec2 { x, y }| {
                    let Vec2 { x, y } = Vec2::new(0.5 + x, 0.5 - y);
                    let UVec2 { x, y } = (Vec2::new(x, y) * Chunk::MESH_SIZE as f32).as_uvec2();
                    let UVec2 { x, y } = UVec2::new(chunk_x + x, chunk_y + y).min(texture_size - 1);
                    height_map.get_pixel(x, y).0[0]
                });

            for [_, y, z] in &mut builder.positions {
                (*y, *z) = (*z, -*y);
            }

            let mut pbr_texture = PbrTexture::new();

            let texture_map_scale = Chunk::MESH_SIZE as f32 / Chunk::TEXTURE_SIZE as f32;

            for y in 0..Chunk::TEXTURE_SIZE {
                for x in 0..Chunk::TEXTURE_SIZE {
                    let map_pos = UVec2::new(
                        (chunk_x as f32 + x as f32 * texture_map_scale) as u32,
                        (chunk_y as f32 + y as f32 * texture_map_scale) as u32,
                    )
                    .min(texture_size - 1);

                    let biome = biome_map.get_pixel(map_pos.x, map_pos.y).0;
                    let height = height_map.get_pixel(map_pos.x, map_pos.y).0[0];

                    let home = PbrTexture::gradient(
                        x,
                        y,
                        height,
                        &[
                            (&textures.tiles, 0.0),
                            (&textures.bricks, 0.1),
                            (&textures.grass, 3.0),
                        ],
                    );

                    let safe = PbrTexture::gradient(
                        x,
                        y,
                        height,
                        &[
                            (&textures.tiles, 0.0),
                            (&textures.grass, 0.1),
                            (&textures.stone, 20.0),
                        ],
                    );

                    let forest = PbrTexture::gradient(
                        x,
                        y,
                        height,
                        &[
                            (&textures.dirt, 0.0),
                            (&textures.grass, 0.1),
                            (&textures.stone, 20.0),
                        ],
                    );

                    let cave = PbrTexture::gradient(
                        x,
                        y,
                        height,
                        &[
                            (&textures.dirt, 0.0),
                            (&textures.guts, 0.1),
                            (&textures.stone, 20.0),
                        ],
                    );

                    pbr_texture.put_pixel(
                        x,
                        y,
                        biome[BiomePixel::AREA_HOME] * home
                            + biome[BiomePixel::AREA_SAFE] * safe
                            + biome[BiomePixel::AREA_FOREST] * forest
                            + biome[BiomePixel::AREA_CAVE] * cave,
                    );
                }
            }

            let uv_0 = builder.uvs.clone();
            let mut mesh = builder.build();
            mesh.generate_tangents().unwrap();
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, uv_0);

            meshes.push((entity, mesh, pbr_texture));
        },
    );

    for (entity, mesh, pbr_texture) in meshes_queue.drain() {
        commands.entity(entity).insert((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color_texture: Some(images.add(Image::from_dynamic(
                    pbr_texture.albedo.into(),
                    true,
                    RenderAssetUsages::RENDER_WORLD,
                ))),
                metallic_roughness_texture: Some(images.add(Image::from_dynamic(
                    pbr_texture.roughness.into(),
                    false,
                    RenderAssetUsages::RENDER_WORLD,
                ))),
                normal_map_texture: Some(images.add(Image::from_dynamic(
                    pbr_texture.normal.into(),
                    false,
                    RenderAssetUsages::RENDER_WORLD,
                ))),
                unlit: true,
                ..Default::default()
            })),
        ));
    }
}

fn update_lightmap(
    level: Res<Level>,
    mut textures: ResMut<Textures>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player: Single<&Transform, With<Player>>,
    chunks: Query<(&Chunk, &MeshMaterial3d<StandardMaterial>)>,
    time: Res<Time>,
) {
    let pos = ((player.translation.xz() - level.bounds().min) / level.bounds().size())
        .clamp(Vec2::ZERO, Vec2::ONE);
    let pos = (UVec2::from(textures.light_map.dimensions()).as_vec2() * pos).as_uvec2();

    for (x, y) in [
        (pos.x as i32 - 1, pos.y as i32 - 1),
        (pos.x as i32 - 1, pos.y as i32),
        (pos.x as i32 - 1, pos.y as i32 + 1),
        (pos.x as i32, pos.y as i32 - 1),
        (pos.x as i32, pos.y as i32),
        (pos.x as i32, pos.y as i32 + 1),
        (pos.x as i32 + 1, pos.y as i32 - 1),
        (pos.x as i32 + 1, pos.y as i32),
        (pos.x as i32 + 1, pos.y as i32 + 1),
    ] {
        let dist = IVec2::new(x, y)
            .as_vec2()
            .distance(UVec2::new(pos.x, pos.y).as_vec2())
            / SQRT_2;
        if x >= 0
            && x < textures.light_map.width() as i32
            && y >= 0
            && y < textures.light_map.height() as i32
        {
            let brightness = 255;//(255.0 * (1.0 - 0.5 * dist)) as u8;
            textures.light_map.put_pixel(
                x as u32,
                y as u32,
                Rgb([brightness, brightness, brightness]),
            );
        }
    }

    let mut to_change = Parallel::<Vec<(Handle<StandardMaterial>, f32)>>::default();
    chunks.par_iter().for_each_init(
        || to_change.borrow_local_mut(),
        |to_change, (Chunk(x, y), handle)| {
            let brightness = textures.light_map.get_pixel(*x, *y).0[0] as f32 / 255.0;
            let brightness = brightness * 0.5;
            let material = materials.get(handle).unwrap();

            let current_brightness = {
                let color = material.base_color.to_srgba();
                (color.red + color.green + color.blue) / 3.0
            };

            let brightness = (current_brightness + time.delta_secs() * 2.0)
                .max(0.0)
                .min(brightness);

            if brightness != current_brightness {
                to_change.push((handle.clone_weak(), brightness));
            }
        },
    );

    for (material, brightness) in to_change.drain() {
        materials.get_mut(&material).unwrap().base_color =
            Color::srgba(brightness, brightness, brightness, 1.0);
    }

    // println!("{}", 1.0 / time.delta_secs_f64());
}
