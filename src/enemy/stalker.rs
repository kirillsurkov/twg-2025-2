use bevy::prelude::*;

use crate::{
    enemy::AttackKind,
    model_loader::{LoadModel, ReadyAction},
};

#[derive(Component)]
pub struct Stalker;

pub fn setup(mut commands: Commands, entities: Query<Entity, Added<Stalker>>) {
    for entity in entities {
        commands.entity(entity).insert(LoadModel::new(
            "stalker",
            ReadyAction::Enemy {
                attack: AttackKind::Ranged,
                attack_range: 15.0,
                speed: 5.0,
            },
            Vec3::splat(0.5),
        ));
    }
}
