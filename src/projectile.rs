use bevy::{math::bounding::Aabb3d, prelude::*, render::view::NoFrustumCulling};

use crate::{
    DeferDespawn, GameState,
    enemy::Enemy,
    level::Level,
    player::Player,
    projectile::{
        beetle_proj::BeetleProj, biogun_proj::BiogunProj, blaster_proj::BlasterProj,
        boss_proj::BossProj, bullet::Bullet, detonation_bolt::DetonationBolt, explosion::Explosion,
        ioncannon_proj::IonCannonProj, pulserifle_proj::PulseRifleProj, stalker_proj::StalkerProj,
        tree_proj::TreeProj, turret_proj::TurretProj, wormbeak_proj::WormbeakProj,
        zapper_proj::ZapperProj,
    },
    terrain::Physics,
};

pub mod beetle_proj;
pub mod biogun_proj;
pub mod blaster_proj;
pub mod boss_proj;
pub mod bullet;
pub mod detonation_bolt;
pub mod explosion;
pub mod ioncannon_proj;
pub mod pulserifle_proj;
pub mod stalker_proj;
pub mod tree_proj;
pub mod turret_proj;
pub mod wormbeak_proj;
pub mod zapper_proj;

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, setup);
        app.add_systems(Update, update.after(setup));
        app.add_systems(Update, beetle_proj::setup);
        app.add_systems(Update, biogun_proj::setup);
        app.add_systems(Update, blaster_proj::setup);
        app.add_systems(Update, boss_proj::setup);
        app.add_systems(Update, bullet::setup);
        app.add_systems(Update, detonation_bolt::setup);
        app.add_systems(Update, explosion::setup);
        app.add_systems(Update, ioncannon_proj::setup);
        app.add_systems(Update, pulserifle_proj::setup);
        app.add_systems(Update, stalker_proj::setup);
        app.add_systems(Update, tree_proj::setup);
        app.add_systems(Update, turret_proj::setup);
        app.add_systems(Update, wormbeak_proj::setup);
        app.add_systems(Update, zapper_proj::setup);
    }
}

#[derive(Clone, Copy)]
pub enum SpawnProjectile {
    Bullet,
    BeetleProj,
    BiogunProj,
    BlasterProj,
    BossProj,
    DetonationBolt,
    Explosion,
    IonCannonProj,
    PulseRifleProj,
    StalkerProj,
    TreeProj,
    TurretProj,
    WormbeakProj,
    ZapperProj,
}

impl SpawnProjectile {
    pub fn spawn(&self, commands: &mut Commands, transform: Transform, damage: Damage) {
        let mut entity = commands.spawn((transform, NoFrustumCulling));
        match self {
            Self::Bullet => entity.insert(Bullet),
            Self::BeetleProj => entity.insert(BeetleProj),
            Self::BiogunProj => entity.insert(BiogunProj),
            Self::BlasterProj => entity.insert(BlasterProj),
            Self::BossProj => entity.insert(BossProj),
            Self::DetonationBolt => entity.insert(DetonationBolt),
            Self::Explosion => entity.insert(Explosion),
            Self::IonCannonProj => entity.insert(IonCannonProj),
            Self::PulseRifleProj => entity.insert(PulseRifleProj),
            Self::StalkerProj => entity.insert(StalkerProj),
            Self::TreeProj => entity.insert(TreeProj),
            Self::TurretProj => entity.insert(TurretProj),
            Self::WormbeakProj => entity.insert(WormbeakProj),
            Self::ZapperProj => entity.insert(ZapperProj),
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
    pub radius: f32,
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
    game_state: Res<GameState>,
) {
    if !matches!(*game_state, GameState::Running) {
        return;
    }

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

            if (0..=10).any(|i| {
                aabb_sphere_intersection(physics.hitbox, from + step * i as f32, projectile.radius)
            }) {
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
            projectile.velocity = -projectile.velocity * 0.5;
            if let Some(action) = projectile.on_bounce {
                action.spawn(&mut commands, transform.clone(), *damage);
            }
        }
        transform.translation = new_pos;
        projectile.lifetime -= delta;
    }
}
