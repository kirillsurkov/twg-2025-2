use bevy::{audio::Volume, prelude::*};
use bevy_hanabi::{
    Attribute, EffectAsset, ExprWriter, Gradient, OrientMode, OrientModifier, ParticleEffect,
    SetAttributeModifier, SetPositionSphereModifier, SetVelocitySphereModifier, ShapeDimension,
    SizeOverLifetimeModifier, SpawnerSettings,
};

use crate::projectile::Projectile;

#[derive(Component)]
pub struct Explosion;

pub fn setup(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    mut effect: Local<Option<Handle<EffectAsset>>>,
    entities: Query<Entity, Added<Explosion>>,
    asset_server: Res<AssetServer>,
) {
    let radius = 5.0;
    let particle_lifetime = 0.5;
    let effect = effect.get_or_insert({
        let writer = ExprWriter::new();
        let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
        let init_lifetime =
            SetAttributeModifier::new(Attribute::LIFETIME, writer.lit(particle_lifetime).expr());
        let init_pos = SetPositionSphereModifier {
            center: writer.lit(Vec3::ZERO).expr(),
            radius: writer.lit(0.5).expr(),
            dimension: ShapeDimension::Volume,
        };
        let init_vel = SetVelocitySphereModifier {
            center: writer.lit(Vec3::ZERO).expr(),
            speed: writer.lit(radius / particle_lifetime).expr(),
        };
        effects.add(
            EffectAsset::new(
                512,
                SpawnerSettings::once((512.0 / particle_lifetime).into()),
                writer.finish(),
            )
            .with_name("Explosion")
            .init(init_age)
            .init(init_lifetime)
            .init(init_pos)
            .init(init_vel)
            .render(OrientModifier {
                mode: OrientMode::FaceCameraPosition,
                rotation: None,
            })
            .render(SizeOverLifetimeModifier {
                gradient: Gradient::linear(Vec3::splat(0.02), Vec3::ZERO),
                screen_space_size: false,
            }),
        )
    });
    for entity in entities {
        commands.entity(entity).insert((
            Projectile {
                speed: 0.0,
                velocity: Vec3::ZERO,
                aceleration: Vec3::ZERO,
                lifetime: 0.5,
                particle_lifetime,
                bounces: 0,
                damage: 20.0,
                radius: 1.0,
                on_bounce: None,
            },
            ParticleEffect::new(effect.clone_weak()),
            AudioPlayer::new(asset_server.load("sounds/explosion.wav")),
            PlaybackSettings {
                volume: Volume::Linear(0.5),
                spatial: true,
                ..Default::default()
            },
        ));
    }
}
