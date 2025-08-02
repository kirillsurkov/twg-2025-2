use bevy::prelude::*;

use crate::model_loader::{LoadModel, ReadyAction};

#[derive(Component)]
pub struct PulseRifle;

pub fn setup(mut commands: Commands, entities: Query<Entity, Added<PulseRifle>>) {
    for entity in entities {
        commands.entity(entity).insert(LoadModel::new(
            "gun3",
            ReadyAction::Weapon {
                offset: Vec3::new(2.5, -1.5, -4.0),
                shoot_delay: 0.5,
            },
            Vec3::splat(0.2),
        ));
    }
}
