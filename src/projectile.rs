use bevy::prelude::*;

use crate::level::Level;

pub mod bullet;

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update);
        app.add_systems(Update, bullet::setup);
    }
}

#[derive(Component)]
pub struct Projectile {
    pub speed: f32,
    pub lifetime: f32,
    pub bounces: u32,
}

fn update(
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut Projectile, &mut Transform)>,
    level: Res<Level>,
    time: Res<Time>,
) {
    for (entity, mut projectile, mut transform) in &mut projectiles {
        if projectile.lifetime <= 0.0 || projectile.bounces == 0 {
            commands.entity(entity).despawn();
            continue;
        }
        let delta = time.delta_secs();
        let dir = transform.forward();
        let desired_pos = transform.translation + dir * delta * projectile.speed;
        let new_pos = level.binary_search(transform.translation, desired_pos, 8);
        if new_pos.distance(desired_pos) >= f32::EPSILON {
            transform.look_to(dir.reflect(level.normal_3d(new_pos.xz())), Vec3::Y);
            projectile.bounces -= 1;
        }
        transform.translation = new_pos;
        projectile.lifetime -= delta;
    }
}
