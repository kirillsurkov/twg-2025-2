use std::f32::consts::FRAC_PI_2;

use bevy::{
    core_pipeline::prepass::DepthPrepass, prelude::*,
    render::experimental::occlusion_culling::OcclusionCulling,
};
use bevy_flycam::{FlyCam, NoCameraPlayerPlugin};

use crate::level::{Level, LevelBiome, LevelPart, LevelPartBuilder, PartAlign};

mod level;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(NoCameraPlayerPlugin)
        .add_systems(Startup, setup)
        .run();
}

const BASE_WIDTH: f32 = 120.0;
const BASE_HEIGHT: f32 = 120.0;

fn area_start() -> LevelPart {
    LevelPartBuilder::new(LevelBiome::Red)
        .with_size(BASE_WIDTH, BASE_HEIGHT)
        .with_count(5)
        .with_fill_ratio(0.2)
        .build()
}

fn area_safe() -> LevelPart {
    LevelPartBuilder::new(LevelBiome::Green)
        .with_size(BASE_WIDTH, BASE_HEIGHT * 0.1)
        .with_count(1)
        .with_fill_ratio(1.0)
        .with_points(vec![
            Vec2::new(-BASE_WIDTH * 0.4, -BASE_HEIGHT * 0.1 * 0.4),
            Vec2::new(-BASE_WIDTH * 0.4, 0.0),
            Vec2::new(BASE_WIDTH * 0.4, 0.0),
            Vec2::new(BASE_WIDTH * 0.4, -BASE_HEIGHT * 0.1 * 0.4),
        ])
        .build()
}

fn area_center(number: usize) -> LevelPart {
    LevelPartBuilder::new(LevelBiome::Blue)
        .with_size(BASE_WIDTH, BASE_HEIGHT)
        .with_count(40)
        .with_fill_ratio(0.2)
        .build()
}

fn area_left(number: usize) -> LevelPart {
    LevelPartBuilder::new(LevelBiome::Cyan)
        .with_size(BASE_WIDTH * 0.25, BASE_HEIGHT)
        .with_count(20)
        .with_fill_ratio(0.2)
        .build()
}

fn area_right(number: usize) -> LevelPart {
    LevelPartBuilder::new(LevelBiome::Magenta)
        .with_size(BASE_WIDTH * 0.25, BASE_HEIGHT)
        .with_count(20)
        .with_fill_ratio(0.2)
        .build()
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut level = Level::new();
    let mut id = level.add(Vec2::ZERO, area_start());
    for i in 1..=6 {
        id = level.add_after(id, PartAlign::Down, area_center(i));
        level.add_after(id, PartAlign::Left, area_left(i));
        level.add_after(id, PartAlign::Right, area_right(i));
        id = level.add_after(id, PartAlign::Down, area_safe());
    }

    let terrain = level.terrain(8.0);

    let mut material = StandardMaterial::from_color(Color::WHITE);
    // material.double_sided = true;
    // material.cull_mode = None;

    println!("{:?}", level.bounds().center());

    for chunk in terrain {
        commands.spawn((
            Mesh3d(meshes.add(chunk)),
            MeshMaterial3d(materials.add(material.clone())),
            Transform::default(),
        ));
    }

    for (part, point) in level.points() {
        commands.spawn((
            Mesh3d(meshes.add(Circle::new(0.25))),
            MeshMaterial3d(materials.add(Color::BLACK)),
            Transform::from_xyz(point.x, 0.02, point.y)
                .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
        ));
        commands.spawn((
            PointLight {
                intensity: 2000.0 * part.radius.powf(3.0),
                range: part.radius * 2.0,
                ..Default::default()
            },
            Transform::from_xyz(point.x, part.radius, point.y),
        ));
    }

    for (start, end) in level.edges() {
        let mid = (start + end) * 0.5;
        let distance = start.distance(end);
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::new(1.0, 0.05)))),
            MeshMaterial3d(materials.add(Color::BLACK)),
            Transform::from_xyz(mid.x as f32, 0.01, mid.y as f32)
                .with_scale(Vec3::new(distance * 0.5, 1.0, 1.0))
                .with_rotation(Quat::from_rotation_y((end - start).angle_to(Vec2::X))),
        ));
    }

    commands.spawn((
        Camera3d::default(),
        FlyCam,
        Transform::from_xyz(-2.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        DepthPrepass,
        OcclusionCulling,
    ));
}
