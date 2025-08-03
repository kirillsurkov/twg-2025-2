use bevy::prelude::*;

use crate::{
    model_loader::{LoadModel, ReadyAction},
    projectile::SpawnProjectile,
};

#[derive(Component)]
pub struct Biogun;

pub fn setup(mut commands: Commands, entities: Query<Entity, Added<Biogun>>) {
    for entity in entities {
        commands.entity(entity).insert(LoadModel::new(
            "gun5",
            ReadyAction::Weapon {
                offset: Vec3::new(1.0, -1.5, -2.0),
                shoot_delay: 0.5,
                projectile: SpawnProjectile::Bullet,
            },
            Vec3::splat(0.5),
        ));
    }
}
