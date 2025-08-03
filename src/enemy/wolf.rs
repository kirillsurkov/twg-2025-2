use bevy::prelude::*;

use crate::{
    enemy::AttackKind,
    model_loader::{LoadModel, ReadyAction},
};

#[derive(Component)]
pub struct Wolf;

pub fn setup(mut commands: Commands, entities: Query<Entity, Added<Wolf>>) {
    for entity in entities {
        commands.entity(entity).insert(LoadModel::new(
            "wolf",
            ReadyAction::Enemy {
                attack: AttackKind::Melee(5.0),
                attack_range: 20.0,
                attack_delay: 2.0,
                speed: 5.0,
                hp: 15.0,
            },
            Vec3::splat(2.0),
        ));
    }
}
