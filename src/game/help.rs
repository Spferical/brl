use crate::game::Player;
use crate::game::TurnCounter;
use crate::game::apply_brainrot_ui;
use crate::game::mobile_apps::AppId;
use crate::game::phone::{PhoneScreen, PhoneState};
use bevy::prelude::*;
use bevy_egui::{
    EguiContexts,
    egui::{self, Color32, RichText},
};

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub enum TutorialStep {
    #[default]
    Welcome,
    PickUpgrade,
    OpenDungeonDash,
    DungeonDashIntro,
    ClosePhone,
    Movement,
    Completed,
}

#[derive(Resource, Default)]
pub struct HelpState {
    pub is_open: bool,
    pub current_step: TutorialStep,
    pub skip_tutorial: bool,
    pub wrong_app_opened: bool,
    pub last_pending_upgrades: Option<usize>,
    pub initial_turns: Option<u64>,
}

pub fn is_help_closed(help_state: Res<HelpState>) -> bool {
    !help_state.is_open || help_state.current_step == TutorialStep::Movement
}

pub fn update_help(
    mut help_state: ResMut<HelpState>,
    phone_state: Res<PhoneState>,
    phone_screen: Res<State<PhoneScreen>>,
    player: Single<&Player>,
    turn_counter: Res<TurnCounter>,
) {
    if !help_state.is_open || help_state.current_step == TutorialStep::Completed {
        return;
    }

    match help_state.current_step {
        TutorialStep::Welcome => {
            if phone_state.is_open {
                help_state.current_step = TutorialStep::PickUpgrade;
                help_state.last_pending_upgrades = Some(player.pending_upgrades);
            }
        }
        TutorialStep::PickUpgrade => {
            if let Some(last) = help_state.last_pending_upgrades {
                if player.pending_upgrades < last {
                    help_state.current_step = TutorialStep::OpenDungeonDash;
                }
            } else {
                help_state.last_pending_upgrades = Some(player.pending_upgrades);
            }
        }
        TutorialStep::OpenDungeonDash => match phone_screen.get() {
            PhoneScreen::App(app_id) => {
                if *app_id == AppId::DungeonDash {
                    help_state.current_step = TutorialStep::DungeonDashIntro;
                    help_state.wrong_app_opened = false;
                } else if *app_id != AppId::Upgrade {
                    help_state.wrong_app_opened = true;
                }
            }
            PhoneScreen::Home => {
                help_state.wrong_app_opened = false;
            }
        },
        TutorialStep::DungeonDashIntro => {
            if *phone_screen.get() == PhoneScreen::Home {
                help_state.current_step = TutorialStep::ClosePhone;
            }
        }
        TutorialStep::ClosePhone => {
            if !phone_state.is_open && phone_state.slide_progress == 0.0 {
                help_state.current_step = TutorialStep::Movement;
                help_state.initial_turns = Some(turn_counter.0);
            }
        }
        TutorialStep::Movement => {
            if let Some(initial) = help_state.initial_turns {
                if turn_counter.0 > initial {
                    help_state.current_step = TutorialStep::Completed;
                    help_state.is_open = false;
                }
            } else {
                help_state.initial_turns = Some(turn_counter.0);
            }
        }
        TutorialStep::Completed => {}
    }
}

pub fn draw_help(
    mut contexts: EguiContexts,
    mut help_state: ResMut<HelpState>,
    player: Single<&Player>,
    phone_state: Res<PhoneState>,
    assets: Res<crate::game::assets::WorldAssets>,
    mut commands: Commands,
) {
    if !help_state.is_open {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let brainrot = player.brainrot;
    let screen_rect = ctx.input(|i| i.content_rect());

    let phone_aspect_ratio = 900.0 / 1600.0;
    let phone_max_width = screen_rect.width() / 4.0;
    let phone_width = (screen_rect.height() * phone_aspect_ratio).min(phone_max_width);
    let phone_right = screen_rect.center().x + phone_width / 2.0;

    let sidebar_left = screen_rect.max.x - 192.0;
    let margin = 60.0;

    let available_width = (sidebar_left - phone_right - margin * 2.0).max(250.0);

    let t = EasingCurve::new(0.0, 1.0, EaseFunction::CubicInOut)
        .sample_clamped(phone_state.slide_progress);

    let window_width = egui::lerp(450.0..=available_width, t).min(450.0);

    let start_pos = egui::pos2(
        screen_rect.center().x - window_width / 2.0,
        screen_rect.min.y + 20.0,
    );

    let target_pos = egui::pos2(phone_right + margin, screen_rect.center().y - 100.0);

    let current_pos = egui::pos2(
        egui::lerp(start_pos.x..=target_pos.x, t),
        egui::lerp(start_pos.y..=target_pos.y, t),
    );

    egui::Area::new(egui::Id::new("help_window_area"))
        .order(egui::Order::Tooltip)
        .fixed_pos(current_pos)
        .show(ctx, |ui| {
            egui::Frame::window(ui.style())
                .fill(Color32::from_rgb(30, 30, 30))
                .inner_margin(egui::Margin::same(20))
                .show(ui, |ui| {
                    ui.set_width(window_width);
                    ui.set_max_width(window_width);

                    // Top row with Skip button
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("Skip Tutorial").size(12.0).color(Color32::LIGHT_GRAY)).clicked() {
                            help_state.is_open = false;
                            help_state.current_step = TutorialStep::Completed;
                            help_state.skip_tutorial = true;
                            commands.spawn(crate::audio::sound_effect(assets.button_click.clone()));
                        }
                        ui.add_space(ui.available_width() - 40.0);
                        if ui.button(RichText::new("Skip").size(14.0).color(Color32::LIGHT_GRAY)).clicked() {
                            help_state.is_open = false;
                            commands.spawn(crate::audio::sound_effect(assets.button_click.clone()));
                        }
                    });

                    ui.vertical_centered(|ui| {
                        ui.add_space(10.0);

                        match help_state.current_step {
                            TutorialStep::Welcome => {
                                ui.label(apply_brainrot_ui(
                                    RichText::new("Welcome to BrainrotRL! Let's go over a quick tutorial of the controls.\n\nFirst off -- press space to open up your phone").size(18.0).color(Color32::WHITE),
                                    brainrot,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                ));
                            }
                            TutorialStep::PickUpgrade => {
                                ui.label(apply_brainrot_ui(
                                    RichText::new("Good. Looks like you have an upgrade you need to select\n\nUpgrades give you access to subscriptions, new abilities, and stat upgrades. Let's pick one now.").size(18.0).color(Color32::WHITE),
                                    brainrot,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                ));
                            }
                            TutorialStep::OpenDungeonDash => {
                                let text = if help_state.wrong_app_opened {
                                    "Ah, that's not DungeonDash. \n\nPress the home button on the bottom of the phone to go to the home screen"
                                } else {
                                    "Great! Now, try and open up the DungeonDash app. It's on the top row"
                                };
                                ui.label(apply_brainrot_ui(
                                    RichText::new(text).size(18.0).color(Color32::WHITE),
                                    brainrot,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                ));
                            }
                            TutorialStep::DungeonDashIntro => {
                                ui.label(apply_brainrot_ui(
                                    RichText::new("Great. You can either order food on DungeonDash or take orders as a delivery driver.\n\nBut you can figure that out on your own. Press the home button on the bottom of the phone to go to the home screen").size(18.0).color(Color32::WHITE),
                                    brainrot,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                ));
                            }
                            TutorialStep::ClosePhone => {
                                ui.label(apply_brainrot_ui(
                                    RichText::new("To close the phone, press space again.").size(18.0).color(Color32::WHITE),
                                    brainrot,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                ));
                            }
                            TutorialStep::Movement => {
                                ui.label(apply_brainrot_ui(
                                    RichText::new("Movement Controls:").size(18.0).color(Color32::LIGHT_GRAY),
                                    brainrot,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                ));
                                ui.add_space(20.0);

                                let key_color = Color32::from_rgb(255, 215, 0); // Gold
                                let bg_color = Color32::from_rgb(50, 50, 50);

                                ui.horizontal(|ui| {
                                    let total_width = ui.available_width();
                                    ui.add_space((total_width - 280.0) / 2.0); // Rough center

                                    egui::Grid::new("movement_grid")
                                        .spacing([15.0, 12.0])
                                        .show(ui, |ui| {
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(RichText::new(" WASD / Arrows ").size(18.0).strong().color(key_color).background_color(bg_color));
                                            });
                                            ui.label(RichText::new("Basic").size(16.0).color(Color32::LIGHT_GRAY));
                                            ui.end_row();

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(RichText::new(" yubnhjkl ").size(18.0).strong().color(key_color).background_color(bg_color));
                                            });
                                            ui.label(RichText::new("Vi-Keys").size(16.0).color(Color32::LIGHT_GRAY));
                                            ui.end_row();

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(RichText::new(" Left Click ").size(18.0).strong().color(key_color).background_color(bg_color));
                                            });
                                            ui.label(RichText::new("Mouse move").size(16.0).color(Color32::LIGHT_GRAY));
                                            ui.end_row();
                                        });
                                });

                                ui.add_space(20.0);
                            }
                            TutorialStep::Completed => {},
                        }

                        ui.add_space(10.0);
                    });
                });
        });
}
