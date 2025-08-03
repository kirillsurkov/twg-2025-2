use bevy::prelude::*;

use crate::{
    enemy::AttackKind,
    model_loader::{LoadModel, ReadyAction},
};

#[derive(Component)]
pub struct Seal;

pub fn setup(mut commands: Commands, entities: Query<Entity, Added<Seal>>) {
    for entity in entities {
        commands.entity(entity).insert(LoadModel::new(
            "seal",
            ReadyAction::Enemy {
                attack: AttackKind::Melee(10.0),
                attack_range: 20.0,
                attack_delay: 2.0,
                speed: 5.0,
                hp: 30.0,
            },
            Vec3::splat(0.75),
        ));
    }
}
