use bevy::prelude::*;

use crate::model_loader::{LoadModel, ReadyAction};

#[derive(Component)]
pub struct Spider;

pub fn setup(mut commands: Commands, entities: Query<Entity, Added<Spider>>) {
    for entity in entities {
        commands.entity(entity).insert(LoadModel::new(
            "spider",
            ReadyAction::Enemy,
            Vec3::splat(3.0),
        ));
    }
}
