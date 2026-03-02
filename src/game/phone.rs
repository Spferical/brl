use bevy::prelude::*;
use bevy_egui::{
    EguiContexts, EguiTextureHandle,
    egui::{self, Color32},
};

use crate::game::assets::WorldAssets;
use crate::game::{Player, apply_brainrot_ui};

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PhoneScreen {
    #[default]
    Home,
    App(usize),
}

#[derive(Resource, Default)]
pub struct PhoneState {
    pub is_open: bool,
    pub slide_progress: f32,
    pub click_progress: [f32; 2],
    pub app_open_progress: f32,
    pub last_opened_app: Option<usize>,
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
    current_screen: Res<State<PhoneScreen>>,
) {
    if let PhoneScreen::App(i) = *current_screen.get() {
        phone_state.last_opened_app = Some(i);
    }

    let target = if phone_state.is_open { 1.0 } else { 0.0 };
    let speed = 4.0 * time.delta_secs();

    let mut needs_repaint = false;

    let old_progress = phone_state.slide_progress;
    if phone_state.slide_progress < target {
        phone_state.slide_progress = (phone_state.slide_progress + speed).min(target);
    } else if phone_state.slide_progress > target {
        phone_state.slide_progress = (phone_state.slide_progress - speed).max(target);
    }

    if old_progress != phone_state.slide_progress {
        needs_repaint = true;
    }

    let click_speed = 3.0 * time.delta_secs();
    for click_p in phone_state.click_progress.iter_mut() {
        if *click_p > 0.0 {
            *click_p += click_speed;
            if *click_p >= 1.0 {
                *click_p = 0.0;
            }
            needs_repaint = true;
        }
    }

    let app_target = if *current_screen.get() != PhoneScreen::Home {
        1.0
    } else {
        0.0
    };
    let app_speed = 4.0 * time.delta_secs();

    let old_app_progress = phone_state.app_open_progress;
    if phone_state.app_open_progress < app_target {
        phone_state.app_open_progress = (phone_state.app_open_progress + app_speed).min(app_target);
    } else if phone_state.app_open_progress > app_target {
        phone_state.app_open_progress = (phone_state.app_open_progress - app_speed).max(app_target);
    }

    if old_app_progress != phone_state.app_open_progress {
        needs_repaint = true;
    }

    if needs_repaint && let Ok(ctx) = contexts.ctx_mut() {
        ctx.request_repaint();
    }
}

pub fn draw_phone(
    mut contexts: EguiContexts,
    player: Single<&Player>,
    mut phone_state: ResMut<PhoneState>,
    assets: Res<WorldAssets>,
    current_screen: Res<State<PhoneScreen>>,
    mut next_screen: ResMut<NextState<PhoneScreen>>,
) {
    if phone_state.slide_progress <= 0.0 {
        return;
    }

    let texture_id = contexts.add_image(EguiTextureHandle::Weak(assets.phone.id()));
    let crawlr_id = contexts.add_image(EguiTextureHandle::Weak(assets.phone_app_icons.crawlr.id()));
    let dungeon_dash_id = contexts.add_image(EguiTextureHandle::Weak(
        assets.phone_app_icons.dungeon_dash.id(),
    ));
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

            let scale_x = final_width / phone_img_width;
            let scale_y = final_height / phone_img_height;

            let phone_screen_rect = egui::Rect::from_min_max(
                rect.min + egui::vec2(163.0 * scale_x, 236.0 * scale_y),
                rect.min + egui::vec2(774.0 * scale_x, 1293.0 * scale_y),
            );

            let screen_width = phone_screen_rect.width();
            let icon_size = 150.0 * scale_x;
            let spacing = (screen_width - 3.0 * icon_size) / 4.0;

            let icons = [(crawlr_id, "Crawlr"), (dungeon_dash_id, "Dungeon Dash")];

            // Draw Home Screen (Apps)
            if phone_state.app_open_progress < 1.0 {
                let home_alpha = (255.0 * (1.0 - phone_state.app_open_progress)) as u8;

                for (i, (icon_id, name)) in icons.iter().enumerate() {
                    let row = i / 3;
                    let col = i % 3;

                    let x = phone_screen_rect.min.x + spacing + (icon_size + spacing) * col as f32;
                    let y = phone_screen_rect.min.y + spacing + (icon_size + spacing) * row as f32;

                    let base_icon_rect = egui::Rect::from_min_size(
                        egui::pos2(x, y),
                        egui::vec2(icon_size, icon_size),
                    );

                    let response = ui.interact(
                        base_icon_rect,
                        ui.id().with("app_icon").with(i),
                        egui::Sense::click(),
                    );

                    let hover_t = ui.ctx().animate_bool_with_time(
                        ui.id().with("hover").with(i),
                        response.hovered(),
                        0.1,
                    );
                    let hover_scale = 1.0 + hover_t * 0.1;

                    if response.clicked() && *current_screen.get() == PhoneScreen::Home {
                        phone_state.click_progress[i] = 0.01;
                        next_screen.set(PhoneScreen::App(i));
                    }

                    let click_p = phone_state.click_progress[i];
                    let click_scale = if click_p > 0.0 {
                        1.0 - (click_p * std::f32::consts::PI * 2.0).sin() * 0.15
                    } else {
                        1.0
                    };

                    let total_scale = hover_scale * click_scale;

                    let scaled_size = icon_size * total_scale;
                    let offset = (scaled_size - icon_size) / 2.0;
                    let icon_rect = egui::Rect::from_min_size(
                        egui::pos2(x - offset, y - offset),
                        egui::vec2(scaled_size, scaled_size),
                    );

                    ui.painter().circle_stroke(
                        icon_rect.center(),
                        icon_rect.width() / 2.0,
                        egui::Stroke::new(
                            2.0,
                            Color32::from_rgba_unmultiplied(255, 255, 255, home_alpha),
                        ),
                    );

                    let mut icon_mesh = egui::Mesh::with_texture(*icon_id);
                    icon_mesh.add_rect_with_uv(
                        icon_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::from_rgba_unmultiplied(255, 255, 255, home_alpha),
                    );
                    ui.painter().add(icon_mesh);

                    let text_job = apply_brainrot_ui(*name, player.brainrot, ui.style())
                        .into_layout_job(
                            ui.style(),
                            egui::FontSelection::FontId(egui::FontId::proportional(22.0 * scale_y)),
                            egui::Align::Center,
                        );

                    let mut text_job = (*text_job).clone();
                    for section in &mut text_job.sections {
                        section.format.color =
                            Color32::from_rgba_unmultiplied(255, 255, 255, home_alpha);
                    }

                    let galley = ui.painter().layout_job(text_job);
                    let text_pos = egui::pos2(
                        base_icon_rect.center().x,
                        base_icon_rect.bottom() + 8.0 * scale_y,
                    );

                    ui.painter().galley(
                        egui::pos2(text_pos.x - galley.size().x / 2.0, text_pos.y),
                        galley,
                        Color32::from_rgba_unmultiplied(255, 255, 255, home_alpha),
                    );
                }
            }

            // Draw App Overlay Transition
            if phone_state.app_open_progress > 0.0 {
                let app_alpha = (255.0 * phone_state.app_open_progress) as u8;
                ui.painter().rect_filled(
                    phone_screen_rect,
                    0.0,
                    Color32::from_rgba_unmultiplied(230, 230, 230, app_alpha),
                );

                if let Some(i) = phone_state.last_opened_app {
                    let (icon_id, name) = icons[i];
                    let large_icon_size = icon_size * 2.0;
                    let large_icon_rect = egui::Rect::from_center_size(
                        phone_screen_rect.center(),
                        egui::vec2(large_icon_size, large_icon_size),
                    );
                    let mut app_mesh = egui::Mesh::with_texture(icon_id);
                    app_mesh.add_rect_with_uv(
                        large_icon_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::from_rgba_unmultiplied(255, 255, 255, app_alpha),
                    );
                    ui.painter().add(app_mesh);

                    let text_job = apply_brainrot_ui(name, player.brainrot, ui.style())
                        .into_layout_job(
                            ui.style(),
                            egui::FontSelection::FontId(egui::FontId::proportional(40.0 * scale_y)),
                            egui::Align::Center,
                        );

                    let mut text_job = (*text_job).clone();
                    for section in &mut text_job.sections {
                        section.format.color = Color32::from_rgba_unmultiplied(0, 0, 0, app_alpha);
                    }

                    let galley = ui.painter().layout_job(text_job);
                    let text_pos = egui::pos2(
                        large_icon_rect.center().x,
                        large_icon_rect.bottom() + 16.0 * scale_y,
                    );

                    ui.painter().galley(
                        egui::pos2(text_pos.x - galley.size().x / 2.0, text_pos.y),
                        galley,
                        Color32::from_rgba_unmultiplied(0, 0, 0, app_alpha),
                    );
                }
            }

            // Home Button Interaction
            let home_x = rect.min.x + 467.0 * scale_x;
            let home_y = rect.min.y + 1366.0 * scale_y;
            let home_radius = 70.0 * scale_x;
            let home_rect = egui::Rect::from_center_size(
                egui::pos2(home_x, home_y),
                egui::vec2(home_radius * 2.0, home_radius * 2.0),
            );

            // We only need to interact if we're not on the Home screen or transitioning.
            // But having it active always doesn't hurt.
            let response =
                ui.interact(home_rect, ui.id().with("home_button"), egui::Sense::click());
            if response.clicked() && *current_screen.get() != PhoneScreen::Home {
                next_screen.set(PhoneScreen::Home);
            }
        });
}
