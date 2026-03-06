use bevy::prelude::*;
use bevy_egui::{
    EguiContexts, EguiTextureHandle,
    egui::{self, Color32, RichText},
};
use rand::Rng;

use crate::game::{Creature, Player, apply_brainrot_ui};
use crate::game::{assets::WorldAssets, upgrades::UpgradeMessage};

use crate::game::mobile_apps::{self, AppId, DungeonDashScreen, DungeonDashSelection};

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PhoneScreen {
    #[default]
    Home,
    App(AppId),
}

use std::collections::HashMap;

#[derive(Resource, Default)]
pub struct PhoneState {
    pub is_open: bool,
    pub slide_progress: f32,
    pub click_progress: HashMap<AppId, f32>,
    pub app_open_progress: f32,
    pub last_opened_app: Option<AppId>,
    pub app_launch_progress: f32,
    pub bump_timer: f32,
    pub bump_progress: f32,
    pub creep_timer: f32,
    pub creep_progress: f32,
    pub is_creeping: bool,
    pub is_hovered: bool,
    pub unread_notification: Option<AppId>,
}

impl PhoneState {
    fn set_notification(&mut self, notif: Option<AppId>) {
        if self.unread_notification.is_none()
            && notif.is_some()
            && !self.is_open
            && self.slide_progress == 0.0
        {
            self.bump_progress = 0.01;
        }
        self.unread_notification = notif;
    }
}

pub fn set_notification(mut phone_state: ResMut<PhoneState>, player: Single<&Player>) {
    if player.pending_upgrades > 0 {
        phone_state.set_notification(Some(AppId::Upgrade));
    } else {
        phone_state.set_notification(None);
    }
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
    player: Single<&Player>,
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

    // Bump logic
    if !phone_state.is_open && phone_state.slide_progress == 0.0 {
        phone_state.bump_timer -= time.delta_secs();

        if phone_state.unread_notification.is_some()
            && phone_state.bump_progress == 0.0
            && phone_state.bump_timer <= 0.0
        {
            phone_state.bump_progress = 0.01;
            phone_state.bump_timer = 1.0;
        }
    }

    if phone_state.bump_progress > 0.0 {
        phone_state.bump_progress += time.delta_secs() * 2.0;
        if phone_state.bump_progress >= 1.0 {
            phone_state.bump_progress = 0.0;
        }
        needs_repaint = true;
    }

    // Creep logic
    if phone_state.is_open {
        phone_state.creep_timer = 5.0; // Wait 5 seconds after closing
        phone_state.is_creeping = false;
        if phone_state.creep_progress > 0.0 {
            phone_state.creep_progress = 0.0;
            needs_repaint = true;
        }
    } else if phone_state.slide_progress == 0.0 {
        if phone_state.is_creeping {
            if !phone_state.is_hovered {
                phone_state.creep_timer -= time.delta_secs();
            }
            let p = ((player.boredom as f32 - 80.0) / 20.0).clamp(0.0, 1.0);
            let target_creep = 0.5 + (0.5 * p); // From 0.5 to 1.0 based on boredom

            if phone_state.creep_progress < target_creep {
                phone_state.creep_progress =
                    (phone_state.creep_progress + time.delta_secs() * 0.5).min(target_creep);
                needs_repaint = true;
            } else if phone_state.creep_progress > target_creep {
                phone_state.creep_progress =
                    (phone_state.creep_progress - time.delta_secs() * 0.5).max(target_creep);
                needs_repaint = true;
            }

            if phone_state.creep_timer <= 0.0 {
                phone_state.is_creeping = false;
                let factor = 1.0 - (p * 0.8); // 1.0 at 80 boredom, 0.2 at 100 boredom
                phone_state.creep_timer = rand::rng().random_range((2.0 * factor)..(6.0 * factor));
            }
        } else if phone_state.creep_timer > 0.0 {
            phone_state.creep_timer -= time.delta_secs();
            if phone_state.creep_progress > 0.0 && !phone_state.is_hovered {
                phone_state.creep_progress =
                    (phone_state.creep_progress - time.delta_secs() * 2.0).max(0.0);
                needs_repaint = true;
            }
        } else if player.boredom > 80 {
            phone_state.is_creeping = true;
            phone_state.creep_timer = rand::rng().random_range(1.5..3.0); // Stay up for 1.5 to 3.0 seconds
        } else if phone_state.creep_progress > 0.0 && !phone_state.is_hovered {
            phone_state.creep_progress =
                (phone_state.creep_progress - time.delta_secs() * 2.0).max(0.0);
            needs_repaint = true;
        }
    }

    let click_speed = 3.0 * time.delta_secs();
    for click_p in phone_state.click_progress.values_mut() {
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

    if phone_state.app_open_progress == 1.0 && matches!(*current_screen.get(), PhoneScreen::App(_))
    {
        let old_launch_progress = phone_state.app_launch_progress;
        // 500ms delay + 500ms fade = 1s
        phone_state.app_launch_progress =
            (phone_state.app_launch_progress + time.delta_secs()).min(1.0);
        if old_launch_progress != phone_state.app_launch_progress {
            needs_repaint = true;
        }
    } else {
        phone_state.app_launch_progress = 0.0;
    }

    if (needs_repaint || (phone_state.is_open && *current_screen.get() != PhoneScreen::Home))
        && let Ok(ctx) = contexts.ctx_mut()
    {
        ctx.request_repaint();
    }
}

pub fn draw_phone(
    mut contexts: EguiContexts,
    player_query: Single<(&mut Player, &mut Creature, &crate::game::map::MapPos)>,
    mut phone_state: ResMut<PhoneState>,
    mut streaming_state: ResMut<crate::game::chat::StreamingState>,
    mut active_delivery: ResMut<crate::game::delivery::ActiveDelivery>,
    walk_blocked_map: Res<crate::game::map::WalkBlockedMap>,
    assets: Res<WorldAssets>,
    current_screen: Res<State<PhoneScreen>>,
    mut next_screen: ResMut<NextState<PhoneScreen>>,
    dd_screen: Res<State<DungeonDashScreen>>,
    mut next_dd_screen: ResMut<NextState<DungeonDashScreen>>,
    mut dd_selection: ResMut<DungeonDashSelection>,
    mut msg_upgrade: MessageWriter<UpgradeMessage>,
    mut cockatrice_state: ResMut<mobile_apps::CockatriceState>,
) {
    let (mut player, mut creature, player_pos) = player_query.into_inner();
    let texture_id = contexts.add_image(EguiTextureHandle::Weak(assets.phone.id()));
    let apps = mobile_apps::get_apps();
    let app_icons: Vec<Option<egui::TextureId>> = apps
        .iter()
        .map(|app| {
            app.1
                .icon(&assets)
                .map(|handle| contexts.add_image(EguiTextureHandle::Weak(handle.id())))
        })
        .collect();

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    if phone_state.slide_progress <= 0.0
        && phone_state.bump_progress <= 0.0
        && phone_state.creep_progress <= 0.0
    {
        return;
    }

    let eased_progress = EasingCurve::new(0.0, 1.0, EaseFunction::CubicInOut)
        .sample_clamped(phone_state.slide_progress);

    let screen_rect = ctx.content_rect();

    if eased_progress > 0.0 {
        let dim_alpha = (220.0 * eased_progress) as u8;

        egui::Area::new(egui::Id::new("phone_dim_area"))
            .order(egui::Order::Foreground)
            .interactable(true)
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(screen_rect.size(), egui::Sense::click());
                ui.painter().rect_filled(
                    rect,
                    0.0,
                    Color32::from_rgba_premultiplied(0, 0, 0, dim_alpha),
                );
                if response.clicked() {
                    phone_state.is_open = false;
                }
            });
    }

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

    let mut current_y = egui::lerp(offscreen_y..=onscreen_y, eased_progress);

    if phone_state.bump_progress > 0.0 {
        let bump_offset = (phone_state.bump_progress * std::f32::consts::PI).sin() * 100.0;
        current_y -= bump_offset;
    }

    if phone_state.creep_progress > 0.0 {
        let creep_offset = phone_state.creep_progress * 150.0;
        current_y -= creep_offset;
    }

    let phone_rect = egui::Rect::from_center_size(egui::pos2(center_x, current_y), phone_size);

    egui::Area::new(egui::Id::new("phone_modal_area"))
        .order(egui::Order::Tooltip)
        .constrain(false)
        .fixed_pos(phone_rect.min)
        .show(ctx, |ui| {
            let sense = if phone_state.is_open {
                egui::Sense::hover()
            } else {
                egui::Sense::click()
            };
            let (rect, response) = ui.allocate_exact_size(phone_size, sense);

            phone_state.is_hovered = response.hovered();

            if response.clicked() && !phone_state.is_open {
                phone_state.is_open = true;
            }

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

            // Draw Home Screen (Apps)
            if phone_state.app_open_progress < 1.0 {
                let home_alpha = (255.0 * (1.0 - phone_state.app_open_progress)) as u8;

                let mut visible_idx = 0;
                for (i, app) in apps.iter().enumerate() {
                    if !app.1.show_on_home_screen() {
                        continue;
                    }
                    let app_id = app.0;
                    let row = visible_idx / 3;
                    let col = visible_idx % 3;
                    visible_idx += 1;

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
                        phone_state.click_progress.insert(app_id, 0.01);
                        next_screen.set(PhoneScreen::App(app_id));
                    }

                    let click_p = phone_state
                        .click_progress
                        .get(&app_id)
                        .copied()
                        .unwrap_or(0.0);
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

                    let icon_id = app_icons[i];
                    if let Some(icon_id) = icon_id {
                        let mut icon_mesh = egui::Mesh::with_texture(icon_id);
                        icon_mesh.add_rect_with_uv(
                            icon_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::from_rgba_unmultiplied(255, 255, 255, home_alpha),
                        );
                        ui.painter().add(icon_mesh);
                    }

                    let wrapped_name = textwrap::fill(app.1.name(), 15);
                    let text_job = apply_brainrot_ui(
                        RichText::new(wrapped_name).size(25.0 * scale_x),
                        player.brainrot,
                        ui.style(),
                        egui::FontSelection::Default,
                        egui::Align::Center,
                    )
                    .into_layout_job(
                        ui.style(),
                        egui::FontSelection::Default,
                        egui::Align::Center,
                    );

                    let mut text_job = (*text_job).clone();
                    for section in &mut text_job.sections {
                        section.format.color =
                            Color32::from_rgba_unmultiplied(50, 50, 50, home_alpha);
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

                if let Some(unread_idx) = phone_state.unread_notification {
                    let banner_width = screen_width * 0.9;
                    let banner_height = 80.0 * scale_y;
                    let banner_rect = egui::Rect::from_center_size(
                        egui::pos2(
                            phone_screen_rect.center().x,
                            phone_screen_rect.bottom() - 250.0 * scale_y,
                        ),
                        egui::vec2(banner_width, banner_height),
                    );

                    let response = ui.interact(
                        banner_rect,
                        ui.id().with("notification_banner"),
                        egui::Sense::click(),
                    );
                    let fill = if response.hovered() {
                        Color32::from_rgba_unmultiplied(220, 220, 220, home_alpha)
                    } else {
                        Color32::from_rgba_unmultiplied(255, 255, 255, home_alpha)
                    };

                    ui.painter().rect_filled(banner_rect, 10.0 * scale_x, fill);

                    let text_job = apply_brainrot_ui(
                        RichText::new("New Upgrade Available!").size(32.0 * scale_x),
                        player.brainrot,
                        ui.style(),
                        egui::FontSelection::Default,
                        egui::Align::Center,
                    )
                    .into_layout_job(
                        ui.style(),
                        egui::FontSelection::Default,
                        egui::Align::Center,
                    );

                    let mut text_job = (*text_job).clone();
                    for section in &mut text_job.sections {
                        section.format.color = Color32::from_rgba_unmultiplied(0, 0, 0, home_alpha);
                    }
                    let galley = ui.painter().layout_job(text_job);
                    ui.painter().galley(
                        egui::pos2(
                            banner_rect.center().x - galley.size().x / 2.0,
                            banner_rect.center().y - galley.size().y / 2.0,
                        ),
                        galley,
                        Color32::from_rgba_unmultiplied(0, 0, 0, home_alpha),
                    );

                    if response.clicked() && *current_screen.get() == PhoneScreen::Home {
                        phone_state.unread_notification = None;
                        next_screen.set(PhoneScreen::App(unread_idx));
                    }
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

                if let Some(app_id) = phone_state.last_opened_app {
                    let i = apps.iter().position(|a| a.0 == app_id).unwrap();
                    let app = &apps[i];

                    if phone_state.app_launch_progress > 0.5 {
                        let content_alpha = (phone_state.app_launch_progress - 0.5) / 0.5;
                        let alpha_byte = (255.0 * content_alpha) as u8;

                        let mut child_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(phone_screen_rect)
                                .layout(egui::Layout::top_down(egui::Align::Center)),
                        );
                        child_ui.set_clip_rect(phone_screen_rect);

                        if player.signal <= 1 {
                            child_ui.add_space(child_ui.available_height() * 0.4);
                            child_ui.label(apply_brainrot_ui(
                                RichText::new("No Network Connection")
                                    .size(32.0 * scale_x)
                                    .color(Color32::from_rgba_unmultiplied(0, 0, 0, alpha_byte)),
                                player.brainrot,
                                child_ui.style(),
                                egui::FontSelection::Default,
                                egui::Align::Center,
                            ));
                        } else {
                            app.1.draw_content(
                                &mut child_ui,
                                &mut phone_state,
                                &mut streaming_state,
                                &mut player,
                                &mut creature,
                                player_pos,
                                &mut active_delivery,
                                &walk_blocked_map,
                                scale_x,
                                alpha_byte,
                                dd_screen.get(),
                                &mut next_dd_screen,
                                &mut dd_selection,
                                &mut msg_upgrade,
                                &mut next_screen,
                                &mut cockatrice_state,
                            );
                        }
                    }

                    // Draw splash screen (icon and name) if not fully faded in.
                    let should_draw_splash_icons = phone_state.app_launch_progress < 0.75;

                    if should_draw_splash_icons {
                        let splash_alpha = if phone_state.app_launch_progress > 0.5 {
                            // Fade out splash icons
                            (255.0 * (1.0 - (phone_state.app_launch_progress - 0.5) / 0.25)) as u8
                        } else {
                            app_alpha
                        };

                        let icon_id = app_icons[i];
                        let splash_name = if player.signal <= 1 {
                            "No Network Connection"
                        } else {
                            app.1.splash_name()
                        };

                        let large_icon_size = icon_size * 2.0;
                        let large_icon_rect = egui::Rect::from_center_size(
                            phone_screen_rect.center(),
                            egui::vec2(large_icon_size, large_icon_size),
                        );

                        if let Some(icon_id) = icon_id {
                            let mut app_mesh = egui::Mesh::with_texture(icon_id);
                            app_mesh.add_rect_with_uv(
                                large_icon_rect,
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                Color32::from_rgba_unmultiplied(255, 255, 255, splash_alpha),
                            );
                            ui.painter().add(app_mesh);
                        }

                        let text_job = apply_brainrot_ui(
                            splash_name,
                            player.brainrot,
                            ui.style(),
                            egui::FontSelection::FontId(egui::FontId::proportional(32.0 * scale_y)),
                            egui::Align::Center,
                        )
                        .into_layout_job(
                            ui.style(),
                            egui::FontSelection::FontId(egui::FontId::proportional(32.0 * scale_y)),
                            egui::Align::Center,
                        );

                        let mut text_job = (*text_job).clone();
                        for section in &mut text_job.sections {
                            section.format.color =
                                Color32::from_rgba_unmultiplied(0, 0, 0, splash_alpha);
                        }

                        let galley = ui.painter().layout_job(text_job);
                        let text_pos = egui::pos2(
                            large_icon_rect.center().x,
                            large_icon_rect.bottom() + 16.0 * scale_y,
                        );

                        ui.painter().galley(
                            egui::pos2(text_pos.x - galley.size().x / 2.0, text_pos.y),
                            galley,
                            Color32::from_rgba_unmultiplied(0, 0, 0, splash_alpha),
                        );
                    }
                }
            }

            // Home Button Interaction
            let home_x = rect.min.x + 457.0 * scale_x;
            let home_y = rect.min.y + 1375.0 * scale_y;
            let home_radius = 46.0 * scale_x;
            let home_rect = egui::Rect::from_center_size(
                egui::pos2(home_x, home_y),
                egui::vec2(home_radius * 2.0, home_radius * 2.0),
            );

            // We only need to interact if we're not on the Home screen or transitioning.
            // But having it active always doesn't hurt.
            let response =
                ui.interact(home_rect, ui.id().with("home_button"), egui::Sense::click());

            let is_not_home = *current_screen.get() != PhoneScreen::Home;

            if is_not_home {
                let time = ui.ctx().input(|i| i.time);
                let pulse = (time * 3.0).sin() as f32 * 2.0 + 0.5;
                let alpha = (120.0 + 120.0 * pulse).clamp(0.0, 255.0) as u8;
                ui.painter().circle_filled(
                    home_rect.center(),
                    home_radius * (1.1 + 0.1 * pulse),
                    Color32::from_rgba_unmultiplied(255, 215, 0, alpha / 4),
                );
            }

            let hover_t = ui.ctx().animate_bool(
                ui.id().with("home_hover"),
                response.hovered() && is_not_home,
            );
            if hover_t > 0.0 {
                let scale = 1.0 + hover_t * 0.15;
                ui.painter().circle_stroke(
                    home_rect.center(),
                    home_radius * scale,
                    egui::Stroke::new(
                        3.0 * scale,
                        Color32::from_rgba_unmultiplied(255, 255, 255, (200.0 * hover_t) as u8),
                    ),
                );
            }

            if response.clicked() && is_not_home {
                next_screen.set(PhoneScreen::Home);
            }
        });
}
