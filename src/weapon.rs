use std::{f32::consts::TAU, time::Duration};

use bevy::{prelude::*, render::view::RenderLayers};

use crate::player::Player;

pub mod zapper;

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update);
        app.add_systems(Update, animate);
        app.add_systems(Update, shoot);
        app.add_systems(Update, drop_weapon.after(update));
        app.add_systems(Update, pick_weapon.after(update));
        app.add_systems(Update, zapper::update);
    }
}

enum State {
    OnGround,
    InHands { shoot: bool },
}

#[derive(Component)]
pub struct Weapon {
    model: Entity,
    anim_player: Entity,
    offset: Vec3,
    state: State,
}

impl Weapon {
    pub fn new(model: Entity, anim_player: Entity, offset: Vec3) -> Self {
        Self {
            model,
            anim_player,
            offset,
            state: State::OnGround,
        }
    }
}

#[derive(Component)]
struct DropWeapon;

#[derive(Component)]
struct PickWeapon;

fn drop_weapon(
    mut commands: Commands,
    mut weapons: Query<(Entity, &mut Weapon), With<DropWeapon>>,
    mut transforms: Query<&mut Transform>,
    player: Single<(&Player, &GlobalTransform)>,
) {
    let (player, player_pos) = player.into_inner();
    let player_pos = player_pos.translation();

    for (entity, mut weapon) in &mut weapons {
        if let Ok(mut transform) = transforms.get_mut(weapon.model) {
            transform.translation = Vec3::ZERO;
        }
        weapon.state = State::OnGround;
        commands
            .entity(player.weapon_camera)
            .remove_children(&[entity]);
        commands
            .entity(entity)
            .insert(Transform::from_translation(player_pos))
            .remove::<DropWeapon>();
    }
}

fn pick_weapon(
    mut commands: Commands,
    mut weapons: Query<(Entity, &mut Weapon), With<PickWeapon>>,
    mut transforms: Query<&mut Transform>,
    player: Single<&mut Player>,
) {
    for (entity, mut weapon) in &mut weapons {
        if let Ok(mut transform) = transforms.get_mut(weapon.model) {
            transform.translation = weapon.offset;
            transform.rotation = Quat::default();
        }
        weapon.state = State::InHands { shoot: false };
        commands.entity(player.weapon_camera).add_child(entity);
        commands
            .entity(entity)
            .insert(Transform::default())
            .remove::<PickWeapon>();
    }
}

fn update(
    mut commands: Commands,
    player: Single<(&mut Player, &GlobalTransform)>,
    weapons2: Query<(Entity, &GlobalTransform), With<Weapon>>,
    mut weapons: Query<&mut Weapon>,
) {
    let layer_world = RenderLayers::layer(0);
    let layer_hands = RenderLayers::layer(1);
    let pickup_dist = 3.0;

    let (mut player, player_pos) = player.into_inner();
    let player_pos = player_pos.translation();

    for (entity, transform) in weapons2 {
        let mut weapon = weapons.get_mut(entity).unwrap();
        let can_pickup = transform.translation().distance(player_pos) <= pickup_dist;

        match &mut weapon.state {
            State::OnGround => {
                if can_pickup && player.interaction {
                    let slot = player.active_slot;
                    if let Ok(mut entity) = commands.get_entity(player.weapons[slot]) {
                        entity.insert(DropWeapon);
                    }
                    player.weapons[slot] = entity;
                    commands.entity(entity).insert(PickWeapon);
                } else {
                    commands
                        .entity(entity)
                        .insert_recursive::<Children>((layer_world.clone(), Visibility::Inherited));
                }
            }
            State::InHands { shoot } => {
                let active = player.weapons[player.active_slot] == entity;
                let visibility = if active {
                    if player.drop_weapon {
                        let slot = player.active_slot;
                        if let Ok(mut entity) = commands.get_entity(player.weapons[slot]) {
                            entity.insert(DropWeapon);
                        }
                        player.weapons[slot] = Entity::PLACEHOLDER;
                    }
                    *shoot = player.shoot;
                    Visibility::Inherited
                } else {
                    *shoot = false;
                    Visibility::Hidden
                };
                commands
                    .entity(entity)
                    .insert_recursive::<Children>((layer_hands.clone(), visibility));
            }
        }
    }
}

fn animate(
    weapons: Query<&Weapon>,
    mut transforms: Query<&mut Transform>,
    mut anim_players: Query<(
        &mut AnimationPlayer,
        &mut AnimationTransitions,
        &AnimationGraphHandle,
    )>,
    graphs: Res<Assets<AnimationGraph>>,
    clips: Res<Assets<AnimationClip>>,
    time: Res<Time>,
) {
    let idle = AnimationNodeIndex::new(1);
    let shoot = AnimationNodeIndex::new(2);

    for weapon in weapons {
        let (mut player, mut transition, graph) = anim_players.get_mut(weapon.anim_player).unwrap();

        let AnimationNodeType::Clip(clip) =
            &graphs.get(graph).unwrap().get(shoot).unwrap().node_type
        else {
            continue;
        };
        let clip = clips.get(clip).unwrap();

        let index = match weapon.state {
            State::InHands { shoot: true } => shoot,
            _ => idle,
        };

        if !player.is_playing_animation(index) {
            transition
                .play(&mut player, index, Duration::from_millis(250))
                .set_speed(clip.duration())
                .repeat();
        }

        if matches!(weapon.state, State::OnGround) {
            if let Ok(mut transform) = transforms.get_mut(weapon.model) {
                let angle = time.elapsed_secs() * TAU;
                transform.translation.y = 0.5 * (angle.sin() + 2.0);
                transform.rotation = Quat::from_rotation_y(angle);
            }
        }
    }
}

fn shoot() {}
