use bevy::prelude::*;
use bevy_egui::{
    EguiContexts, EguiTextureHandle,
    egui::{self, Color32, RichText},
};
use rand::Rng;

use crate::game::{Creature, Player, apply_brainrot_ui};
use crate::game::{assets::WorldAssets, upgrades::UpgradeMessage};

use crate::game::delivery::{DungeonDashScreen, DungeonDashState as DungeonDashSelection};
use crate::game::mobile_apps::{self, AppId};

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
    pub forced_open: bool,
    pub vibrate_timer: f32,
    pub dim_flash_timer: f32,
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

pub fn set_notification(
    mut phone_state: ResMut<PhoneState>,
    player: Single<&Player>,
    crawlr_state: Res<mobile_apps::CrawlrState>,
) {
    if player.pending_upgrades > 0 {
        phone_state.set_notification(Some(AppId::Upgrade));
    } else if crawlr_state.has_new_match {
        phone_state.set_notification(Some(AppId::Crawlr));
    } else {
        phone_state.set_notification(None);
    }
}

pub fn is_phone_closed(phone_state: Res<PhoneState>) -> bool {
    !phone_state.is_open && phone_state.slide_progress == 0.0
}

pub fn toggle_phone(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    mut phone_state: ResMut<PhoneState>,
) {
    if keyboard_input.just_pressed(KeyCode::Space)
        || mouse_button.just_pressed(MouseButton::Right)
        || (touches.any_just_pressed() && touches.iter().count() == 2)
    {
        if phone_state.is_open && phone_state.forced_open {
            return;
        }
        phone_state.is_open = !phone_state.is_open;
    }
}

pub fn update_phone(
    time: Res<Time>,
    mut phone_state: ResMut<PhoneState>,
    mut contexts: EguiContexts,
    current_screen: Res<State<PhoneScreen>>,
    mut next_screen: ResMut<NextState<PhoneScreen>>,
    player: Single<&Player>,
) {
    if let PhoneScreen::App(i) = *current_screen.get() {
        phone_state.last_opened_app = Some(i);
    }

    if player.boredom > 95 {
        phone_state.is_open = true;
        phone_state.forced_open = true;
        if !matches!(*current_screen.get(), PhoneScreen::App(AppId::Cockatrice)) {
            next_screen.set(PhoneScreen::App(AppId::Cockatrice));
        }
    } else if phone_state.forced_open && player.boredom < 65 {
        phone_state.forced_open = false;
        phone_state.is_open = false;
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

    if phone_state.vibrate_timer > 0.0 {
        phone_state.vibrate_timer -= time.delta_secs();
        needs_repaint = true;
    }

    if phone_state.dim_flash_timer > 0.0 {
        phone_state.dim_flash_timer -= time.delta_secs();
        needs_repaint = true;
    }

    if (needs_repaint || (phone_state.is_open && *current_screen.get() != PhoneScreen::Home))
        && let Ok(ctx) = contexts.ctx_mut()
    {
        ctx.request_repaint();
    }
}

use bevy::ecs::system::SystemParam;

#[derive(SystemParam)]
pub struct DrawPhoneParams<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub contexts: EguiContexts<'w, 's>,
    pub player_query: Single<
        'w,
        's,
        (
            &'static mut Player,
            &'static mut Creature,
            &'static crate::game::map::MapPos,
        ),
    >,
    pub phone_state: ResMut<'w, PhoneState>,
    pub streaming_state: ResMut<'w, crate::game::chat::StreamingState>,
    pub active_delivery: ResMut<'w, crate::game::delivery::ActiveDelivery>,
    pub walk_blocked_map: Res<'w, crate::game::map::WalkBlockedMap>,
    pub assets: Res<'w, WorldAssets>,
    pub current_screen: Res<'w, State<PhoneScreen>>,
    pub next_screen: ResMut<'w, NextState<PhoneScreen>>,
    pub dd_screen: Res<'w, State<DungeonDashScreen>>,
    pub next_dd_screen: ResMut<'w, NextState<DungeonDashScreen>>,
    pub dd_selection: ResMut<'w, DungeonDashSelection>,
    pub msg_upgrade: MessageWriter<'w, UpgradeMessage>,
    pub cockatrice_state: ResMut<'w, mobile_apps::CockatriceState>,
    pub crawlr_state: ResMut<'w, mobile_apps::CrawlrState>,
    pub map_info: Res<'w, crate::game::mapgen::MapInfo>,
}

pub fn draw_phone(mut params: DrawPhoneParams) {
    let (mut player, mut creature, player_pos) = params.player_query.into_inner();
    let texture_id = params
        .contexts
        .add_image(EguiTextureHandle::Weak(params.assets.phone.id()));
    let apps = mobile_apps::get_apps();
    let app_icons: Vec<Option<egui::TextureId>> = apps
        .iter()
        .map(|app| {
            app.1.icon(&params.assets).map(|handle| {
                params
                    .contexts
                    .add_image(EguiTextureHandle::Weak(handle.id()))
            })
        })
        .collect();

    let Ok(ctx) = params.contexts.ctx_mut() else {
        return;
    };

    if params.phone_state.slide_progress <= 0.0
        && params.phone_state.bump_progress <= 0.0
        && params.phone_state.creep_progress <= 0.0
    {
        return;
    }

    let eased_progress = EasingCurve::new(0.0, 1.0, EaseFunction::CubicInOut)
        .sample_clamped(params.phone_state.slide_progress);

    let screen_rect = ctx.content_rect();

    if eased_progress > 0.0 {
        let mut dim_alpha = (220.0 * eased_progress) as u8;
        if params.phone_state.dim_flash_timer > 0.0 {
            dim_alpha = (dim_alpha as f32
                * (1.0 - (params.phone_state.dim_flash_timer * 5.0).min(1.0)))
                as u8;
        }

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
                if response.clicked() && !params.phone_state.forced_open {
                    params.phone_state.is_open = false;
                    params.commands.spawn(crate::audio::sound_effect(
                        params.assets.button_click.clone(),
                    ));
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

    if params.phone_state.bump_progress > 0.0 {
        let bump_offset = (params.phone_state.bump_progress * std::f32::consts::PI).sin() * 100.0;
        current_y -= bump_offset;
    }

    if params.phone_state.creep_progress > 0.0 {
        let creep_offset = params.phone_state.creep_progress * 150.0;
        current_y -= creep_offset;
    }

    if params.phone_state.vibrate_timer > 0.0 {
        let shake = (params.phone_state.vibrate_timer * 100.0).sin() * 5.0;
        current_y += shake;
    }

    let phone_rect = egui::Rect::from_center_size(egui::pos2(center_x, current_y), phone_size);

    egui::Area::new(egui::Id::new("phone_modal_area"))
        .order(egui::Order::Tooltip)
        .constrain(false)
        .fixed_pos(phone_rect.min)
        .show(ctx, |ui| {
            let sense = if params.phone_state.is_open {
                egui::Sense::hover()
            } else {
                egui::Sense::click()
            };
            let (rect, response) = ui.allocate_exact_size(phone_size, sense);

            params.phone_state.is_hovered = response.hovered();

            if response.clicked() && !params.phone_state.is_open {
                params.phone_state.is_open = true;
                params.commands.spawn(crate::audio::sound_effect(
                    params.assets.button_click.clone(),
                ));
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
            let columns = 3;
            let spacing = (screen_width - columns as f32 * icon_size) / (columns as f32 + 1.0);
            let top_offset = 60.0 * scale_y;

            // Draw Home Screen (Apps)
            if params.phone_state.app_open_progress < 1.0 {
                let home_alpha = (255.0 * (1.0 - params.phone_state.app_open_progress)) as u8;

                let mut visible_idx = 0;
                for (i, app) in apps.iter().enumerate() {
                    if !app.1.show_on_home_screen() {
                        continue;
                    }
                    let app_id = app.0;
                    let row = visible_idx / columns;
                    let col = visible_idx % columns;
                    visible_idx += 1;

                    let x = phone_screen_rect.min.x + spacing + (icon_size + spacing) * col as f32;
                    let y = phone_screen_rect.min.y
                        + top_offset
                        + (icon_size + spacing + 40.0 * scale_y) * row as f32;

                    let base_icon_rect = egui::Rect::from_min_size(
                        egui::pos2(x, y),
                        egui::vec2(icon_size, icon_size),
                    );

                    let response = ui.interact(
                        base_icon_rect,
                        ui.id().with("app_icon").with(i),
                        egui::Sense::click(),
                    );
                    mobile_apps::play_button_sounds(
                        ui,
                        &mut params.commands,
                        &params.assets,
                        &response,
                    );

                    let hover_t = ui.ctx().animate_bool_with_time(
                        ui.id().with("hover").with(i),
                        response.hovered(),
                        0.1,
                    );
                    let hover_scale = 1.0 + hover_t * 0.1;

                    if response.clicked() && *params.current_screen.get() == PhoneScreen::Home {
                        params.phone_state.click_progress.insert(app_id, 0.01);
                        params.next_screen.set(PhoneScreen::App(app_id));
                    }

                    let click_p = params
                        .phone_state
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

                    let wrapped_name = textwrap::fill(app.1.name(), 12);
                    let text_job = apply_brainrot_ui(
                        RichText::new(wrapped_name).size(20.0 * scale_x),
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
                        base_icon_rect.bottom() + 4.0 * scale_y,
                    );

                    ui.painter().galley(
                        egui::pos2(text_pos.x - galley.size().x / 2.0, text_pos.y),
                        galley,
                        Color32::from_rgba_unmultiplied(255, 255, 255, home_alpha),
                    );

                    // Notification dot
                    if params.phone_state.unread_notification == Some(app_id) {
                        let dot_radius = 12.0 * scale_x;
                        let dot_center =
                            base_icon_rect.right_top() + egui::vec2(-dot_radius, dot_radius);
                        ui.painter().circle_filled(
                            dot_center,
                            dot_radius,
                            Color32::from_rgba_unmultiplied(255, 0, 0, home_alpha),
                        );
                        ui.painter().circle_stroke(
                            dot_center,
                            dot_radius,
                            egui::Stroke::new(
                                2.0,
                                Color32::from_rgba_unmultiplied(255, 255, 255, home_alpha),
                            ),
                        );
                    }
                }

                if let Some(unread_idx) = params.phone_state.unread_notification {
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
                    mobile_apps::play_button_sounds(
                        ui,
                        &mut params.commands,
                        &params.assets,
                        &response,
                    );
                    let fill = if response.hovered() {
                        Color32::from_rgba_unmultiplied(220, 220, 220, home_alpha)
                    } else {
                        Color32::from_rgba_unmultiplied(255, 255, 255, home_alpha)
                    };

                    ui.painter().rect_filled(banner_rect, 10.0 * scale_x, fill);

                    let banner_text = match unread_idx {
                        AppId::Upgrade => "New Upgrade Available!",
                        AppId::Crawlr => "You got a like!",
                        _ => "New notification",
                    };

                    let text_job = apply_brainrot_ui(
                        RichText::new(banner_text).size(32.0 * scale_x),
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

                    if response.clicked() && *params.current_screen.get() == PhoneScreen::Home {
                        params.phone_state.unread_notification = None;
                        params.next_screen.set(PhoneScreen::App(unread_idx));
                    }
                }
            }

            // Draw App Overlay Transition
            if params.phone_state.app_open_progress > 0.0 {
                let app_alpha = (255.0 * params.phone_state.app_open_progress) as u8;
                ui.painter().rect_filled(
                    phone_screen_rect,
                    0.0,
                    Color32::from_rgba_unmultiplied(230, 230, 230, app_alpha),
                );

                if let Some(app_id) = params.phone_state.last_opened_app {
                    let i = apps.iter().position(|a| a.0 == app_id).unwrap();
                    let app = &apps[i];

                    if params.phone_state.app_launch_progress > 0.5 {
                        let content_alpha = (params.phone_state.app_launch_progress - 0.5) / 0.5;
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
                                &mut params.commands,
                                &params.assets,
                                &mut params.phone_state,
                                &mut params.streaming_state,
                                &mut player,
                                &mut creature,
                                player_pos,
                                &mut params.active_delivery,
                                &params.walk_blocked_map,
                                scale_x,
                                alpha_byte,
                                params.dd_screen.get(),
                                &mut params.next_dd_screen,
                                &mut params.dd_selection,
                                &mut params.msg_upgrade,
                                &mut params.next_screen,
                                &mut params.cockatrice_state,
                                &mut params.crawlr_state,
                                &params.map_info,
                            );
                        }
                    }

                    // Draw splash screen (icon and name) if not fully faded in.
                    let should_draw_splash_icons = params.phone_state.app_launch_progress < 0.75;

                    if should_draw_splash_icons {
                        let splash_alpha = if params.phone_state.app_launch_progress > 0.5 {
                            // Fade out splash icons
                            (255.0 * (1.0 - (params.phone_state.app_launch_progress - 0.5) / 0.25))
                                as u8
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
            let home_x = rect.min.x + 461.0 * scale_x;
            let home_y = rect.min.y + 1365.0 * scale_y;
            let home_radius = 46.0 * scale_x;
            let home_rect = egui::Rect::from_center_size(
                egui::pos2(home_x, home_y),
                egui::vec2(home_radius * 2.0, home_radius * 2.0),
            );

            // We only need to interact if we're not on the Home screen or transitioning.
            // But having it active always doesn't hurt.
            let response =
                ui.interact(home_rect, ui.id().with("home_button"), egui::Sense::click());

            let is_not_home = *params.current_screen.get() != PhoneScreen::Home;
            let can_go_home = is_not_home && !params.phone_state.forced_open;

            if can_go_home {
                mobile_apps::play_button_sounds(
                    ui,
                    &mut params.commands,
                    &params.assets,
                    &response,
                );
            }

            if can_go_home {
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
                response.hovered() && can_go_home,
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

            if response.clicked() && is_not_home && !params.phone_state.forced_open {
                params.next_screen.set(PhoneScreen::Home);
            }
        });
}
