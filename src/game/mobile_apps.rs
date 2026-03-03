use bevy::prelude::*;
use bevy_egui::egui::{self, Color32, RichText};

use crate::game::Player;
use crate::game::assets::WorldAssets;
use crate::game::phone::PhoneState;

pub trait MobileApp: Send + Sync {
    fn name(&self) -> &str;
    fn splash_name(&self) -> &str;
    fn icon(&self, assets: &WorldAssets) -> Handle<Image>;
    fn draw_content(
        &self,
        ui: &mut egui::Ui,
        phone_state: &mut PhoneState,
        player: &Player,
        scale: f32,
        alpha: u8,
    );
}

pub struct Crawlr;

impl MobileApp for Crawlr {
    fn name(&self) -> &str {
        "Crawlr"
    }
    fn splash_name(&self) -> &str {
        "Crawlr"
    }
    fn icon(&self, assets: &WorldAssets) -> Handle<Image> {
        assets.phone_app_icons.crawlr.clone()
    }
    fn draw_content(
        &self,
        _ui: &mut egui::Ui,
        _phone_state: &mut PhoneState,
        _player: &Player,
        _scale: f32,
        _alpha: u8,
    ) {
    }
}

pub struct DungeonDash;

impl MobileApp for DungeonDash {
    fn name(&self) -> &str {
        "Dungeon Dash"
    }
    fn splash_name(&self) -> &str {
        "DungeonDash"
    }
    fn icon(&self, assets: &WorldAssets) -> Handle<Image> {
        assets.phone_app_icons.dungeon_dash.clone()
    }
    fn draw_content(
        &self,
        _ui: &mut egui::Ui,
        _phone_state: &mut PhoneState,
        _player: &Player,
        _scale: f32,
        _alpha: u8,
    ) {
    }
}

pub struct UndergroundTV;

impl MobileApp for UndergroundTV {
    fn name(&self) -> &str {
        "Underground TV"
    }
    fn splash_name(&self) -> &str {
        "UndergroundTV"
    }
    fn icon(&self, assets: &WorldAssets) -> Handle<Image> {
        assets.phone_app_icons.underground_tv.clone()
    }
    fn draw_content(
        &self,
        ui: &mut egui::Ui,
        phone_state: &mut PhoneState,
        _player: &Player,
        scale: f32,
        alpha: u8,
    ) {
        let button_text = if phone_state.is_streaming {
            "Stop Streaming"
        } else {
            "Start Streaming"
        };

        let button_res = ui.add(
            egui::Button::new(
                RichText::new(button_text)
                    .size(64.0 * scale)
                    .color(Color32::BLACK),
            )
            .stroke(egui::Stroke::new(2.0, Color32::BLACK))
            .fill(Color32::from_rgba_unmultiplied(200, 200, 200, alpha)),
        );
        if button_res.clicked() {
            phone_state.is_streaming = !phone_state.is_streaming;
            if phone_state.is_streaming {
                phone_state.viewers = phone_state.subscribers;
                phone_state.viewers_displayed = phone_state.subscribers as f32;
            } else {
                phone_state.viewers = 0;
                phone_state.viewers_displayed = 0.0;
            }
        }
    }
}

pub fn get_apps() -> Vec<Box<dyn MobileApp>> {
    vec![
        Box::new(Crawlr),
        Box::new(DungeonDash),
        Box::new(UndergroundTV),
    ]
}
