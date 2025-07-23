use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use bevy_flycam::PlayerPlugin;

use crate::level::{Level, LevelBiome, LevelPartBuilder, PartAlign};

mod level;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PlayerPlugin)
        .add_systems(Startup, setup)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut level = Level::new();
    let id = level.add(
        Vec2::ZERO,
        LevelPartBuilder::new(LevelBiome::Red)
            .with_size(20.0, 20.0)
            .with_count(5)
            .with_fill_ratio(0.2)
            .build(),
    );
    let id0 = id;
    level.add_after(
        id,
        PartAlign::Left,
        LevelPartBuilder::new(LevelBiome::Green)
            .with_size(5.0, 20.0)
            .with_count(20)
            .with_fill_ratio(0.2)
            .build(),
    );
    let id = level.add_after(
        id,
        PartAlign::Right,
        LevelPartBuilder::new(LevelBiome::Blue)
            .with_size(10.0, 5.0)
            .with_count(10)
            .with_fill_ratio(0.2)
            .build(),
    );
    level.add_after(
        id,
        PartAlign::Down,
        LevelPartBuilder::new(LevelBiome::Cyan)
            .with_size(5.0, 5.0)
            .with_count(5)
            .with_fill_ratio(0.2)
            .build(),
    );
    let id = level.add_after(
        id0,
        PartAlign::Down,
        LevelPartBuilder::new(LevelBiome::Magenta)
            .with_size(20.0, 20.0)
            .with_count(25)
            .with_fill_ratio(0.2)
            .build(),
    );
    let id = level.add_after(
        id,
        PartAlign::Down,
        LevelPartBuilder::new(LevelBiome::Yellow)
            .with_size(20.0, 20.0)
            .with_count(25)
            .with_fill_ratio(0.2)
            .build(),
    );
    let id = level.add_after(
        id,
        PartAlign::Down,
        LevelPartBuilder::new(LevelBiome::Orange)
            .with_size(20.0, 20.0)
            .with_count(25)
            .with_fill_ratio(0.2)
            .build(),
    );
    let id = level.add_after(
        id,
        PartAlign::Down,
        LevelPartBuilder::new(LevelBiome::Purple)
            .with_size(20.0, 20.0)
            .with_count(25)
            .with_fill_ratio(0.2)
            .build(),
    );
    level.add_after(
        id,
        PartAlign::Down,
        LevelPartBuilder::new(LevelBiome::Red)
            .with_size(20.0, 20.0)
            .with_count(25)
            .with_fill_ratio(0.2)
            .build(),
    );

    let terrain = meshes.add(level.terrain(32.0));

    let mut material = StandardMaterial::from_color(Color::WHITE);
    material.double_sided = true;
    material.cull_mode = None;

    println!("{:?}", level.bounds().center());

    commands.spawn((
        Mesh3d(terrain),
        MeshMaterial3d(materials.add(material)),
        Transform::from_translation(level.bounds().center().extend(0.0).xzy())
            .with_scale(level.bounds().size().extend(1.0))
            .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
    ));

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
}
