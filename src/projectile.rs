use bevy::{math::bounding::Aabb3d, prelude::*};

use crate::{
    DeferDespawn,
    enemy::Enemy,
    level::Level,
    player::Player,
    projectile::{bullet::Bullet, detonation_bolt::DetonationBolt, explosion::Explosion},
    terrain::Physics,
};

pub mod bullet;
pub mod detonation_bolt;
pub mod explosion;

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, setup);
        app.add_systems(Update, update.after(setup));
        app.add_systems(Update, bullet::setup);
        app.add_systems(Update, detonation_bolt::setup);
        app.add_systems(Update, explosion::setup);
    }
}

#[derive(Clone, Copy)]
pub enum SpawnProjectile {
    Bullet,
    DetonationBolt,
    Explosion,
}

impl SpawnProjectile {
    pub fn spawn(&self, commands: &mut Commands, transform: Transform, damage: Damage) {
        let mut entity = commands.spawn(transform);
        match self {
            Self::Bullet => entity.insert(Bullet),
            Self::DetonationBolt => entity.insert(DetonationBolt),
            Self::Explosion => entity.insert(Explosion),
        };
        match self {
            Self::Explosion => entity.insert(Damage::All),
            _ => entity.insert(damage),
        };
    }
}

#[derive(Component)]
pub struct Projectile {
    pub speed: f32,
    pub velocity: Vec3,
    pub aceleration: Vec3,
    pub lifetime: f32,
    pub particle_lifetime: f32,
    pub bounces: i32,
    pub damage: f32,
    pub on_bounce: Option<SpawnProjectile>,
}

#[derive(Component, Clone, Copy)]
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

fn aabb_sphere_intersection(aabb: Aabb3d, center: Vec3, radius: f32) -> bool {
    let mut dmin = 0.0;

    for i in 0..3 {
        if center[i] < aabb.min[i] {
            dmin += (center[i] - aabb.min[i]).powi(2);
        } else if center[i] > aabb.max[i] {
            dmin += (center[i] - aabb.max[i]).powi(2);
        }
    }

    dmin <= radius.powi(2)
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

        let delta_vel = projectile.aceleration * delta;
        projectile.velocity += delta_vel;

        let offset = (dir * projectile.speed + projectile.velocity) * delta;
        let desired_pos = pos + offset;

        let mut hit = None;
        for (entity, _) in level.nearest_creatures(5, pos) {
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
            let step = (to - from) / 10.0;

            if (0..=10)
                .any(|i| aabb_sphere_intersection(physics.hitbox, from + step * i as f32, 0.1))
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
            projectile.velocity = -projectile.velocity;
            if let Some(action) = projectile.on_bounce {
                action.spawn(&mut commands, transform.clone(), *damage);
            }
        }
        transform.translation = new_pos;
        projectile.lifetime -= delta;
    }
}
