use bevy::prelude::*;

use crate::{
    enemy::AttackKind,
    model_loader::{LoadModel, ReadyAction},
};

#[derive(Component)]
pub struct Glutton;

pub fn setup(mut commands: Commands, entities: Query<Entity, Added<Glutton>>) {
    for entity in entities {
        commands.entity(entity).insert(LoadModel::new(
            "glutton",
            ReadyAction::Enemy {
                attack: AttackKind::Melee(25.0),
                attack_range: 20.0,
                attack_delay: 2.0,
                speed: 5.0,
                hp: 600.0,
            },
            Vec3::splat(0.25),
        ));
    }
}
