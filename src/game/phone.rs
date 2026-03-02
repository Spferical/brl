use bevy::prelude::*;
use bevy_egui::{
    EguiContexts, EguiTextureHandle,
    egui::{self, Color32},
};

use crate::game::assets::WorldAssets;

#[derive(Resource, Default)]
pub struct PhoneState {
    pub is_open: bool,
    pub slide_progress: f32,
}

pub fn is_phone_closed(phone_state: Res<PhoneState>) -> bool {
    !phone_state.is_open && phone_state.slide_progress == 0.0
}

pub fn toggle_phone(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut phone_state: ResMut<PhoneState>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        phone_state.is_open = !phone_state.is_open;
    }
}

pub fn update_phone(
    time: Res<Time>,
    mut phone_state: ResMut<PhoneState>,
    mut contexts: EguiContexts,
) {
    let target = if phone_state.is_open { 1.0 } else { 0.0 };
    let speed = 4.0 * time.delta_secs();

    let old_progress = phone_state.slide_progress;
    if phone_state.slide_progress < target {
        phone_state.slide_progress = (phone_state.slide_progress + speed).min(target);
    } else if phone_state.slide_progress > target {
        phone_state.slide_progress = (phone_state.slide_progress - speed).max(target);
    }

    if old_progress != phone_state.slide_progress
        && let Ok(ctx) = contexts.ctx_mut()
    {
        ctx.request_repaint();
    }
}

pub fn draw_phone(
    mut contexts: EguiContexts,
    mut phone_state: ResMut<PhoneState>,
    assets: Res<WorldAssets>,
) {
    if phone_state.slide_progress <= 0.0 {
        return;
    }

    let texture_id = contexts.add_image(EguiTextureHandle::Weak(assets.phone.id()));
    let ctx = contexts.ctx_mut().unwrap();

    let eased_progress = EasingCurve::new(0.0, 1.0, EaseFunction::CubicInOut)
        .sample_clamped(phone_state.slide_progress);

    let screen_rect = ctx.content_rect();
    let dim_alpha = (220.0 * eased_progress) as u8;

    egui::Area::new(egui::Id::new("phone_dim_area"))
        .order(egui::Order::Foreground)
        .interactable(true)
        .fixed_pos(screen_rect.min)
        .show(ctx, |ui| {
            let (rect, response) = ui.allocate_exact_size(screen_rect.size(), egui::Sense::click());
            ui.painter().rect_filled(
                rect,
                0.0,
                Color32::from_rgba_premultiplied(0, 0, 0, dim_alpha),
            );
            if response.clicked() {
                phone_state.is_open = false;
            }
        });

    let phone_img_width = 900.0;
    let phone_img_height = 1600.0;
    let aspect_ratio = phone_img_width / phone_img_height;

    let max_width = screen_rect.width() / 4.0;
    let height = screen_rect.height();
    let width = height * aspect_ratio;

    let final_width = width.min(max_width);
    let final_height = final_width / aspect_ratio;

    let phone_size = egui::vec2(final_width, final_height);

    let center_x = screen_rect.center().x;
    
    let offscreen_y = screen_rect.max.y + final_height * 0.5;
    let onscreen_y = screen_rect.center().y;

    let current_y = egui::lerp(offscreen_y..=onscreen_y, eased_progress);

    let phone_rect = egui::Rect::from_center_size(egui::pos2(center_x, current_y), phone_size);

    egui::Area::new(egui::Id::new("phone_modal_area"))
        .order(egui::Order::Tooltip)
        .constrain(false)
        .fixed_pos(phone_rect.min)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(phone_size, egui::Sense::hover());
            let mut mesh = egui::Mesh::with_texture(texture_id);
            mesh.add_rect_with_uv(
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            ui.painter().add(mesh);
        });
}
