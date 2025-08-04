use bevy::prelude::*;

use crate::{
    enemy::AttackKind,
    model_loader::{LoadModel, ReadyAction},
};

#[derive(Component)]
pub struct Spider;

pub fn setup(mut commands: Commands, entities: Query<Entity, Added<Spider>>) {
    for entity in entities {
        commands.entity(entity).insert(LoadModel::new(
            "spider",
            ReadyAction::Enemy {
                attack: AttackKind::Melee(15.0),
                attack_range: 15.0,
                attack_delay: 1.0,
                speed: 5.0,
                hp: 90.0,
            },
            Vec3::splat(1.0),
        ));
    }
}
