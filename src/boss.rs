use bevy::prelude::*;

use crate::{
    model_loader::{LoadModel, ReadyAction},
    player::Player,
    projectile::{Damage, Projectile, SpawnProjectile},
    ui::UserNotify, GameState,
};

pub struct BossPlugin;

impl Plugin for BossPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, setup);
        app.add_systems(Update, animate);
        app.add_systems(Update, update);
    }
}

#[derive(Component)]
pub struct BossSpawner;

fn setup(mut commands: Commands, spawners: Query<Entity, Added<BossSpawner>>) {
    for spawner in spawners {
        commands.entity(spawner).insert(LoadModel::new(
            "boss",
            ReadyAction::Boss,
            Vec3::splat(5.0),
        ));
    }
}

#[derive(Component)]
pub struct Boss {
    pub attack_delay: f32,
    pub timer: f32,
    pub max_hp: f32,
    pub hp: f32,
}

fn animate(
    mut bosses: Query<&mut Transform, (With<Boss>, Without<Player>)>,
    player: Single<&Transform, With<Player>>,
) {
    for mut transform in &mut bosses {
        transform.translation.y = 40.0;
        transform.look_at(player.translation, Vec3::Y);
    }
}

fn update(
    mut commands: Commands,
    player: Single<(&mut Player, &Transform)>,
    mut bosses: Query<(Entity, &mut Boss, &Transform)>,
    mut user_notify: EventWriter<UserNotify>,
    projectiles: Query<(Entity, &Projectile, &Transform)>,
    time: Res<Time>,
    mut game_state: ResMut<GameState>,
) {
    let radius = 2.76 * 5.0;

    let aggro_dist = 150.0;
    let (mut player, player_transform) = player.into_inner();

    for (entity, mut boss, transform) in &mut bosses {
        boss.timer += time.delta_secs();
        let mut attack = false;
        if boss.timer >= boss.attack_delay {
            attack = true;
            boss.timer = 0.0;
        }

        let pos = transform.translation.xz().extend(40.0).xzy();

        for (entity, projectile, transform) in projectiles {
            if transform.translation.distance(pos) <= radius {
                commands.entity(entity).despawn();
                boss.hp -= projectile.damage;
                let perc = (100.0 * boss.hp / boss.max_hp) as u32;
                user_notify.write(UserNotify("Здоровье босса".to_string(), format!("{perc}%")));
            }
        }

        if boss.hp <= 0.0 {
            commands.entity(entity).despawn();
            *game_state = GameState::Win;
            return;
        }

        let diff = player_transform.translation - pos + Vec3::new(0.0, 1.7, 0.0);
        let dir = Dir3::new(diff).unwrap();
        let shoot_point = pos + dir * radius * 1.1;
        let can_attack = diff.length() < 100.0;

        if attack && can_attack {
            SpawnProjectile::BossProj.spawn(
                &mut commands,
                Transform::from_translation(shoot_point)
                    .looking_at(player_transform.translation.xz().extend(1.7).xzy(), Vec3::Y),
                Damage::Player,
            );
        }
    }
}
