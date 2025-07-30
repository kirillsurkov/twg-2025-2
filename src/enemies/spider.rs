use bevy::prelude::*;

#[derive(Component)]
pub struct Spider;

pub fn update(mut commands: Commands, entities: Query<Entity, Added<Spider>>) {
    for entity in entities {
        commands.entity(entity).insert(());
    }
}
