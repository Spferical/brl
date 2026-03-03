use bevy::prelude::*;

use crate::game::Player;

#[derive(Component)]
pub(crate) struct CameraFollow;

pub(crate) fn update_camera(
    mut camera: Single<&mut Transform, (With<Camera2d>, Without<CameraFollow>)>,
    follow: Single<&Transform, (With<CameraFollow>, Without<Camera2d>)>,
    player: Query<&Player>,
    time: Res<Time>,
) {
    let Vec3 { x, y, .. } = follow.translation;
    let target = Vec3::new(x, y, camera.translation.z);
    let t = 1.0 - (-10.0 * time.delta_secs()).exp();
    camera.translation = camera.translation.lerp(target, t);

    if let Some(player) = player.iter().next() {
        let p = ((player.brainrot as f32 - 70.0) / 30.0).clamp(0.0, 1.0);
        if p > 0.0 {
            let max_angle = 5.0_f32.to_radians();
            camera.rotation = Quat::from_rotation_z(p * max_angle);
        } else {
            camera.rotation = Quat::IDENTITY;
        }
    } else {
        camera.rotation = Quat::IDENTITY;
    }
}
