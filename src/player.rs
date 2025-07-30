use bevy::{
    core_pipeline::prepass::DepthPrepass,
    input::mouse::MouseMotion,
    prelude::*,
    render::experimental::occlusion_culling::OcclusionCulling,
    window::{CursorGrabMode, PrimaryWindow},
};

use crate::level::Level;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, init);
        app.add_systems(Update, controller);
        app.add_systems(Update, nearest_node);
    }
}

#[derive(Component)]
pub struct Player;

fn init(mut commands: Commands, players: Query<Entity, Added<Player>>) {
    for entity in players {
        commands
            .entity(entity)
            .insert(Visibility::default())
            .with_child((
                Camera3d::default(),
                Projection::from(PerspectiveProjection {
                    fov: 60.0_f32.to_radians(),
                    ..Default::default()
                }),
                Transform::from_xyz(0.0, 1.7, 0.0).looking_to(-Vec3::Z, Vec3::Y),
                DepthPrepass,
                OcclusionCulling,
            ));
    }
}

fn world_to_texture(world_pos: Vec2, world_bounds: Rect, texture_size: UVec2) -> Vec2 {
    (world_pos - world_bounds.min) * texture_size.as_vec2() / world_bounds.size()
}

fn controller(
    window: Single<&Window, With<PrimaryWindow>>,
    player: Single<(Entity, &Children), With<Player>>,
    camera: Query<(), With<Camera>>,
    mut transforms: Query<&mut Transform>,
    keys: Res<ButtonInput<KeyCode>>,
    mut mouse: EventReader<MouseMotion>,
    time: Res<Time>,
    level: Res<Level>,
) {
    let sensivity = 0.12;
    let speed = 12.0;

    let [mut transform_player, mut transform_camera] = {
        let (entity_player, children) = player.into_inner();
        let mut entity_camera = Entity::PLACEHOLDER;

        for child in children {
            if camera.contains(*child) {
                entity_camera = *child;
            }
        }

        transforms
            .get_many_mut([entity_player, entity_camera])
            .unwrap()
    };

    for ev in mouse.read() {
        let (mut yaw, mut pitch, _) = transform_camera.rotation.to_euler(EulerRot::YXZ);
        match window.cursor_options.grab_mode {
            CursorGrabMode::None => (),
            _ => {
                pitch -= (sensivity * ev.delta.y).to_radians();
                yaw -= (sensivity * ev.delta.x).to_radians();
            }
        }

        pitch = pitch.clamp(-1.54, 1.54);

        transform_camera.rotation =
            Quat::from_axis_angle(Vec3::Y, yaw) * Quat::from_axis_angle(Vec3::X, pitch);
    }

    let forward = transform_camera.forward().xz().extend(0.0).xzy();
    let right = transform_camera.right().xz().extend(0.0).xzy();

    let mut move_vec = Vec3::default();
    for key in keys.get_pressed() {
        match key {
            KeyCode::KeyW => move_vec += forward,
            KeyCode::KeyA => move_vec -= right,
            KeyCode::KeyS => move_vec -= forward,
            KeyCode::KeyD => move_vec += right,
            _ => {}
        }
    }

    if let Some(move_vec) = move_vec.try_normalize() {
        let mut desired_pos = transform_player.translation + move_vec * time.delta_secs() * speed;

        let height_map = level.height_map();
        let level_bounds = level.bounds();

        let texture_pos = world_to_texture(
            transform_player.translation.xz(),
            level_bounds,
            UVec2::from(height_map.dimensions()),
        );

        let scale = level_bounds.size() / UVec2::from(height_map.dimensions()).as_vec2();

        let mut resolved = false;
        for i in 0..100 {
            let step = i as f32 / 10.0;

            for vec in [Vec3::X, -Vec3::X, Vec3::Z, -Vec3::Z] {
                let pos = texture_pos + vec.xz() * step;
                if height_map.get_pixel(pos.x as u32, pos.y as u32).0[0] == 0.0 {
                    desired_pos += vec * step * scale.x;
                    resolved = true;
                    break;
                }
            }

            if resolved {
                break;
            }
        }

        if resolved {
            transform_player.translation = desired_pos;
        }
    }
}

fn nearest_node(level: Res<Level>, player: Single<&Transform, With<Player>>) {
    let player_pos = player.translation.xz();
    let nearest = level.nearest_one(player_pos).unwrap();
}
