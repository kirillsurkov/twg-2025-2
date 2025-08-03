use bevy::prelude::*;

use crate::{
    enemy::AttackKind,
    model_loader::{LoadModel, ReadyAction},
    projectile::SpawnProjectile,
};

#[derive(Component)]
pub struct Wormbeak;

pub fn setup(mut commands: Commands, entities: Query<Entity, Added<Wormbeak>>) {
    for entity in entities {
        commands.entity(entity).insert(LoadModel::new(
            "wormbeak",
            ReadyAction::Enemy {
                attack: AttackKind::Ranged(SpawnProjectile::Bullet),
                attack_range: 15.0,
                attack_delay: 0.5,
                speed: 5.0,
                hp: 25.0,
            },
            Vec3::splat(0.5),
        ));
    }
}
