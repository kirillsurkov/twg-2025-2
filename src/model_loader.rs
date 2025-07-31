use bevy::prelude::*;

use crate::{enemies::Enemy, terrain::Physics};

pub struct ModelLoaderPlugin;

impl Plugin for ModelLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, load_model);
    }
}

#[derive(Component, Clone, Copy)]
pub enum ReadyAction {
    Enemy,
}

#[derive(Component)]
pub struct LoadModel {
    name: String,
    action: ReadyAction,
}

#[derive(Component)]
enum WaitFor {
    Gltf(Handle<Gltf>, ReadyAction),
    Scene(Handle<AnimationGraph>, ReadyAction),
}

impl LoadModel {
    pub fn new(name: &str, action: ReadyAction) -> Self {
        Self {
            name: name.to_string(),
            action,
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
    for (entity, LoadModel { name, action }) in models {
        let handle = assets.load(format!("./models/{name}.glb"));

        commands
            .entity(entity)
            .remove::<LoadModel>()
            .insert(WaitFor::Gltf(handle, *action));
    }

    for (entity, wait_for) in pending {
        match wait_for {
            WaitFor::Gltf(handle, action) => match action {
                ReadyAction::Enemy => {
                    println!("WaitFor Gltf");
                    if let Some(gltf) = assets_gltf.get(handle) {
                        let (graph, _) = AnimationGraph::from_clips([
                            gltf.named_animations["idle"].clone(),
                            gltf.named_animations["walk"].clone(),
                            gltf.named_animations["attack"].clone(),
                            gltf.named_animations["death"].clone(),
                        ]);
                        let handle = graphs.add(graph);

                        commands
                            .entity(entity)
                            .insert(SceneRoot(gltf.scenes[0].clone()))
                            .insert(WaitFor::Scene(handle, *action));
                    }
                }
            },
            WaitFor::Scene(handle, action) => match action {
                ReadyAction::Enemy => {
                    println!("WaitFor Scene");
                    let Some(entity_anim_player) = children
                        .iter_descendants(entity)
                        .chain([entity])
                        .find(|e| anim_players.contains(*e))
                    else {
                        continue;
                    };

                    commands
                        .entity(entity)
                        .remove::<WaitFor>()
                        .insert(Enemy::new(entity_anim_player))
                        .insert(Physics::new(0.5, 5.0));
                    commands
                        .entity(entity_anim_player)
                        .insert(AnimationGraphHandle(handle.clone()))
                        .insert(AnimationTransitions::new());
                }
            },
        }
    }
}
