use std::f32::consts::FRAC_PI_2;

use bevy::{
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use bevy_flycam::{FlyCam, NoCameraPlayerPlugin};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};

use crate::{
    level::{LevelBiome, LevelBuilder, LevelPart, LevelPartBuilder, PartAlign},
    player::{Player, PlayerPlugin},
    terrain::TerrainPlugin,
};

mod level;
mod player;
mod terrain;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::default())
        // .add_plugins(NoCameraPlayerPlugin)
        .add_plugins(PlayerPlugin)
        .add_plugins(TerrainPlugin)
        .insert_resource(AmbientLight {
            color: Color::BLACK,
            brightness: 0.0,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, setup)
        .add_systems(Update, grab_cursor)
        .run();
}

const BASE_WIDTH: f32 = 120.0;
const BASE_HEIGHT: f32 = 120.0;

fn area_start() -> LevelPart {
    LevelPartBuilder::new(LevelBiome::Home)
        .with_size(BASE_WIDTH, BASE_HEIGHT)
        .with_count(5)
        .with_fill_ratio(0.2)
        .build()
}

fn area_safe() -> LevelPart {
    LevelPartBuilder::new(LevelBiome::Safe)
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
    LevelPartBuilder::new(LevelBiome::Forest)
        .with_size(BASE_WIDTH, BASE_HEIGHT)
        .with_count(40)
        .with_fill_ratio(0.2)
        .build()
}

fn area_left(number: usize) -> LevelPart {
    LevelPartBuilder::new(LevelBiome::Cave)
        .with_size(BASE_WIDTH * 0.25, BASE_HEIGHT)
        .with_count(20)
        .with_fill_ratio(0.2)
        .build()
}

fn area_right(number: usize) -> LevelPart {
    LevelPartBuilder::new(LevelBiome::Cave)
        .with_size(BASE_WIDTH * 0.25, BASE_HEIGHT)
        .with_count(20)
        .with_fill_ratio(0.2)
        .build()
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    let mut level_builder = LevelBuilder::new();
    let mut id = level_builder.add(Vec2::ZERO, area_start());
    for i in 1..=1 {
        id = level_builder.add_after(id, PartAlign::Down, area_center(i));
        level_builder.add_after(id, PartAlign::Left, area_left(i));
        level_builder.add_after(id, PartAlign::Right, area_right(i));
        id = level_builder.add_after(id, PartAlign::Down, area_safe());
    }

    let level = level_builder.build(4.0);

    for (_, [x, y]) in level.kd().iter() {
        commands.spawn((
            Mesh3d(meshes.add(Circle::new(0.25))),
            MeshMaterial3d(materials.add(Color::BLACK)),
            Transform::from_xyz(x, 0.02, y).with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
        ));
        commands.spawn((
            PointLight {
                intensity: 1000000.0,
                range: 100.0,
                ..Default::default()
            },
            Transform::from_xyz(x, 10.0, y),
        ));
    }

    commands.insert_resource(level);

    // for chunk in chunks {
    //     commands.spawn((
    //         Mesh3d(meshes.add(chunk)),
    //         MeshMaterial3d(materials.add(Color::WHITE)),
    //         Transform::default(),
    //     ));
    // }

    // for (part, point) in level.points() {

    // }

    // for (start, end) in level.edges() {
    //     let mid = (start + end) * 0.5;
    //     let distance = start.distance(end);
    //     commands.spawn((
    //         Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::new(1.0, 0.05)))),
    //         MeshMaterial3d(materials.add(Color::BLACK)),
    //         Transform::from_xyz(mid.x as f32, 0.01, mid.y as f32)
    //             .with_scale(Vec3::new(distance * 0.5, 1.0, 1.0))
    //             .with_rotation(Quat::from_rotation_y((end - start).angle_to(Vec2::X))),
    //     ));
    // }

    // commands.spawn((
    //     Camera3d::default(),
    //     FlyCam,
    //     Transform::default(),
    //     Visibility::default(),
    // ));

    commands.spawn((Player, Transform::from_xyz(-2.0, 0.0, 5.0)));

    window.cursor_options.grab_mode = CursorGrabMode::Confined;
    window.cursor_options.visible = false;
}

fn grab_cursor(
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        if window.cursor_options.visible {
            window.cursor_options.grab_mode = CursorGrabMode::Confined;
            window.cursor_options.visible = false;
        } else {
            window.cursor_options.grab_mode = CursorGrabMode::None;
            window.cursor_options.visible = true;
        }
    }
}
