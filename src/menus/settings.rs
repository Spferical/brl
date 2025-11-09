//! The settings menu.
//!
//! Additional settings and accessibility options should go here.

use bevy::{audio::Volume, prelude::*};
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

use crate::{menus::Menu, screens::Screen};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        EguiPrimaryContextPass,
        settings_menu.run_if(in_state(Menu::Settings)),
    );
}

fn settings_menu(
    mut contexts: EguiContexts,
    mut global_volume: ResMut<GlobalVolume>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    screen: Res<State<Screen>>,
    mut next_menu: ResMut<NextState<Menu>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    ctx.style_mut(crate::theme::use_menu_theme);
    egui::Window::new("Settings")
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::new(0.0, 0.0))
        .show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.add(
                    egui::Slider::from_get_set(0.0..=3.0, |vol| {
                        if let Some(vol) = vol {
                            global_volume.volume = Volume::Linear(vol as f32);
                            vol
                        } else {
                            global_volume.volume.to_linear() as f64
                        }
                    })
                    .text("Volume"),
                );
                if ui.button("Back").clicked() || keyboard_input.just_pressed(KeyCode::Escape) {
                    next_menu.set(if screen.get() == &Screen::Title {
                        Menu::Main
                    } else {
                        Menu::Pause
                    });
                }
            })
        });
}
