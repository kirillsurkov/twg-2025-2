use std::{collections::VecDeque, time::Duration};

use bevy::prelude::*;
use petgraph::{algo::astar, graph::NodeIndex};

use crate::{
    level::Level,
    player::Player,
    projectile::{Damage, bullet::Bullet},
    terrain::Physics,
};

pub mod beetle;
pub mod glutton;
pub mod spider;
pub mod stalker;
pub mod wormbeak;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, animate);
        app.add_systems(Update, ai);
        app.add_systems(Update, beetle::setup);
        app.add_systems(Update, glutton::setup);
        app.add_systems(Update, spider::setup);
        app.add_systems(Update, stalker::setup);
        app.add_systems(Update, wormbeak::setup);
    }
}

#[derive(Clone, Copy)]
pub enum AttackKind {
    Ranged,
    Melee,
}

#[derive(Clone)]
enum State {
    Idle,
    Walk {
        aggro_timer: f32,
        aggro_entity: Entity,
        aggro_nearest_node: NodeIndex,
        walk_path: VecDeque<Vec2>,
        walk_target: Option<Vec2>,
    },
    Attack {
        timer_prepare: f32,
        timer_action: f32,
        ranged_done: bool,
        origin: Vec2,
        target: Entity,
        target_pos: Vec2,
    },
    Death,
}

enum Animation {
    Idle,
    Walk,
    Attack,
    Death,
}

#[derive(Component)]
pub struct Enemy {
    scene: Entity,
    anim_player: Entity,
    attack: AttackKind,
    attack_range: f32,
    attack_delay: f32,
    speed: f32,
    shoot_point: Vec3,
    state: State,
    animation: Option<Animation>,
}

impl Enemy {
    pub fn new(
        scene: Entity,
        anim_player: Entity,
        attack: AttackKind,
        attack_range: f32,
        attack_delay: f32,
        speed: f32,
        shoot_point: Vec3,
    ) -> Self {
        Self {
            scene,
            anim_player,
            attack,
            attack_range,
            attack_delay,
            speed,
            shoot_point,
            state: State::Idle,
            animation: None,
        }
    }
}

fn animate(
    mut enemies: Query<(&mut Enemy, &Physics)>,
    mut animation: Query<(
        &mut AnimationPlayer,
        &mut AnimationTransitions,
        &AnimationGraphHandle,
    )>,
    graphs: Res<Assets<AnimationGraph>>,
    clips: Res<Assets<AnimationClip>>,
) {
    let idle = AnimationNodeIndex::new(1);
    let walk = AnimationNodeIndex::new(2);
    let attack = AnimationNodeIndex::new(3);
    let death = AnimationNodeIndex::new(4);

    for (mut enemy, physics) in &mut enemies {
        let (mut player, mut transition, graph) = animation.get_mut(enemy.anim_player).unwrap();

        let index = match enemy.animation.take() {
            Some(Animation::Idle) => idle,
            Some(Animation::Walk) => walk,
            Some(Animation::Attack) => attack,
            Some(Animation::Death) => death,
            _ => continue,
        };

        let AnimationNodeType::Clip(clip) =
            &graphs.get(graph).unwrap().get(index).unwrap().node_type
        else {
            continue;
        };
        let clip = clips.get(clip).unwrap();

        if !player.is_playing_animation(index) || player.all_finished() {
            transition
                .play(&mut player, index, Duration::from_millis(100))
                .set_speed(match true {
                    _ if index == idle => 1.0,
                    _ if index == walk => clip.duration() * physics.speed * 0.5,
                    _ if index == attack => clip.duration() / enemy.attack_delay,
                    _ if index == death => 1.0,
                    _ => unreachable!(),
                });
        }
    }
}

fn ai(
    mut commands: Commands,
    level: Res<Level>,
    player: Single<Entity, With<Player>>,
    transforms: Query<&Transform>,
    global_transforms: Query<&GlobalTransform>,
    mut enemies: Query<(Entity, &mut Enemy, &mut Physics)>,
    time: Res<Time>,
) {
    let aggro_distance = 50.0;
    let aggro_timer = 3.0;

    let player_pos = transforms.get(*player).unwrap().translation.xz();

    for (entity, mut enemy, mut physics) in &mut enemies {
        let transform = transforms.get(entity).unwrap();
        let pos_3d = transform.translation;
        let pos = pos_3d.xz();

        physics.move_vec = Vec2::ZERO;
        physics.speed = enemy.speed;
        physics.ignore_overlap = false;

        match enemy.state.clone() {
            State::Idle => {
                if player_pos.distance(pos) < aggro_distance {
                    enemy.state = State::Walk {
                        aggro_timer,
                        aggro_entity: *player,
                        aggro_nearest_node: NodeIndex::end(),
                        walk_path: VecDeque::new(),
                        walk_target: None,
                    };
                } else {
                    enemy.animation = Some(Animation::Idle);
                }
            }
            State::Walk {
                mut aggro_timer,
                aggro_entity,
                mut aggro_nearest_node,
                mut walk_path,
                mut walk_target,
            } => {
                if aggro_timer <= 0.0 {
                    enemy.state = State::Idle;
                    continue;
                }

                aggro_timer -= time.delta_secs();

                let aggro_pos = transforms.get(aggro_entity).unwrap().translation.xz();
                let nearest_node = level.nearest_id_terrain(1, aggro_pos)[0];

                if aggro_nearest_node != nearest_node {
                    aggro_nearest_node = nearest_node;
                    let (_, new_path) = astar(
                        &level.graph,
                        nearest_node,
                        |id| id == nearest_node,
                        |e| *e.weight(),
                        |_| 0.0,
                    )
                    .unwrap();
                    walk_path = new_path
                        .into_iter()
                        .map(|node| *level.graph.node_weight(node).unwrap())
                        .collect();
                }

                walk_path.push_back(aggro_pos + level.normal_2d(aggro_pos) * physics.radius);

                while let Some(target) = walk_path.pop_front() {
                    let Some(dir) = (target - pos).try_normalize() else {
                        continue;
                    };

                    let can_pass = {
                        let mut march_pos = pos;
                        loop {
                            let max_dist = -level.height(march_pos);
                            if march_pos.distance(target) <= max_dist {
                                break true;
                            }
                            if max_dist < physics.radius * 0.9 {
                                break false;
                            }
                            march_pos += dir * max_dist;
                        }
                    };

                    if can_pass {
                        walk_target = Some(target);
                    } else {
                        walk_path.push_front(target);
                        break;
                    }
                }

                walk_path.pop_back();

                let dist = aggro_pos.distance(pos);
                if walk_path.is_empty() && dist <= enemy.attack_range {
                    enemy.state = State::Attack {
                        timer_prepare: 0.5,
                        timer_action: 0.5,
                        origin: pos,
                        target: aggro_entity,
                        target_pos: aggro_pos,
                        ranged_done: false,
                    };
                    enemy.animation = Some(Animation::Attack);
                    continue;
                } else {
                    enemy.animation = Some(Animation::Walk);
                }

                if let Some(target) = walk_target {
                    physics.move_vec = target - pos;
                    physics.look_to = physics.look_to.slerp(
                        Dir2::new(-physics.move_vec).unwrap_or(Dir2::NEG_Y),
                        time.delta_secs() * 10.0,
                    );
                    if physics.move_vec.length() < 0.01 {
                        walk_target = None;
                    }
                }

                enemy.state = State::Walk {
                    aggro_timer,
                    aggro_entity,
                    aggro_nearest_node,
                    walk_path,
                    walk_target,
                };
            }
            State::Attack {
                mut timer_prepare,
                mut timer_action,
                origin,
                target,
                mut target_pos,
                mut ranged_done,
            } => {
                let diff = target_pos - origin;
                physics.look_to = Dir2::new(-diff).unwrap_or(Dir2::NEG_Y);

                if timer_prepare > 0.0 {
                    timer_prepare -= time.delta_secs() / enemy.attack_delay;
                    target_pos =
                        transforms.get(target).unwrap().translation.xz() - physics.look_to * 5.0;
                } else if timer_action >= 0.0 {
                    timer_action -= time.delta_secs() / enemy.attack_delay;
                    match enemy.attack {
                        AttackKind::Melee => {
                            physics.move_vec = diff;
                            physics.speed = 2.0 * diff.length() / enemy.attack_delay;
                            physics.ignore_overlap = true;
                        }
                        AttackKind::Ranged if !ranged_done => {
                            ranged_done = true;
                            let shoot_point = global_transforms
                                .get(enemy.scene)
                                .unwrap()
                                .transform_point(enemy.shoot_point);
                            commands.spawn((
                                Transform::from_translation(shoot_point)
                                    .looking_at(target_pos.extend(1.7).xzy(), Vec3::Y),
                                Bullet,
                                Damage::Player,
                            ));
                        }
                        _ => {}
                    }
                } else {
                    enemy.state = State::Idle;
                    continue;
                }

                enemy.state = State::Attack {
                    timer_prepare,
                    timer_action,
                    origin,
                    target,
                    target_pos,
                    ranged_done,
                };
            }
            State::Death => {
                enemy.animation = Some(Animation::Death);
            }
        }

        // println!("{player_pos:?}\n{:?}\n{:?}\n", physics.move_vec, enemy.path);
    }
}
