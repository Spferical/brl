//! The credits menu.

use bevy::prelude::*;
use bevy_egui::egui;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass};

use crate::menus::Menu;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        EguiPrimaryContextPass,
        credits_menu.run_if(in_state(Menu::Credits)),
    );
}

fn credits_menu(
    mut contexts: EguiContexts,
    mut next_menu: ResMut<NextState<Menu>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    ctx.style_mut(crate::theme::use_menu_theme);
    egui::Window::new("Credits")
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::new(0.0, 0.0))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Created by").heading());
                egui::Grid::new("credits_grid")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Spferical");
                        ui.label("Programming & Level Design");
                        ui.label("https://github.com/spferical");
                        ui.end_row();
                        ui.label("AnimatedRNG");
                        ui.label("Programming & Level Design");
                        ui.label("https://github.com/animatedrng");
                        ui.end_row();
                        ui.label("ellenjiang7");
                        ui.label("Art");
                        ui.label("https://instagram.com/ellenjiang7");
                        ui.end_row();
                        ui.label("inexzakt");
                        ui.label("Music");
                        ui.label("https://soundcloud.com/inexzakt");
                        ui.end_row();
                    });
                if ui.button("Back").clicked() || keyboard_input.just_pressed(KeyCode::Escape) {
                    next_menu.set(Menu::Main);
                }
            });
        });
}
