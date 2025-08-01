use bevy::{
    core_pipeline::prepass::DepthPrepass,
    input::mouse::MouseMotion,
    prelude::*,
    render::{experimental::occlusion_culling::OcclusionCulling, view::RenderLayers},
    window::{CursorGrabMode, PrimaryWindow},
};

use crate::terrain::Physics;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, init);
        app.add_systems(Update, controller.after(init));
    }
}

#[derive(Component)]
pub struct Player {
    world_camera: Entity,
    pub weapon_camera: Entity,
    pub weapons: Vec<Entity>,
    pub active_slot: usize,
    pub interaction: bool,
    pub drop_weapon: bool,
    pub shoot: bool,
}

impl Player {
    pub fn new() -> Self {
        Self {
            world_camera: Entity::PLACEHOLDER,
            weapon_camera: Entity::PLACEHOLDER,
            weapons: vec![Entity::PLACEHOLDER; 4],
            active_slot: 0,
            interaction: false,
            drop_weapon: false,
            shoot: false,
        }
    }
}

fn init(mut commands: Commands, player: Single<(Entity, &mut Player), Added<Player>>) {
    let (player_entity, mut player) = player.into_inner();

    let camera_projection = Projection::from(PerspectiveProjection {
        fov: 60.0_f32.to_radians(),
        ..Default::default()
    });

    player.weapon_camera = commands
        .spawn((
            RenderLayers::layer(1),
            Camera3d::default(),
            Camera {
                order: 1,
                ..Default::default()
            },
            camera_projection.clone(),
            Transform::default(),
        ))
        .id();

    player.world_camera = commands
        .spawn((
            RenderLayers::layer(0),
            Camera3d::default(),
            Camera {
                order: 0,
                ..Default::default()
            },
            camera_projection,
            Transform::from_xyz(0.0, 1.7, 0.0).looking_to(-Vec3::Z, Vec3::Y),
            DepthPrepass,
            OcclusionCulling,
        ))
        .add_child(player.weapon_camera)
        .id();

    commands
        .entity(player_entity)
        .insert(Visibility::default())
        .insert(Physics::new(0.5, 12.0))
        .add_child(player.world_camera);
}

fn controller(
    window: Single<&Window, With<PrimaryWindow>>,
    player: Single<(&mut Player, &mut Physics)>,
    mut transforms: Query<&mut Transform>,
    keys: Res<ButtonInput<KeyCode>>,
    keys_mouse: Res<ButtonInput<MouseButton>>,
    mut mouse: EventReader<MouseMotion>,
) {
    let sensivity = 0.12;
    let (mut player, mut physics) = player.into_inner();

    let mut transform_camera = transforms.get_mut(player.world_camera).unwrap();

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

    let forward = transform_camera.forward().xz();
    let right = transform_camera.right().xz();

    let mut move_vec = Vec2::ZERO;
    for key in keys.get_pressed() {
        match key {
            KeyCode::KeyW => move_vec += forward,
            KeyCode::KeyA => move_vec -= right,
            KeyCode::KeyS => move_vec -= forward,
            KeyCode::KeyD => move_vec += right,
            _ => {}
        }
    }
    physics.move_vec = move_vec.normalize_or_zero();

    player.interaction = keys.just_pressed(KeyCode::KeyE);
    player.drop_weapon = keys.just_pressed(KeyCode::KeyQ);
    player.shoot = keys_mouse.pressed(MouseButton::Left);
    player.active_slot = match true {
        _ if keys.just_pressed(KeyCode::Digit1) => 0,
        _ if keys.just_pressed(KeyCode::Digit2) => 1,
        _ if keys.just_pressed(KeyCode::Digit3) => 2,
        _ if keys.just_pressed(KeyCode::Digit4) => 3,
        _ => player.active_slot,
    };
}
