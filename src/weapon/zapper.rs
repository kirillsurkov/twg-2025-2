use bevy::prelude::*;

use crate::{
    model_loader::{LoadModel, ReadyAction},
    projectile::SpawnProjectile,
};

#[derive(Component)]
pub struct Zapper;

pub fn setup(mut commands: Commands, entities: Query<Entity, Added<Zapper>>) {
    for entity in entities {
        commands.entity(entity).insert(LoadModel::new(
            "gun1",
            ReadyAction::Weapon {
                offset: Vec3::new(1.5, -2.0, -1.5),
                shoot_delay: 0.1,
                projectile: SpawnProjectile::ZapperProj,
            },
            Vec3::splat(0.5),
        ));
    }
}
