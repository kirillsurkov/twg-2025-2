use bevy::{
    color::palettes::css::{AQUA, BLUE, GREEN, MAGENTA, ORANGE, PURPLE, RED, YELLOW},
    prelude::*,
    utils::Parallel,
};
use bevy_heightmap::mesh_builder::MeshBuilder;
use rand::distr::{Distribution, weighted::WeightedIndex};

use crate::level::{BiomePixel, Level};

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, setup.run_if(resource_added::<Level>));
        app.add_systems(Update, init_chunks);
    }
}

#[derive(Component)]
struct Chunk(u32, u32);

impl Chunk {
    const SIZE: u32 = 64;
}

fn setup(mut commands: Commands, level: Res<Level>) {
    let texture_size = UVec2::from(level.height_map().dimensions());
    let chunk_size = UVec2::splat(Chunk::SIZE);
    let chunks_count = (texture_size / chunk_size) + (texture_size % chunk_size).min(UVec2::ONE);
    let starting_point = level.bounds().min;
    let scale = chunk_size.as_vec2() * level.bounds().size() / texture_size.as_vec2();

    let mut terrain = commands.spawn((
        Name::new("Terrain"),
        Transform::default(),
        Visibility::default(),
    ));

    for y in 0..chunks_count.y {
        for x in 0..chunks_count.x {
            let pos = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * scale + starting_point;
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
    mut materials: ResMut<Assets<StandardMaterial>>,
    level: Res<Level>,
    chunks: Query<(Entity, &Chunk), Added<Chunk>>,
) {
    let mut meshes_queue = Parallel::<Vec<(Entity, Mesh)>>::default();
    chunks.par_iter().for_each_init(
        || meshes_queue.borrow_local_mut(),
        |meshes, (entity, Chunk(chunk_x, chunk_y))| {
            let UVec2 {
                x: chunk_x,
                y: chunk_y,
            } = UVec2::new(*chunk_x, *chunk_y) * Chunk::SIZE;

            let height_map = level.height_map();
            let biome_map = level.biome_map();
            let texture_size = UVec2::from(height_map.dimensions());

            let mut builder = MeshBuilder::grid(UVec2::splat(Chunk::SIZE), &|Vec2 { x, y }| {
                let Vec2 { x, y } = Vec2::new(0.5 + x, 0.5 - y);
                let UVec2 { x, y } = (Vec2::new(x, y) * Chunk::SIZE as f32).as_uvec2();
                let UVec2 { x, y } = UVec2::new(chunk_x + x, chunk_y + y).min(texture_size - 1);
                height_map.get_pixel(x, y).0[0]
            });

            for [_, y, z] in &mut builder.positions {
                (*y, *z) = (*z, -*y);
            }

            let choices = [RED, GREEN, BLUE, AQUA, MAGENTA, YELLOW, ORANGE, PURPLE];
            let mut rng = rand::rng();
            let mut colors = vec![];
            for y in (0..Chunk::SIZE).rev() {
                for x in 0..Chunk::SIZE {
                    let UVec2 { x, y } = UVec2::new(chunk_x + x, chunk_y + y).min(texture_size - 1);
                    let pixel = biome_map.get_pixel(x, y);
                    let dist = WeightedIndex::new(&pixel.0[1..=8]).unwrap();
                    colors.push(choices[dist.sample(&mut rng)].to_f32_array());
                }
            }

            meshes.push((
                entity,
                builder
                    .build()
                    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors),
            ));
        },
    );

    for (entity, mesh) in meshes_queue.drain() {
        commands.entity(entity).insert((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(Color::WHITE)),
        ));
    }
}
