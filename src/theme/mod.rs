//! Reusable UI widgets & theming.

// Unused utilities may trigger this lints undesirably.
#![allow(dead_code)]

use bevy::prelude::*;
use bevy_egui::egui;
use once_cell::sync::Lazy;

pub(super) fn plugin(_app: &mut App) {}

pub static TITLE_STYLE: Lazy<egui::TextStyle> = Lazy::new(|| egui::TextStyle::Name("Title".into()));

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
