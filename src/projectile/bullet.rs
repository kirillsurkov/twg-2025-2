use bevy::prelude::*;

use crate::projectile::Projectile;

#[derive(Component)]
pub struct Bullet;

pub fn setup(mut commands: Commands, entities: Query<Entity, Added<Bullet>>) {
    for entity in entities {
        commands.entity(entity).insert(Projectile);
    }
}
