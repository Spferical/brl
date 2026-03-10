use crate::game::Player;
use crate::game::apply_brainrot_ui;
use bevy::prelude::*;
use bevy_egui::{
    EguiContexts,
    egui::{self, Color32, RichText},
};

#[derive(Resource, Default)]
pub struct HelpState {
    pub is_open: bool,
    pub current_screen: usize,
    pub skip_tutorial: bool,
}

pub fn is_help_closed(help_state: Res<HelpState>) -> bool {
    !help_state.is_open
}

pub fn draw_help(
    mut contexts: EguiContexts,
    mut help_state: ResMut<HelpState>,
    player: Single<&Player>,
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

    // Dim background
    let screen_rect = ctx.input(|i| i.content_rect());
    egui::Area::new(egui::Id::new("help_dim_area"))
        .order(egui::Order::Foreground)
        .interactable(true)
        .fixed_pos(screen_rect.min)
        .show(ctx, |ui| {
            ui.allocate_exact_size(screen_rect.size(), egui::Sense::click());
            ui.painter().rect_filled(
                screen_rect,
                0.0,
                Color32::from_rgba_premultiplied(0, 0, 0, 180),
            );
        });

    egui::Area::new(egui::Id::new("help_window_area"))
        .order(egui::Order::Tooltip)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            egui::Frame::window(ui.style())
                .fill(Color32::from_rgb(30, 30, 30))
                .inner_margin(egui::Margin::same(20))
                .show(ui, |ui| {
                    ui.set_width(450.0);
                    ui.set_height(400.0);

                    // Top row with Skip button
                    ui.horizontal(|ui| {
                        ui.add_space(ui.available_width() - 40.0);
                        if ui.button(RichText::new("Skip").size(14.0).color(Color32::LIGHT_GRAY)).clicked() {
                            help_state.is_open = false;
                            commands.spawn(crate::audio::sound_effect(assets.button_click.clone()));
                        }
                    });

                    ui.vertical_centered(|ui| {
                        ui.add_space(10.0);

                        match help_state.current_screen {
                            0 => {
                                ui.label(apply_brainrot_ui(
                                    RichText::new("Welcome to BrainrotRL!").size(24.0).strong().color(Color32::WHITE),
                                    brainrot,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                ));
                                ui.add_space(15.0);
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
                            },
                            1 => {
                                ui.add_space(40.0);
                                ui.label(apply_brainrot_ui(
                                    RichText::new("The Phone").size(24.0).strong().color(Color32::WHITE),
                                    brainrot,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                ));
                                ui.add_space(30.0);
                                ui.label(apply_brainrot_ui(
                                    RichText::new("Press Space to toggle your phone.\nUse the mouse to interact with apps.").size(18.0).color(Color32::LIGHT_GRAY),
                                    brainrot,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                ));
                            },
                            2 => {
                                ui.add_space(40.0);
                                ui.label(apply_brainrot_ui(
                                    RichText::new("Stats & Survival").size(24.0).strong().color(Color32::WHITE),
                                    brainrot,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                ));
                                ui.add_space(30.0);
                                ui.label(apply_brainrot_ui(
                                    RichText::new("Watch your stats on the left sidebar.\nKeep your brainrot in check!").size(18.0).color(Color32::LIGHT_GRAY),
                                    brainrot,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                ));
                            },
                            _ => {},
                        }

                        ui.add_space(40.0);

                        ui.horizontal(|ui| {
                            let total_width = ui.available_width();
                            let button_width = 100.0;

                            ui.add_space((total_width - (button_width * 2.0 + 20.0)) / 2.0);

                            if help_state.current_screen > 0 {
                                if ui.add_sized([button_width, 40.0], egui::Button::new(apply_brainrot_ui(
                                    "< Back",
                                    brainrot,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                ))).clicked() {
                                    help_state.current_screen -= 1;
                                    commands.spawn(crate::audio::sound_effect(assets.button_click.clone()));
                                }
                            } else {
                                ui.add_enabled_ui(false, |ui| {
                                    ui.add_sized([button_width, 40.0], egui::Button::new(apply_brainrot_ui(
                                        "< Back",
                                        brainrot,
                                        ui.style(),
                                        egui::FontSelection::Default,
                                        egui::Align::Center,
                                    )));
                                });
                            }

                            ui.add_space(20.0);

                            if help_state.current_screen < 2 {
                                if ui.add_sized([button_width, 40.0], egui::Button::new(apply_brainrot_ui(
                                    "Next >",
                                    brainrot,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                ))).clicked() {
                                    help_state.current_screen += 1;
                                    commands.spawn(crate::audio::sound_effect(assets.button_click.clone()));
                                }
                            } else {
                                if ui.add_sized([button_width, 40.0], egui::Button::new(apply_brainrot_ui(
                                    "Close",
                                    brainrot,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                ))).clicked() {
                                    help_state.is_open = false;
                                    commands.spawn(crate::audio::sound_effect(assets.button_click.clone()));
                                }
                            }
                        });

                        ui.add_space(10.0);
                        ui.checkbox(&mut help_state.skip_tutorial, "Don't show again at start");
                    });
                });
        });
}
