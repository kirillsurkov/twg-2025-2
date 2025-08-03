use bevy::prelude::*;

use crate::{
    enemy::AttackKind,
    model_loader::{LoadModel, ReadyAction}, projectile::SpawnProjectile,
};

#[derive(Component)]
pub struct Turret;

pub fn setup(mut commands: Commands, entities: Query<Entity, Added<Turret>>) {
    for entity in entities {
        commands.entity(entity).insert(LoadModel::new(
            "turret",
            ReadyAction::Enemy {
                attack: AttackKind::Ranged(SpawnProjectile::Bullet),
                attack_range: 30.0,
                attack_delay: 0.25,
                speed: 0.0,
                hp: 120.0,
            },
            Vec3::splat(0.5),
        ));
    }
}
