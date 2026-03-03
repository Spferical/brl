use std::time::Duration;

use bevy::prelude::*;

use crate::game::{DAMAGE_Z, assets::WorldAssets};

const MAX_TICK: Duration = Duration::from_nanos(1_000_000_000 / 30);

#[derive(Component, Debug)]
pub struct MoveAnimation {
    pub from: Vec3,
    pub to: Vec3,
    pub timer: Timer,
    pub ease: EaseFunction,
    pub rotation: Option<f32>,
    pub sway: Option<f32>,
}

pub fn process_move_animations(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut MoveAnimation)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut animation) in query.iter_mut() {
        animation.timer.tick(time.delta().min(MAX_TICK));
        let fraction = animation.timer.fraction();
        let MoveAnimation { from, to, ease, .. } = *animation;
        transform.translation = EasingCurve::new(from, to, ease).sample_clamped(fraction);

        let mut rotation = 0.0;
        if let Some(total_rotation) = animation.rotation {
            rotation += total_rotation * fraction;
        }
        if let Some(sway_angle) = animation.sway {
            rotation += (fraction * std::f32::consts::PI).sin() * sway_angle;
        }
        transform.rotation = Quat::from_rotation_z(rotation);

        if animation.timer.is_finished() {
            commands.entity(entity).try_remove::<MoveAnimation>();
        }
    }
}

#[derive(Component, Debug)]
pub struct AttackAnimation {
    pub direction: Vec2,
    pub timer: Timer,
    pub base_translation: Vec3,
}

pub fn process_attack_animations(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut AttackAnimation)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut animation) in query.iter_mut() {
        animation.timer.tick(time.delta().min(MAX_TICK));
        let fraction = animation.timer.fraction();

        let bump = (fraction * std::f32::consts::PI).sin();
        let displacement_amount = bump * 12.0;

        let stretch = 1.0 + bump * 0.4;
        let squash = 1.0 - bump * 0.2;

        if animation.direction.x.abs() > animation.direction.y.abs() {
            transform.scale = Vec3::new(stretch, squash, 1.0);
        } else {
            transform.scale = Vec3::new(squash, stretch, 1.0);
        }

        transform.translation.x =
            animation.base_translation.x + animation.direction.x * displacement_amount;
        transform.translation.y =
            animation.base_translation.y + animation.direction.y * displacement_amount;
        transform.translation.z = animation.base_translation.z;

        if animation.timer.is_finished() {
            transform.scale = Vec3::ONE;
            transform.translation = animation.base_translation;
            commands.entity(entity).try_remove::<AttackAnimation>();
        }
    }
}

#[derive(Message)]
pub struct DamageAnimationMessage {
    pub entity: Entity,
}

pub fn spawn_damage_animations(
    mut commands: Commands,
    mut messages: MessageReader<DamageAnimationMessage>,
    assets: Res<WorldAssets>,
) {
    let sprite = assets.get_ascii_sprite('!', Color::srgb(1.0, 0.0, 0.0));
    for DamageAnimationMessage { entity } in messages.read() {
        let mut transform = Transform::IDENTITY;
        transform.translation.z = DAMAGE_Z;
        if commands.get_entity(*entity).is_ok() {
            let id = commands
                .spawn((
                    sprite.clone(),
                    transform,
                    DamageAnimation(Timer::from_seconds(0.5, TimerMode::Once)),
                ))
                .id();
            commands.entity(*entity).add_child(id);
        }
    }
}

#[derive(Component)]
pub struct DamageAnimation(pub Timer);

pub fn update_damage_animations(
    mut commands: Commands,
    mut query: Query<(Entity, &mut DamageAnimation, &mut TextColor)>,
    time: Res<Time>,
) {
    for (entity, mut anim, mut color) in query.iter_mut() {
        anim.0.tick(time.delta().min(MAX_TICK));
        let ease =
            EasingCurve::new(1.0, 0.0, EaseFunction::CubicOut).sample_clamped(anim.0.fraction());
        color.0.set_alpha(ease);
        if anim.0.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
