use bevy::prelude::*;

use crate::{
    model_loader::{LoadModel, ReadyAction},
    projectile::SpawnProjectile,
};

#[derive(Component)]
pub struct IonCannon;

pub fn setup(mut commands: Commands, entities: Query<Entity, Added<IonCannon>>) {
    for entity in entities {
        commands.entity(entity).insert(LoadModel::new(
            "gun4",
            ReadyAction::Weapon {
                offset: Vec3::new(2.0, -2.5, -3.0),
                shoot_delay: 0.5,
                projectile: SpawnProjectile::IonCannonProj,
            },
            Vec3::splat(0.5),
        ));
    }
}
