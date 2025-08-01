use bevy::prelude::*;

mod bullet;

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update);
        app.add_systems(Update, bullet::setup);
    }
}

#[derive(Component)]
pub struct Projectile;

fn update(mut projectiles: Query<(&Projectile, &mut Transform)>, time: Res<Time>) {
    for (projectile, mut transform) in &mut projectiles {
        let forward = transform.forward();
        transform.translation += forward * time.delta_secs() * 10.0;
    }
}