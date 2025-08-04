use bevy::prelude::*;

use crate::{
    model_loader::{LoadModel, ReadyAction},
    projectile::SpawnProjectile,
};

#[derive(Component)]
pub struct Blaster;

pub fn setup(mut commands: Commands, entities: Query<Entity, Added<Blaster>>) {
    for entity in entities {
        commands.entity(entity).insert(LoadModel::new(
            "gun2",
            ReadyAction::Weapon {
                offset: Vec3::new(2.0, -2.2, -3.0),
                shoot_delay: 0.5,
                projectile: SpawnProjectile::BlasterProj,
            },
            Vec3::splat(0.5),
        ));
    }
}
