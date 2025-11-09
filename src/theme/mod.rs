//! Reusable UI widgets & theming.

// Unused utilities may trigger this lints undesirably.
#![allow(dead_code)]

pub mod interaction;
pub mod palette;
pub mod widget;

#[allow(unused_imports)]
pub mod prelude {
    pub use super::{interaction::InteractionPalette, palette as ui_palette, widget};
}

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass};

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(interaction::plugin);
    app.add_systems(EguiPrimaryContextPass, init_egui_theme);
}

fn init_egui_theme(mut contexts: EguiContexts) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    ctx.style_mut(|style| {
        style.visuals.widgets.noninteractive.fg_stroke.color = bevy_egui::egui::Color32::WHITE;
    });
}
