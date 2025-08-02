use bevy::{math::bounding::Aabb3d, prelude::*};

use crate::model_loader::{LoadModel, ReadyAction};

#[derive(Component)]
pub struct Spider;

pub fn setup(mut commands: Commands, entities: Query<Entity, Added<Spider>>) {
    for entity in entities {
        commands.entity(entity).insert(LoadModel::new(
            "spider",
            ReadyAction::Enemy {
                hitbox: Aabb3d::new(Vec3::new(0.0, 1.2, -0.2), Vec3::new(1.0, 1.2, 2.4) * 0.5),
            },
            Vec3::splat(3.0),
        ));
    }
}
