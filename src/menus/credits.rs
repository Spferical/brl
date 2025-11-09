//! The credits menu.

use bevy::prelude::*;
use bevy_egui::egui;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass};

use crate::menus::Menu;
use crate::theme::TITLE_STYLE;

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
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered_justified(|ui| {
            ui.label(egui::RichText::new("Credits").text_style(TITLE_STYLE.clone()));
            ui.label("By Spferical");
            if ui.button("Back").clicked() || keyboard_input.just_pressed(KeyCode::Escape) {
                next_menu.set(Menu::Main);
            }
        });
    });
}
