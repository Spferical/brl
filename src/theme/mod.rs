//! Reusable UI widgets & theming.

// Unused utilities may trigger this lints undesirably.
#![allow(dead_code)]

use std::sync::LazyLock;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, setup_egui_fonts);
}

fn setup_egui_fonts(mut contexts: EguiContexts, mut done: Local<bool>) {
    if *done {
        return;
    }
    match contexts.ctx_mut() {
        Ok(ctx) => {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "press_start_2p".to_owned(),
                std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                    "../../assets/PressStart2P/PressStart2P-Regular.ttf"
                ))),
            );
            fonts.font_data.insert(
                "comic_regular".to_owned(),
                std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                    "../../assets/Comic_Relief/ComicRelief-Regular.ttf"
                ))),
            );
            fonts.font_data.insert(
                "comic_bold".to_owned(),
                std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                    "../../assets/Comic_Relief/ComicRelief-Bold.ttf"
                ))),
            );

            let mut comic_fallbacks = vec!["comic_regular".to_owned(), "comic_bold".to_owned()];
            if let Some(default_fallbacks) = fonts.families.get(&egui::FontFamily::Proportional) {
                comic_fallbacks.extend(default_fallbacks.iter().cloned());
            }

            fonts.families.insert(
                egui::FontFamily::Name("comic_relief".into()),
                comic_fallbacks,
            );

            fonts.families.insert(
                egui::FontFamily::Name("press_start".into()),
                vec!["press_start_2p".to_owned()],
            );

            ctx.set_fonts(fonts);
            bevy::log::info!("Successfully set merged egui fonts!");
            *done = true;
        }
        Err(_) => {
            // Egui context not ready yet
        }
    }
}
pub static TITLE_STYLE: LazyLock<egui::TextStyle> =
    LazyLock::new(|| egui::TextStyle::Name("Title".into()));

pub fn use_menu_theme(style: &mut egui::Style) {
    style.visuals.widgets.noninteractive.fg_stroke.color = bevy_egui::egui::Color32::WHITE;
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(30.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        TITLE_STYLE.clone(),
        egui::FontId::new(40.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(20.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(20.0, egui::FontFamily::Proportional),
    );
}
