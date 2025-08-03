use std::f32::consts::TAU;

use bevy::prelude::*;

use crate::{
    model_loader::{LoadModel, ReadyAction},
    player::Player,
    ui::UserNotify,
};

pub struct HeartPlugin;

impl Plugin for HeartPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, setup);
        app.add_systems(Update, animate);
        app.add_systems(Update, update);
    }
}

#[derive(Component)]
pub struct HeartSpawner;

fn setup(mut commands: Commands, spawners: Query<Entity, Added<HeartSpawner>>) {
    for spawner in spawners {
        commands.entity(spawner).insert(LoadModel::new(
            "Heart",
            ReadyAction::Heart,
            Vec3::new(1.5, 1.5, 3.0),
        ));
    }
}

#[derive(Component)]
pub struct Heart;

fn animate(mut hearts: Query<&mut Transform, With<Heart>>, time: Res<Time>) {
    for mut transform in &mut hearts {
        let angle = 1.0 * time.elapsed_secs() * TAU;
        transform.translation.y = 0.5 * (0.5 * angle.sin() + 0.5);
        transform.rotation = Quat::from_rotation_y(angle);
    }
}

fn update(
    mut commands: Commands,
    player: Single<(&mut Player, &Transform)>,
    hearts: Query<(Entity, &Transform), With<Heart>>,
    mut user_notify: EventWriter<UserNotify>,
) {
    let pickup_dist = 3.0;
    let (mut player, player_transform) = player.into_inner();

    for (entity, transform) in hearts {
        let can_pickup = transform
            .translation
            .xz()
            .distance(player_transform.translation.xz())
            <= pickup_dist;
        if can_pickup {
            user_notify.write(UserNotify("Чтобы восполнить здоровье".to_string()));
        }
        if can_pickup && player.interaction {
            commands.entity(entity).despawn();
            player.max_hp += 100.0;
            player.hp = player.max_hp;
        }
    }
}
