//! The pause menu.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

use crate::{menus::Menu, screens::Screen};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        EguiPrimaryContextPass,
        pause_menu.run_if(in_state(Menu::Pause)),
    );
}

fn pause_menu(
    mut contexts: EguiContexts,
    mut next_menu: ResMut<NextState<Menu>>,
    mut next_screen: ResMut<NextState<Screen>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    ctx.style_mut(crate::theme::use_menu_theme);
    egui::Window::new("Pause Menu")
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::new(0.0, 0.0))
        .show(ctx, |ui| {
            if ui.button("Continue").clicked() || keyboard_input.just_pressed(KeyCode::Escape) {
                next_menu.set(Menu::None);
            }
            if ui.button("Settings").clicked() {
                next_menu.set(Menu::Settings);
            }
            if ui.button("Quit to title").clicked() {
                next_screen.set(Screen::Title);
            }
        });
}
