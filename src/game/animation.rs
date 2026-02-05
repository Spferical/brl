use bevy::prelude::*;

use crate::game::{GameWorld, assets::WorldAssets};

#[derive(Component, Debug)]
pub struct MoveAnimation {
    pub from: Vec3,
    pub to: Vec3,
    pub timer: Timer,
    pub ease: EaseFunction,
    pub rotation: Option<f32>,
}

pub fn process_move_animations(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut MoveAnimation)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut animation) in query.iter_mut() {
        animation.timer.tick(time.delta());
        let fraction = animation.timer.fraction();
        let MoveAnimation { from, to, ease, .. } = *animation;
        transform.translation = EasingCurve::new(from, to, ease).sample_clamped(fraction);
        if let Some(total_rotation) = animation.rotation {
            transform.rotation = Quat::from_rotation_z(total_rotation * fraction);
        }
        if animation.timer.is_finished() {
            commands.entity(entity).try_remove::<MoveAnimation>();
        }
    }
}

#[derive(Message)]
pub struct DamageAnimationMessage {
    pub entity: Entity,
}

pub fn spawn_damage_animations(
    world: Single<Entity, With<GameWorld>>,
    mut commands: Commands,
    mut messages: MessageReader<DamageAnimationMessage>,
    q_transform: Query<&Transform>,
    assets: Res<WorldAssets>,
) {
    let sprite = assets.get_urizen_sprite(5150);
    for DamageAnimationMessage { entity } in messages.read() {
        if let Ok(transform) = q_transform.get(*entity) {
            let mut transform = *transform;
            transform.translation.z += 1.0;
            let id = commands
                .spawn((
                    sprite.clone(),
                    transform,
                    DamageAnimation(Timer::from_seconds(0.5, TimerMode::Once)),
                ))
                .id();
            commands.entity(*world).add_child(id);
        }
    }
}

#[derive(Component)]
pub struct DamageAnimation(pub Timer);

pub fn update_damage_animations(
    mut commands: Commands,
    mut query: Query<(Entity, &mut DamageAnimation, &mut Sprite)>,
    time: Res<Time>,
) {
    for (entity, mut anim, mut sprite) in query.iter_mut() {
        anim.0.tick(time.delta());
        let ease =
            EasingCurve::new(1.0, 0.0, EaseFunction::CubicOut).sample_clamped(anim.0.fraction());
        sprite.color.set_alpha(ease);
        if anim.0.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
