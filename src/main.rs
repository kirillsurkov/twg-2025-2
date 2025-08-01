// #![windows_subsystem = "windows"]

use core::f32;

use bevy::{
    prelude::*,
    render::view::RenderLayers,
    window::{CursorGrabMode, PrimaryWindow},
};
// use bevy_flycam::{FlyCam, NoCameraPlayerPlugin};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use kiddo::SquaredEuclidean;
use petgraph::{algo::dijkstra, graph::NodeIndex};

use crate::{
    enemy::{EnemyPlugin, spider::Spider},
    level::{LevelBiome, LevelBuilder, LevelPart, LevelPartBuilder, PartAlign},
    model_loader::{LoadModel, ModelLoaderPlugin, ReadyAction},
    player::{Player, PlayerPlugin},
    terrain::TerrainPlugin,
    weapon::{WeaponPlugin, zapper::Zapper},
};

mod enemy;
mod level;
mod model_loader;
mod player;
mod terrain;
mod weapon;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Tuonela".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::default())
        // .add_plugins(NoCameraPlayerPlugin)
        .insert_resource(AmbientLight {
            color: Color::BLACK,
            brightness: 0.0,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::srgba(0.02, 0.02, 0.02, 1.0)))
        .add_systems(Startup, setup)
        .add_systems(Update, grab_cursor)
        .add_plugins(PlayerPlugin)
        .add_plugins(TerrainPlugin)
        .add_plugins(ModelLoaderPlugin)
        .add_plugins(EnemyPlugin)
        .add_plugins(WeaponPlugin)
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

fn setup(mut commands: Commands, mut window: Single<&mut Window, With<PrimaryWindow>>) {
    let mut level_builder = LevelBuilder::new();
    let mut id = level_builder.add(Vec2::ZERO, area_start());
    for i in 1..=2 {
        id = level_builder.add_after(id, PartAlign::Down, area_center(i));
        level_builder.add_after(id, PartAlign::Left, area_left(i));
        level_builder.add_after(id, PartAlign::Right, area_right(i));
        id = level_builder.add_after(id, PartAlign::Down, area_safe());
    }

    let level = level_builder.build(4.0);

    let player_xy = level.nearest_one(Vec2::new(0.0, f32::MAX)).unwrap();

    let node = NodeIndex::new(
        level
            .kd
            .nearest_one::<SquaredEuclidean>(&[player_xy.x, player_xy.y])
            .item as usize,
    );

    let spawn_point = {
        let node = level.graph.neighbors(node).next().unwrap();
        let point = level.graph.node_weight(node).unwrap();
        player_xy + (point - player_xy).normalize() * 5.0
    };

    commands.spawn((
        Zapper,
        Transform::from_xyz(spawn_point.x, 0.0, spawn_point.y),
    ));

    commands.spawn((
        Spider,
        Transform::from_xyz(spawn_point.x, 0.0, spawn_point.y),
    ));

    commands.insert_resource(level);

    commands.spawn((
        Player::new(),
        Transform::from_xyz(player_xy.x, 0.0, player_xy.y),
    ));

    for (x, y) in [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
        commands.spawn((
            DirectionalLight {
                illuminance: 200.0,
                ..Default::default()
            },
            Transform::default().looking_to(Vec3::new(x, -1.0, y), Vec3::Y),
            RenderLayers::from_layers(&[0, 1]),
        ));
    }

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
