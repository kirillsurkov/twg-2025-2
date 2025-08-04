#![windows_subsystem = "windows"]

use core::f32;

use bevy::{
    audio::{PlaybackMode, Volume},
    prelude::*,
    render::view::RenderLayers,
    window::{CursorGrabMode, PrimaryWindow, WindowMode},
};
use bevy_hanabi::HanabiPlugin;
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use bevy_mod_skinned_aabb::SkinnedAabbPlugin;
use petgraph::visit::EdgeRef;
use rand::{
    distr::{Distribution, weighted::WeightedIndex},
    seq::{IndexedRandom, SliceRandom},
};

use crate::{
    boss::{BossPlugin, BossSpawner},
    enemy::{
        Enemy, EnemyPlugin, beetle::Beetle, glutton::Glutton, mushroom::Mushroom, seal::Seal,
        spider::Spider, stalker::Stalker, tree::Tree, turret::Turret, wolf::Wolf,
        wormbeak::Wormbeak,
    },
    heart::{HeartPlugin, HeartSpawner},
    level::{Level, LevelBiome, LevelBuilder, LevelPart, LevelPartBuilder, PartAlign},
    model_loader::ModelLoaderPlugin,
    player::{Player, PlayerPlugin},
    projectile::ProjectilePlugin,
    terrain::TerrainPlugin,
    ui::{GameUiPlugin, UserNotify},
    weapon::{
        WeaponPlugin, biogun::Biogun, blaster::Blaster, ion_cannon::IonCannon,
        pulse_rifle::PulseRifle, zapper::Zapper,
    },
};

mod boss;
mod enemy;
mod heart;
mod level;
mod model_loader;
mod player;
mod projectile;
mod terrain;
mod ui;
mod weapon;

#[derive(Resource)]
pub enum GameState {
    Running,
    Paused,
    Win,
    Lose,
}

fn gamestate(
    state: Res<GameState>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    mut user_notify: EventWriter<UserNotify>,
) {
    match *state {
        GameState::Win => {
            user_notify.write(UserNotify(
                "Поздравляем".to_string(),
                "Вы прошли игру".to_string(),
            ));
        }
        GameState::Lose => {
            user_notify.write(UserNotify(
                "Они ждали тебя не как врага".to_string(),
                "А как жертву".to_string(),
            ));
        }
        GameState::Running => {
            window.cursor_options.grab_mode = CursorGrabMode::Confined;
            window.cursor_options.visible = false;
        }
        GameState::Paused => {
            window.cursor_options.grab_mode = CursorGrabMode::None;
            window.cursor_options.visible = true;

            user_notify.write(UserNotify(
                "PAUSED".to_string(),
                "PRESS ESC чтобы продолжить".to_string(),
            ));
        }
    }
}

fn fullscreen(
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut fullscreen: Local<bool>,
) {
    if keys.just_pressed(KeyCode::F11) {
        window.mode = if *fullscreen {
            WindowMode::Windowed
        } else {
            WindowMode::BorderlessFullscreen(MonitorSelection::Current)
        };
        *fullscreen = !*fullscreen;
    }
}

fn main() {
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
        // .add_plugins(EguiPlugin::default())
        // .add_plugins(WorldInspectorPlugin::default())
        .add_plugins(SkinnedAabbPlugin)
        .insert_resource(AmbientLight {
            color: Color::BLACK,
            brightness: 0.0,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::srgba(0.02, 0.02, 0.02, 1.0)))
        .insert_resource(GameState::Running)
        .add_systems(Startup, setup)
        .add_systems(Update, defer_despawn)
        .add_systems(Update, gamestate)
        .add_systems(Update, fullscreen)
        // .add_systems(Update, bury)
        .add_systems(Update, update_level)
        .add_systems(Update, grab_cursor)
        .insert_resource(level_builder.build(4.0))
        .add_plugins(EnemyPlugin)
        .add_plugins(BossPlugin)
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

fn setup(
    mut commands: Commands,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
    level: Res<Level>,
) {
    let mut enemy_points = vec![];
    for edge in level.graph.edge_references() {
        let source = level.graph.node_weight(edge.source()).unwrap();
        let target = level.graph.node_weight(edge.target()).unwrap();
        let dir = (target - source).normalize();
        let dist = source.distance(*target);
        for _ in 0..10 {
            enemy_points.push(source + dir * rand::random_range(0.0..=dist));
        }
    }

    let mut rng = rand::rng();

    enemy_points.shuffle(&mut rng);

    let mut spawned = 0;
    while let Some(point) = enemy_points.pop() {
        if spawned >= 100 {
            break;
        }

        let biome = level.biome(point).0;
        let Ok(dist) = WeightedIndex::new(&biome[3..=7]) else {
            continue;
        };

        let choices = [
            ["tree", "wolf"],        // forest
            ["seal", "wormbeak"],    // cave
            ["mushroom", "stalker"], // mushroom
            ["spider", "turret"],    // temple
            ["glutton", "beetle"],   // meat
        ];

        let Some(choice) = choices[dist.sample(&mut rng)].choose(&mut rng) else {
            continue;
        };

        spawned += 1;
        // println!("{choice}: {point:?}");
        match *choice {
            "tree" => commands.spawn(Tree),
            "wolf" => commands.spawn(Wolf),
            "seal" => commands.spawn(Seal),
            "wormbeak" => commands.spawn(Wormbeak),
            "mushroom" => commands.spawn(Mushroom),
            "stalker" => commands.spawn(Stalker),
            "spider" => commands.spawn(Spider),
            "turret" => commands.spawn(Turret),
            "glutton" => commands.spawn(Glutton),
            "beetle" => commands.spawn(Beetle),
            _ => panic!("Unknown enemy {choice}"),
        }
        .insert(Transform::from_xyz(point.x, 0.0, point.y));
    }

    let player_xy = level.nearest_terrain(1, Vec2::new(0.0, f32::MAX))[0].unwrap();
    let node = level.nearest_id_terrain(1, player_xy)[0];

    let spawn_point = {
        let node = level.graph.neighbors(node).next().unwrap();
        let point = level.graph.node_weight(node).unwrap();
        player_xy + (point - player_xy).normalize() * 5.0
    };

    let step = (spawn_point - player_xy).normalize() * 5.0;

    let home = Vec2::new(0.0, 0.0);
    let forest = Vec2::new(0.0, -170.0);
    let cave = Vec2::new(0.0, -340.0);
    let mushroom = Vec2::new(-1000.0, -170.0);
    let safe1 = Vec2::new(0.0, -456.0);
    let temple = Vec2::new(0.0, -572.0);
    let meat = Vec2::new(1000.0, -286.0);
    let safe2 = Vec2::new(0.0, -688.0);
    let boss = Vec2::new(0.0, -750.0);

    for point in [home, safe1, safe2] {
        let pos = level.nearest_terrain(1, point)[0].unwrap();
        commands.spawn((
            HeartSpawner,
            Transform::from_translation((pos).extend(0.0).xzy()),
        ));
    }

    commands.spawn((
        Blaster,
        Transform::from_translation((spawn_point + step * 1.0).extend(0.0).xzy()),
    ));

    commands.spawn((
        PulseRifle,
        Transform::from_translation(level.nearest_terrain(1, cave)[0].unwrap().extend(0.0).xzy()),
    ));

    commands.spawn((
        Zapper,
        Transform::from_translation(
            level.nearest_terrain(1, mushroom)[0]
                .unwrap()
                .extend(0.0)
                .xzy(),
        ),
    ));

    commands.spawn((
        IonCannon,
        Transform::from_translation(
            level.nearest_terrain(1, temple)[0]
                .unwrap()
                .extend(0.0)
                .xzy(),
        ),
    ));

    commands.spawn((
        Biogun,
        Transform::from_translation(level.nearest_terrain(1, meat)[0].unwrap().extend(0.0).xzy()),
    ));

    commands.spawn((
        BossSpawner,
        Transform::from_translation(level.nearest_terrain(1, boss)[0].unwrap().extend(0.0).xzy()),
    ));

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

    commands.spawn((
        AudioPlayer::new(asset_server.load("music/valaam_drums.ogg")),
        PlaybackSettings {
            mode: PlaybackMode::Loop,
            volume: Volume::Linear(0.1),
            ..Default::default()
        },
    ));

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

fn grab_cursor(keys: Res<ButtonInput<KeyCode>>, mut game_state: ResMut<GameState>) {
    if keys.just_pressed(KeyCode::Escape) {
        match *game_state {
            GameState::Running => *game_state = GameState::Paused,
            GameState::Paused => *game_state = GameState::Running,
            _ => {}
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
