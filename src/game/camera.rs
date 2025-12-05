use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct CameraFollow;

pub(crate) fn update_camera(
    mut camera: Single<&mut Transform, (With<Camera2d>, Without<CameraFollow>)>,
    follow: Single<&Transform, (With<CameraFollow>, Without<Camera2d>)>,
    time: Res<Time>,
) {
    let Vec3 { x, y, .. } = follow.translation;
    let target = Vec3::new(x, y, camera.translation.z);
    let t = 1.0 - (-10.0 * time.delta_secs()).exp();
    camera.translation = camera.translation.lerp(target, t);
}
