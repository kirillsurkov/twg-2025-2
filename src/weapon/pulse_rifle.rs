use bevy::prelude::*;

use crate::{
    model_loader::{LoadModel, ReadyAction},
    projectile::SpawnProjectile,
};

#[derive(Component)]
pub struct PulseRifle;

pub fn setup(mut commands: Commands, entities: Query<Entity, Added<PulseRifle>>) {
    for entity in entities {
        commands.entity(entity).insert(LoadModel::new(
            "gun3",
            ReadyAction::Weapon {
                offset: Vec3::new(1.5, -2.3, -2.5),
                shoot_delay: 0.25,
                projectile: SpawnProjectile::PulseRifleProj,
            },
            Vec3::splat(0.15),
        ));
    }
}
