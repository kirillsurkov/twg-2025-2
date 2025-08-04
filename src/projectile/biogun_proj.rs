use bevy::prelude::*;
use bevy_hanabi::{
    Attribute, ColorOverLifetimeModifier, EffectAsset, ExprWriter, Gradient, OrientMode,
    OrientModifier, ParticleEffect, SetAttributeModifier, SetPositionSphereModifier,
    SetVelocitySphereModifier, ShapeDimension, SizeOverLifetimeModifier, SpawnerSettings,
};

use crate::projectile::Projectile;

#[derive(Component)]
pub struct BiogunProj;

pub fn setup(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    mut effect: Local<Option<Handle<EffectAsset>>>,
    asset_server: Res<AssetServer>,
    entities: Query<Entity, Added<BiogunProj>>,
) {
    let particles = 64;
    let particle_lifetime = 0.2;
    let radius = 0.1;
    let color = Vec4::new(237.0 / 255.0, 118.0 / 255.0, 14.0 / 255.0, 1.0);
    let size = 0.05;

    let damage = 10.0;
    let speed = 40.0;

    let effect = effect.get_or_insert({
        let writer = ExprWriter::new();
        let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
        let init_lifetime =
            SetAttributeModifier::new(Attribute::LIFETIME, writer.lit(particle_lifetime).expr());
        let init_pos = SetPositionSphereModifier {
            center: writer.lit(Vec3::ZERO).expr(),
            radius: writer.lit(0.1).expr(),
            dimension: ShapeDimension::Volume,
        };
        let init_vel = SetVelocitySphereModifier {
            center: writer.lit(Vec3::ZERO).expr(),
            speed: writer.lit(radius).expr(),
        };
        let init_ribbon_id = SetAttributeModifier {
            attribute: Attribute::RIBBON_ID,
            value: writer.lit(0u32).expr(),
        };
        effects.add(
            EffectAsset::new(
                particles,
                SpawnerSettings::rate((particles as f32 / particle_lifetime).into()),
                writer.finish(),
            )
            .with_name("BiogunProj")
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
                gradient: Gradient::linear(Vec3::splat(size), Vec3::ZERO),
                screen_space_size: false,
            })
            .render(ColorOverLifetimeModifier::new(Gradient::from_keys([
                (0.0, Vec4::ONE),
                (0.1, color),
                (0.8, Vec4::ZERO),
            ]))),
        )
    });
    for entity in entities {
        commands.entity(entity).insert((
            Projectile {
                speed,
                velocity: Vec3::ZERO,
                aceleration: Vec3::ZERO,
                lifetime: 3.0,
                particle_lifetime,
                bounces: 3,
                damage,
                radius: 0.1,
                on_bounce: None,
            },
            ParticleEffect::new(effect.clone_weak()),
            AudioPlayer::new(asset_server.load("sounds/2.wav")),
            PlaybackSettings {
                spatial: true,
                ..Default::default()
            },
        ));
    }
}
