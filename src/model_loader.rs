use bevy::{
    math::bounding::{Aabb3d, BoundingVolume},
    prelude::*,
};

use crate::{
    enemy::{AttackKind, Enemy},
    terrain::Physics,
    weapon::Weapon,
};

pub struct ModelLoaderPlugin;

impl Plugin for ModelLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, load_model);
    }
}

#[derive(Component, Clone, Copy)]
pub enum ReadyAction {
    Enemy {
        attack: AttackKind,
        attack_range: f32,
        speed: f32,
    },
    Weapon {
        offset: Vec3,
        shoot_delay: f32,
    },
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
        name: String,
        gltf_handle: Handle<Gltf>,
        action: ReadyAction,
        scale: Vec3,
    },
    Scene {
        name: String,
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
    names: Query<&Name>,
    transforms: Query<&Transform>,

    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
                name: name.clone(),
                gltf_handle: assets.load(format!("./models/{name}.glb")),
                action: *action,
                scale: *scale,
            });
    }

    for (entity, wait_for) in pending {
        match wait_for {
            WaitFor::Gltf {
                name,
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
                                name: name.clone(),
                                scene,
                                graph_handle: graphs.add(
                                    match action {
                                        ReadyAction::Enemy { .. } => AnimationGraph::from_clips([
                                            gltf.named_animations["idle"].clone(),
                                            gltf.named_animations["walk"].clone(),
                                            gltf.named_animations["attack"].clone(),
                                            gltf.named_animations["death"].clone(),
                                        ]),
                                        ReadyAction::Weapon { .. } => AnimationGraph::from_clips([
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
                name,
                scene,
                graph_handle,
                action,
            } => {
                commands.entity(entity).remove::<WaitFor>();
                match action {
                    ReadyAction::Enemy {
                        attack,
                        attack_range,
                        speed,
                    } => {
                        let mut anim_player = Entity::PLACEHOLDER;
                        let mut hitbox = Entity::PLACEHOLDER;
                        for entity in children.iter_descendants(entity).chain([entity]) {
                            if anim_players.contains(entity) {
                                anim_player = entity;
                            }
                            if let Ok(name) = names.get(entity) {
                                if name.as_str() == "hitbox" {
                                    hitbox = entity;
                                }
                            }
                        }

                        let Ok(_) = anim_players.get(anim_player) else {
                            panic!("Enemy {name} doesn't have an animation player");
                        };

                        let Ok(hitbox) = transforms.get(hitbox) else {
                            panic!("Enemy {name} doesn't have a hitbox");
                        };
                        let hitbox = Aabb3d::new(hitbox.translation * 0.5, hitbox.scale * 0.5);

                        commands
                            .entity(anim_player)
                            .insert(AnimationGraphHandle(graph_handle.clone()))
                            .insert(AnimationTransitions::new());

                        commands
                            .entity(entity)
                            .insert(Enemy::new(anim_player, *attack, *attack_range, *speed))
                            .insert(Physics::new(0.5, 5.0, hitbox))
                            .with_child((
                                Mesh3d(
                                    meshes
                                        .add(Cuboid::from_size((hitbox.half_size() * 2.0).into())),
                                ),
                                MeshMaterial3d(materials.add(Color::WHITE)),
                                Transform::from_translation(hitbox.center().into()),
                                Visibility::default(),
                            ));
                    }
                    ReadyAction::Weapon {
                        offset,
                        shoot_delay,
                    } => {
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

                        let Some(shoot_point) = children
                            .iter_descendants(entity)
                            .chain([entity])
                            .find(|e| {
                                names
                                    .get(*e)
                                    .map(|n| n.as_str() == "shoot_point")
                                    .unwrap_or_default()
                            })
                            .and_then(|e| transforms.get(e).map(|t| t.translation).ok())
                        else {
                            panic!("Weapon {name} doesn't have a shoot point");
                        };

                        commands.entity(entity).insert(Weapon::new(
                            *scene,
                            entity_anim_player,
                            *offset,
                            shoot_point,
                            *shoot_delay,
                        ));
                    }
                }
            }
        }
    }
}
