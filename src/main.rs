// #![windows_subsystem = "windows"]

use core::f32;

use bevy::{
    prelude::*,
    render::view::RenderLayers,
    window::{CursorGrabMode, PrimaryWindow, WindowMode},
};
use bevy_hanabi::HanabiPlugin;
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};

use crate::{
    enemy::{
        Enemy, EnemyPlugin, glutton::Glutton, mushroom::Mushroom, seal::Seal, spider::Spider,
        tree::Tree, turret::Turret, wolf::Wolf, wormbeak::Wormbeak,
    },
    heart::{HeartPlugin, HeartSpawner},
    level::{Level, LevelBiome, LevelBuilder, LevelPart, LevelPartBuilder, PartAlign},
    model_loader::ModelLoaderPlugin,
    player::{Player, PlayerPlugin},
    projectile::ProjectilePlugin,
    terrain::TerrainPlugin,
    ui::GameUiPlugin,
    weapon::{
        WeaponPlugin, biogun::Biogun, blaster::Blaster, ion_cannon::IonCannon,
        pulse_rifle::PulseRifle, zapper::Zapper,
    },
};

mod enemy;
mod heart;
mod level;
mod model_loader;
mod player;
mod projectile;
mod terrain;
mod ui;
mod weapon;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Tuonela".to_string(),
                // mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins(HanabiPlugin)
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::default())
        .insert_resource(AmbientLight {
            color: Color::BLACK,
            brightness: 0.0,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::srgba(0.02, 0.02, 0.02, 1.0)))
        .add_systems(Startup, setup)
        .add_systems(Update, defer_despawn)
        // .add_systems(Update, bury)
        .add_systems(Update, update_level)
        .add_systems(Update, grab_cursor)
        .add_plugins(EnemyPlugin)
        .add_plugins(HeartPlugin)
        .add_plugins(ModelLoaderPlugin)
        .add_plugins(PlayerPlugin)
        .add_plugins(ProjectilePlugin)
        .add_plugins(TerrainPlugin)
        .add_plugins(GameUiPlugin)
        .add_plugins(WeaponPlugin)
        .run();
}

const BASE_WIDTH: f32 = 120.0;
const BASE_HEIGHT: f32 = 120.0;

fn area_home() -> LevelPart {
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

fn area_forest() -> LevelPart {
    LevelPartBuilder::new(LevelBiome::Forest)
        .with_size(BASE_WIDTH, BASE_HEIGHT)
        .with_count(40)
        .with_fill_ratio(0.2)
        .build()
}

fn area_cave() -> LevelPart {
    LevelPartBuilder::new(LevelBiome::Cave)
        .with_size(BASE_WIDTH, BASE_HEIGHT)
        .with_count(40)
        .with_fill_ratio(0.2)
        .build()
}

fn area_mushroom() -> LevelPart {
    LevelPartBuilder::new(LevelBiome::Mushroom)
        .with_size(BASE_WIDTH * 0.5, BASE_HEIGHT)
        .with_count(20)
        .with_fill_ratio(0.2)
        .build()
}

fn area_temple() -> LevelPart {
    LevelPartBuilder::new(LevelBiome::Temple)
        .with_size(BASE_WIDTH, BASE_HEIGHT)
        .with_count(40)
        .with_fill_ratio(0.2)
        .build()
}

fn area_meat() -> LevelPart {
    LevelPartBuilder::new(LevelBiome::Meat)
        .with_size(BASE_WIDTH * 0.5, BASE_HEIGHT)
        .with_count(20)
        .with_fill_ratio(0.2)
        .build()
}

fn area_boss() -> LevelPart {
    LevelPartBuilder::new(LevelBiome::Boss)
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

fn setup(mut commands: Commands, mut window: Single<&mut Window, With<PrimaryWindow>>) {
    let mut level_builder = LevelBuilder::new();

    let mut id = level_builder.add(Vec2::ZERO, area_home());

    id = level_builder.add_after(id, PartAlign::Down, area_forest());

    id = level_builder.add_after(id, PartAlign::Down, area_cave());
    level_builder.add_after(id, PartAlign::Left, area_mushroom());
    id = level_builder.add_after(id, PartAlign::Down, area_safe());

    id = level_builder.add_after(id, PartAlign::Down, area_temple());
    level_builder.add_after(id, PartAlign::Right, area_meat());
    id = level_builder.add_after(id, PartAlign::Down, area_safe());

    level_builder.add_after(id, PartAlign::Down, area_boss());

    let level = level_builder.build(4.0);

    let player_xy = level.nearest_terrain(1, Vec2::new(0.0, f32::MAX))[0].unwrap();
    let node = level.nearest_id_terrain(1, player_xy)[0];

    let spawn_point = {
        let node = level.graph.neighbors(node).next().unwrap();
        let point = level.graph.node_weight(node).unwrap();
        player_xy + (point - player_xy).normalize() * 5.0
    };

    let step = (spawn_point - player_xy).normalize() * 5.0;

    commands.spawn((
        Zapper,
        Transform::from_translation((spawn_point + step * 0.0).extend(0.0).xzy()),
    ));

    commands.spawn((
        Blaster,
        Transform::from_translation((spawn_point + step * 1.0).extend(0.0).xzy()),
    ));

    commands.spawn((
        PulseRifle,
        Transform::from_translation((spawn_point + step * 2.0).extend(0.0).xzy()),
    ));

    commands.spawn((
        IonCannon,
        Transform::from_translation((spawn_point + step * 3.0).extend(0.0).xzy()),
    ));

    commands.spawn((
        Biogun,
        Transform::from_translation((spawn_point + step * 4.0).extend(0.0).xzy()),
    ));

    commands.spawn((
        Wolf,
        Transform::from_translation((spawn_point + step * 5.0).extend(0.0).xzy()),
    ));

    commands.spawn((
        HeartSpawner,
        Transform::from_translation((spawn_point + step * 6.0).extend(0.0).xzy()),
    ));

    commands.insert_resource(level);

    commands.spawn((
        Player::new(100.0),
        Transform::from_xyz(player_xy.x, 0.0, player_xy.y),
    ));

    let mut shadows = true;
    for (x, y) in [
        (0.0, 0.0),
        (-1.0, -1.0),
        (-1.0, 1.0),
        (1.0, -1.0),
        (1.0, 1.0),
    ] {
        commands.spawn((
            DirectionalLight {
                illuminance: 100.0,
                shadows_enabled: shadows,
                ..Default::default()
            },
            Transform::default().looking_to(Vec3::new(x, -1.0, y), Vec3::Y),
            RenderLayers::from_layers(&[0, 1]),
        ));
        shadows = false;
    }

    window.cursor_options.grab_mode = CursorGrabMode::Confined;
    window.cursor_options.visible = false;
}

fn update_level(
    mut level: ResMut<Level>,
    enemies: Query<(Entity, &GlobalTransform), With<Enemy>>,
    player: Single<(Entity, &GlobalTransform), With<Player>>,
) {
    level.clear_creatures();
    level.add_creature(player.0, player.1.translation());
    for (enemy, transform) in enemies {
        level.add_creature(enemy, transform.translation());
    }
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

#[derive(Component)]
pub struct DeferDespawn(pub f32);

fn defer_despawn(
    mut commands: Commands,
    mut despawns: Query<(Entity, &mut DeferDespawn)>,
    time: Res<Time>,
) {
    for (entity, mut despawn) in &mut despawns {
        if despawn.0 <= 0.0 {
            commands.entity(entity).despawn();
        } else {
            despawn.0 -= time.delta_secs();
        }
    }
}

#[derive(Component)]
pub struct Bury {
    pub meters_per_second: f32,
    pub time: f32,
}

fn bury(
    mut commands: Commands,
    mut buries: Query<(Entity, &mut Bury, &mut Transform)>,
    time: Res<Time>,
) {
    for (entity, mut bury, mut transform) in &mut buries {
        bury.time -= time.delta_secs();
        if bury.time > 0.0 {
            transform.translation.y += time.delta_secs() * bury.meters_per_second;
        } else if let Ok(mut entity) = commands.get_entity(entity) {
            entity.remove::<Bury>();
        }
    }
}
