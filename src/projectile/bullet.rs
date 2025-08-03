use bevy::prelude::*;
use bevy_hanabi::{
    Attribute, EffectAsset, ExprWriter, Gradient, OrientMode, OrientModifier, ParticleEffect,
    SetAttributeModifier, SetPositionSphereModifier, SetVelocitySphereModifier, ShapeDimension,
    SizeOverLifetimeModifier, SpawnerSettings,
};

use crate::projectile::Projectile;

#[derive(Component)]
pub struct Bullet;

pub fn setup(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    mut effect: Local<Option<Handle<EffectAsset>>>,
    entities: Query<Entity, Added<Bullet>>,
) {
    let particle_lifetime = 0.2;
    let effect = effect.get_or_insert({
        let writer = ExprWriter::new();
        let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
        let init_lifetime =
            SetAttributeModifier::new(Attribute::LIFETIME, writer.lit(particle_lifetime).expr());
        let init_pos = SetPositionSphereModifier {
            center: writer.lit(Vec3::ZERO).expr(),
            radius: writer.lit(0.01).expr(),
            dimension: ShapeDimension::Volume,
        };
        let init_vel = SetVelocitySphereModifier {
            center: writer.lit(Vec3::ZERO).expr(),
            speed: writer.lit(0.1).expr(),
        };
        let init_ribbon_id = SetAttributeModifier {
            attribute: Attribute::RIBBON_ID,
            value: writer.lit(0u32).expr(),
        };
        effects.add(
            EffectAsset::new(
                64,
                SpawnerSettings::rate((64.0 / particle_lifetime).into()),
                writer.finish(),
            )
            .with_name("Bullet")
            .init(init_age)
            .init(init_lifetime)
            .init(init_pos)
            .init(init_vel)
            .init(init_ribbon_id)
            .render(OrientModifier {
                mode: OrientMode::FaceCameraPosition,
                rotation: None,
            })
            .render(SizeOverLifetimeModifier {
                gradient: Gradient::linear(Vec3::splat(0.1), Vec3::ZERO),
                screen_space_size: false,
            }),
        )
    });
    for entity in entities {
        commands.entity(entity).insert((
            Projectile {
                speed: 50.0,
                velocity: Vec3::ZERO,
                aceleration: Vec3::ZERO,
                lifetime: 3.0,
                particle_lifetime,
                bounces: 3,
                damage: 3.0,
                on_bounce: None,
            },
            ParticleEffect::new(effect.clone_weak()),
        ));
    }
}
