use bevy::prelude::*;

use crate::{
    enemy::AttackKind,
    model_loader::{LoadModel, ReadyAction},
};

#[derive(Component)]
pub struct Tree;

pub fn setup(mut commands: Commands, entities: Query<Entity, Added<Tree>>) {
    for entity in entities {
        commands.entity(entity).insert(LoadModel::new(
            "tree",
            ReadyAction::Enemy {
                attack: AttackKind::Ranged,
                attack_range: 20.0,
                attack_delay: 0.5,
                speed: 5.0,
            },
            Vec3::splat(0.25),
        ));
    }
}
