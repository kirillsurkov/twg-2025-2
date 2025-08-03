use bevy::prelude::*;

use crate::{
    enemy::AttackKind,
    model_loader::{LoadModel, ReadyAction}, projectile::SpawnProjectile,
};

#[derive(Component)]
pub struct Beetle;

pub fn setup(mut commands: Commands, entities: Query<Entity, Added<Beetle>>) {
    for entity in entities {
        commands.entity(entity).insert(LoadModel::new(
            "beetle",
            ReadyAction::Enemy {
                attack: AttackKind::Ranged(SpawnProjectile::Bullet),
                attack_range: 15.0,
                attack_delay: 1.0,
                speed: 5.0,
                hp: 450.0,
            },
            Vec3::splat(0.5),
        ));
    }
}
