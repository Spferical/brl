use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct CameraFollow;

pub(crate) fn update_camera(
    mut camera: Single<&mut Transform, (With<Camera2d>, Without<CameraFollow>)>,
    follow: Single<&Transform, (With<CameraFollow>, Without<Camera2d>)>,
    time: Res<Time>,
) {
    let Vec3 { x, y, .. } = follow.translation;
    let direction = Vec3::new(x, y, camera.translation.z);

    const CAMERA_DECAY_RATE: f32 = 2.3;
    camera
        .translation
        .smooth_nudge(&direction, CAMERA_DECAY_RATE, time.delta_secs());
}
