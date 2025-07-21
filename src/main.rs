use bevy::{color::palettes::css::GREEN, prelude::*};

use crate::level::{Level, LevelPartBuilder, PartAlign};

mod level;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
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
        LevelPartBuilder::new()
            .with_size(20.0, 20.0)
            .with_count(20)
            .with_fill_ratio(0.2)
            .build(),
    );
    level.add_after(
        id,
        PartAlign::Left,
        LevelPartBuilder::new()
            .with_size(5.0, 20.0)
            .with_count(30)
            .with_fill_ratio(0.2)
            .build(),
    );
    let id = level.add_after(
        id,
        PartAlign::Right,
        LevelPartBuilder::new()
            .with_size(10.0, 5.0)
            .with_count(10)
            .with_fill_ratio(0.2)
            .build(),
    );
    level.add_after(
        id,
        PartAlign::Down,
        LevelPartBuilder::new()
            .with_size(5.0, 5.0)
            .with_count(5)
            .with_fill_ratio(0.2)
            .build(),
    );

    for point in level.points() {
        commands.spawn((
            Mesh3d(meshes.add(Circle::new(0.25))),
            MeshMaterial3d(materials.add(Color::WHITE)),
            Transform::from_xyz(point.x as f32, 0.0, point.y as f32)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ));
    }

    for (start, end) in level.edges() {
        let mid = (start + end) * 0.5;
        let distance = start.distance(end);
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::new(1.0, 0.05)))),
            MeshMaterial3d(materials.add(Color::WHITE)),
            Transform::from_xyz(mid.x as f32, 0.0, mid.y as f32)
                .with_scale(Vec3::new(distance * 0.5, 1.0, 1.0))
                .with_rotation(Quat::from_rotation_y((end - start).angle_to(Vec2::X))),
        ));
    }

    for bounds in level.bounds() {
        let center = bounds.center();
        let center = Vec3::new(center.x, -0.01, center.y);
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::new(Vec3::Y, bounds.size() * 0.5))),
            MeshMaterial3d(materials.add(StandardMaterial::from_color(GREEN))),
            Transform::from_translation(center),
        ));
    }

    // light
    commands.spawn((
        DirectionalLight {
            illuminance: 1000.0,
            ..Default::default()
        },
        Transform::default().looking_to(-Vec3::Y, Vec3::Y),
    ));

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 40.0, 0.0).looking_at(Vec3::ZERO, -Vec3::Z),
    ));
}
