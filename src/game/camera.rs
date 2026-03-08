use bevy::prelude::*;

use crate::{PrimaryCamera, game::Player};

#[derive(Component)]
pub(crate) struct CameraFollow;

#[derive(Resource, Default)]
pub struct ScreenShake {
    pub trauma: f32,
}

pub(crate) fn update_camera(
    mut camera: Single<&mut Transform, (With<PrimaryCamera>, Without<CameraFollow>)>,
    follow: Single<&Transform, (With<CameraFollow>, Without<PrimaryCamera>)>,
    player: Single<&Player>,
    mut screen_shake: ResMut<ScreenShake>,
    time: Res<Time>,
) {
    let br = player.brainrot as f32;
    let p_zoom = ((br - 80.0) / 20.0).clamp(0.0, 1.0);
    let p_rot = ((br - 70.0) / 30.0).clamp(0.0, 1.0);
    let zoom_scale = 0.8 - (p_zoom * 0.3);
    let rotation_p = p_rot;

    camera.scale = Vec3::new(zoom_scale, zoom_scale, 1.0);

    let Vec3 { x, y, .. } = follow.translation;
    let target = Vec3::new(x, y, camera.translation.z);
    let t = 1.0 - (-10.0 * time.delta_secs()).exp();
    camera.translation = camera.translation.lerp(target, t);

    // Apply screen shake
    if screen_shake.trauma > 0.0 {
        let shake = screen_shake.trauma * screen_shake.trauma;
        let max_offset = 15.0;
        let time_s = time.elapsed_secs() * 50.0;
        let offset_x = (time_s.sin() * 1.5 + (time_s * 1.3).cos()) * max_offset * shake;
        let offset_y = ((time_s * 1.2).cos() * 1.5 + (time_s * 0.8).sin()) * max_offset * shake;
        camera.translation.x += offset_x;
        camera.translation.y += offset_y;

        screen_shake.trauma -= time.delta_secs() * 1.5; // decay
        if screen_shake.trauma < 0.0 {
            screen_shake.trauma = 0.0;
        }
    }

    if rotation_p > 0.0 {
        let max_angle = 5.0_f32.to_radians();
        camera.rotation = Quat::from_rotation_z(rotation_p * max_angle);
    } else {
        camera.rotation = Quat::IDENTITY;
    }
}
