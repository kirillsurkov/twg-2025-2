use bevy::{math::bounding::Aabb3d, prelude::*};

use crate::{DeferDespawn, enemy::Enemy, level::Level, player::Player, terrain::Physics};

pub mod bullet;

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, setup);
        app.add_systems(Update, update.after(setup));
        app.add_systems(Update, bullet::setup);
    }
}

#[derive(Component)]
pub struct Projectile {
    pub speed: f32,
    pub lifetime: f32,
    pub particle_lifetime: f32,
    pub bounces: i32,
    pub damage: f32,
}

#[derive(Component)]
pub enum Damage {
    Player,
    Enemy,
    All,
}

#[derive(Component)]
pub struct ApplyDamage(pub f32);

#[derive(Component)]
struct Ready; // 1 frame lag in hanabi?

fn setup(mut commands: Commands, projectiles: Query<Entity, Added<Projectile>>) {
    for entity in projectiles {
        commands.entity(entity).insert(Ready);
    }
}

fn point_in_aabb(point: Vec3, aabb: Aabb3d) -> bool {
    (aabb.min.x..=aabb.max.x).contains(&point.x)
        && (aabb.min.y..=aabb.max.y).contains(&point.y)
        && (aabb.min.z..=aabb.max.z).contains(&point.z)
}

fn aabb_ray_intersection(aabb: Aabb3d, ray: Ray3d) -> Option<f32> {
    let inv_dir = ray.direction.recip();

    let t0s = (Vec3::from(aabb.min) - ray.origin) * inv_dir;
    let t1s = (Vec3::from(aabb.max) - ray.origin) * inv_dir;

    let t_min = t0s.min(t1s);
    let t_max = t0s.max(t1s);

    let t_enter = t_min.max_element();
    let t_exit = t_max.min_element();

    if t_exit >= t_enter.max(0.0) {
        Some(t_enter.max(0.0))
    } else {
        None
    }
}

fn update(
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut Projectile, &Damage, &mut Transform), With<Ready>>,
    transforms: Query<(&GlobalTransform, &Physics, Option<&Player>, Option<&Enemy>)>,
    level: Res<Level>,
    time: Res<Time>,
) {
    for (entity, mut projectile, damage, mut transform) in &mut projectiles {
        if projectile.lifetime <= 0.0 || projectile.bounces < 0 {
            commands
                .entity(entity)
                .remove::<Projectile>()
                .insert(DeferDespawn(projectile.particle_lifetime));
            continue;
        }

        let pos = transform.translation;
        let delta = time.delta_secs();
        let dir = transform.forward();
        let desired_pos = pos + dir * projectile.speed * delta;

        let mut hit = None;
        for (entity, _) in level.nearest_creatures(10, pos) {
            let Ok((transform, physics, player, enemy)) = transforms.get(entity) else {
                continue;
            };

            match (player, enemy, damage) {
                (Some(_), None, Damage::Player) => {}
                (None, Some(_), Damage::Enemy) => {}
                (_, _, Damage::All) => {}
                _ => continue,
            }

            let inverse = transform.compute_matrix().inverse();
            let from = inverse.transform_point3(pos);
            let to = inverse.transform_point3(desired_pos);

            if let Some(intersect) = Dir3::new(to - from)
                .ok()
                .and_then(|dir| {
                    aabb_ray_intersection(physics.hitbox, Ray3d::new(to, -dir))
                        .and_then(|_| aabb_ray_intersection(physics.hitbox, Ray3d::new(from, dir)))
                        .map(|dist| from + dir * dist)
                })
                .or_else(|| point_in_aabb(from, physics.hitbox).then_some(from))
            {
                hit = Some(entity);
                break;
            }
        }

        if let Some(hit) = hit {
            commands
                .entity(entity)
                .remove::<Projectile>()
                .insert(DeferDespawn(projectile.particle_lifetime));
            commands.entity(hit).insert(ApplyDamage(projectile.damage));
            continue;
        }

        let new_pos = level.binary_search(pos, desired_pos, 8);
        if new_pos.distance(desired_pos) >= f32::EPSILON {
            transform.look_to(dir.reflect(level.normal_3d(new_pos.xz())), Vec3::Y);
            projectile.bounces -= 1;
        }
        transform.translation = new_pos;
        projectile.lifetime -= delta;
    }
}
