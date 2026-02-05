use bevy::prelude::*;

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
