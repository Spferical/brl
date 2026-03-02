//! The main menu (seen on the title screen).
use bevy::prelude::*;
use bevy_egui::EguiContexts;
use bevy_egui::EguiPrimaryContextPass;
use bevy_egui::egui;

use crate::{asset_tracking::ResourceHandles, menus::Menu, screens::Screen};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        EguiPrimaryContextPass,
        main_menu.run_if(in_state(Menu::Main)),
    );
}

fn main_menu(
    mut contexts: EguiContexts,
    resource_handles: Res<ResourceHandles>,
    mut next_screen: ResMut<NextState<Screen>>,
    mut next_menu: ResMut<NextState<Menu>>,
    #[cfg(target_family = "wasm")] mut app_exit: MessageWriter<AppExit>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    ctx.style_mut(crate::theme::use_menu_theme);
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered_justified(|ui| {
            ui.label(egui::RichText::new("BrainrotRL\n").heading());
            if ui.button("Play").clicked() {
                if resource_handles.is_all_done() {
                    next_screen.set(Screen::Gameplay);
                } else {
                    next_screen.set(Screen::Loading);
                }
            }
            if ui.button("Settings").clicked() {
                next_menu.set(Menu::Settings);
            }
            if ui.button("Credits").clicked() {
                next_menu.set(Menu::Credits);
            }
            #[cfg(target_family = "wasm")]
            if ui.button("Exit").clicked() {
                app_exit.write(AppExit::Success);
            }
        });
    });
}
