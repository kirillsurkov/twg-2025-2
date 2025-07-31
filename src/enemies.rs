use std::{collections::VecDeque, time::Duration};

use bevy::prelude::*;
use bresenham::Bresenham;
use petgraph::{algo::astar, graph::NodeIndex};

use crate::{level::Level, player::Player, terrain::Physics};

pub mod spider;

pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, animate);
        app.add_systems(Update, ai);
        app.add_systems(Update, spider::update);
    }
}

#[derive(Component)]
pub struct Enemy {
    anim_player: Entity,
    aggro_timer: f32,
    path: VecDeque<Vec2>,
    target: Option<Vec2>,
}

impl Enemy {
    pub fn new(anim_player: Entity) -> Self {
        Self {
            anim_player,
            aggro_timer: 0.0,
            path: VecDeque::new(),
            target: None,
        }
    }
}

fn animate(
    enemies: Query<&Enemy>,
    mut anim_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    for enemy in enemies {
        let (mut player, mut transition) = anim_players.get_mut(enemy.anim_player).unwrap();

        let index = AnimationNodeIndex::new(2);

        if !player.is_playing_animation(index) {
            transition
                .play(&mut player, index, Duration::from_millis(250))
                .repeat();
        }
    }
}

fn ai(
    level: Res<Level>,
    player: Single<Entity, With<Player>>,
    transforms: Query<&Transform>,
    mut enemies: Query<(Entity, &mut Enemy, &mut Physics)>,
    mut last_player_nearest: Local<NodeIndex>,
) {
    let player_pos = transforms.get(*player).unwrap().translation.xz();
    let player_nearest = level.nearest_one_id(player_pos);

    let recalculate = player_nearest != *last_player_nearest;
    *last_player_nearest = player_nearest;

    // for (entity, mut enemy, mut physics) in &mut enemies {
    //     let pos = transforms.get(entity).unwrap().translation.xz();
    //     let chase_player = player_pos.distance(pos) < 10.0;

    //     if recalculate {
    //         let (_, path) = astar(
    //             &level.graph,
    //             level.nearest_one_id(pos),
    //             |id| id == player_nearest,
    //             |e| *e.weight(),
    //             |_| 0.0,
    //         )
    //         .unwrap();
    //         enemy.path = path
    //             .into_iter()
    //             .map(|node| *level.graph.node_weight(node).unwrap())
    //             .collect();
    //     }

    //     let IVec2 {
    //         x: x_from,
    //         y: y_from,
    //     } = world_to_texture(
    //         pos,
    //         level.bounds(),
    //         UVec2::from(level.height_map().dimensions()),
    //     )
    //     .as_ivec2();

    //     if chase_player {
    //         enemy.path.push_back(player_pos);
    //     }

    //     while let Some(pos) = enemy.path.pop_front() {
    //         let IVec2 { x: x_to, y: y_to } = world_to_texture(
    //             pos,
    //             level.bounds(),
    //             UVec2::from(level.height_map().dimensions()),
    //         )
    //         .as_ivec2();

    //         let mut can_pass = true;
    //         for (x, y) in Bresenham::new(
    //             (x_from as isize, y_from as isize),
    //             (x_to as isize, y_to as isize),
    //         ) {
    //             can_pass &=
    //                 level.height_map().get_pixel(x as u32, y as u32).0[0] < -physics.radius * 0.8;
    //             if !can_pass {
    //                 break;
    //             }
    //         }

    //         if can_pass {
    //             enemy.target = Some(pos);
    //         } else {
    //             enemy.path.push_front(pos);
    //             break;
    //         }
    //     }

    //     if chase_player {
    //         enemy.path.pop_back();
    //     }

    //     if let Some(target) = enemy.target {
    //         physics.move_vec = target - pos;
    //         physics.look_to = -physics.move_vec;
    //     } else {
    //         physics.move_vec = Vec2::ZERO;
    //     }

    //     // println!("{player_pos:?}\n{:?}\n{:?}\n", physics.move_vec, enemy.path);
    // }
}
