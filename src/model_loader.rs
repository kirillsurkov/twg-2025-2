use bevy::prelude::*;

use crate::{enemy::Enemy, terrain::Physics, weapon::Weapon};

pub struct ModelLoaderPlugin;

impl Plugin for ModelLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, load_model);
    }
}

#[derive(Component, Clone, Copy)]
pub enum ReadyAction {
    Enemy,
    Weapon(Vec3),
}

#[derive(Component)]
pub struct LoadModel {
    name: String,
    action: ReadyAction,
    scale: Vec3,
}

#[derive(Component)]
enum WaitFor {
    Gltf {
        gltf_handle: Handle<Gltf>,
        action: ReadyAction,
        scale: Vec3,
    },
    Scene {
        scene: Entity,
        graph_handle: Handle<AnimationGraph>,
        action: ReadyAction,
    },
}

impl LoadModel {
    pub fn new(name: &str, action: ReadyAction, scale: Vec3) -> Self {
        Self {
            name: name.to_string(),
            action,
            scale,
        }
    }
}

fn load_model(
    mut commands: Commands,
    assets: Res<AssetServer>,
    assets_gltf: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    models: Query<(Entity, &LoadModel)>,
    pending: Query<(Entity, &WaitFor)>,
    children: Query<&Children>,
    anim_players: Query<&AnimationPlayer>,
) {
    for (
        entity,
        LoadModel {
            name,
            action,
            scale,
        },
    ) in models
    {
        commands
            .entity(entity)
            .remove::<LoadModel>()
            .insert(WaitFor::Gltf {
                gltf_handle: assets.load(format!("./models/{name}.glb")),
                action: *action,
                scale: *scale,
            });
    }

    for (entity, wait_for) in pending {
        match wait_for {
            WaitFor::Gltf {
                gltf_handle,
                action,
                scale,
            } => {
                if let Some(gltf) = assets_gltf.get(gltf_handle) {
                    let scene = commands
                        .spawn((
                            SceneRoot(gltf.scenes[0].clone()),
                            Transform::from_scale(*scale),
                        ))
                        .id();
                    commands
                        .entity(entity)
                        .insert((
                            WaitFor::Scene {
                                scene,
                                graph_handle: graphs.add(
                                    match action {
                                        ReadyAction::Enemy => AnimationGraph::from_clips([
                                            gltf.named_animations["idle"].clone(),
                                            gltf.named_animations["walk"].clone(),
                                            gltf.named_animations["attack"].clone(),
                                            gltf.named_animations["death"].clone(),
                                        ]),
                                        ReadyAction::Weapon(_) => AnimationGraph::from_clips([
                                            gltf.named_animations["idle"].clone(),
                                            gltf.named_animations["shoot"].clone(),
                                        ]),
                                    }
                                    .0,
                                ),
                                action: *action,
                            },
                            Visibility::default(),
                        ))
                        .add_child(scene);
                }
            }
            WaitFor::Scene {
                scene,
                graph_handle,
                action,
            } => {
                commands.entity(entity).remove::<WaitFor>();
                match action {
                    ReadyAction::Enemy => {
                        let Some(entity_anim_player) = children
                            .iter_descendants(entity)
                            .chain([entity])
                            .find(|e| anim_players.contains(*e))
                        else {
                            continue;
                        };

                        commands
                            .entity(entity_anim_player)
                            .insert(AnimationGraphHandle(graph_handle.clone()))
                            .insert(AnimationTransitions::new());

                        commands
                            .entity(entity)
                            .insert(Enemy::new(entity_anim_player))
                            .insert(Physics::new(0.5, 5.0));
                    }
                    ReadyAction::Weapon(offset) => {
                        let Some(entity_anim_player) = children
                            .iter_descendants(entity)
                            .chain([entity])
                            .find(|e| anim_players.contains(*e))
                        else {
                            continue;
                        };

                        commands
                            .entity(entity_anim_player)
                            .insert(AnimationGraphHandle(graph_handle.clone()))
                            .insert(AnimationTransitions::new());

                        commands.entity(entity).insert(Weapon::new(
                            *scene,
                            entity_anim_player,
                            *offset,
                        ));
                    }
                }
            }
        }
    }
}
