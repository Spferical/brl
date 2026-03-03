use bevy::prelude::*;
use bevy_egui::egui::{self, Color32, RichText};

use crate::game::Player;
use crate::game::apply_brainrot_ui;
use crate::game::assets::WorldAssets;
use crate::game::chat::StreamingState;
use crate::game::phone::PhoneState;

pub trait MobileApp: Send + Sync {
    fn name(&self) -> &str;
    fn splash_name(&self) -> &str;
    fn icon(&self, assets: &WorldAssets) -> Handle<Image>;
    fn draw_content(
        &self,
        ui: &mut egui::Ui,
        phone_state: &mut PhoneState,
        streaming_state: &mut StreamingState,
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
        _streaming_state: &mut StreamingState,
        _player: &Player,
        _scale: f32,
        _alpha: u8,
    ) {
    }
}

pub struct DungeonDash;

struct FoodItem {
    name: &'static str,
    price: i32,
    effects: &'static str,
}

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
        ui: &mut egui::Ui,
        _phone_state: &mut PhoneState,
        _streaming_state: &mut StreamingState,
        player: &Player,
        scale: f32,
        alpha: u8,
    ) {
        let foods = [
            FoodItem {
                name: "Burrito",
                price: 8,
                effects: "-60 hunger, +1hp, +3 strength",
            },
            FoodItem {
                name: "Protein Shake",
                price: 20,
                effects: "-5 hunger, +15 strength",
            },
            FoodItem {
                name: "Health Salad",
                price: 20,
                effects: "-5 hunger, +6hp",
            },
            FoodItem {
                name: "Chicken Tenders",
                price: 4,
                effects: "-30 hunger",
            },
            FoodItem {
                name: "Pizza",
                price: 5,
                effects: "-60 hunger",
            },
            FoodItem {
                name: "Milkshake",
                price: 5,
                effects: "-100 hunger, -1 hp",
            },
            FoodItem {
                name: "Poke",
                price: 20,
                effects: "-40 hunger, +10 strength",
            },
        ];

        let item_height = 120.0 * scale;
        let spacing = 16.0 * scale;
        let total_height = foods.len() as f32 * (item_height + spacing);
        let available_height = ui.available_height();
        if total_height < available_height {
            ui.add_space((available_height - total_height) / 2.0);
        }

        egui::ScrollArea::vertical()
            .id_salt("dungeon_dash_food_list")
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    let width = ui.available_width() * 0.9;
                    for food in foods {
                        let height = 120.0 * scale;
                        let (rect, response) =
                            ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());

                        let fill = if response.is_pointer_button_down_on() {
                            Color32::from_rgba_unmultiplied(150, 150, 150, alpha)
                        } else if response.hovered() {
                            Color32::from_rgba_unmultiplied(220, 220, 220, alpha)
                        } else {
                            Color32::from_rgba_unmultiplied(200, 200, 200, alpha)
                        };

                        ui.painter().rect_filled(rect, 4.0 * scale, fill);
                        ui.painter().rect_stroke(
                            rect,
                            4.0 * scale,
                            egui::Stroke::new(2.0, Color32::BLACK),
                            egui::StrokeKind::Middle,
                        );

                        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(8.0 * scale);
                                ui.label(apply_brainrot_ui(
                                    RichText::new(food.name)
                                        .size(48.0 * scale)
                                        .color(Color32::BLACK),
                                    player.brainrot,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                ));
                                ui.label(apply_brainrot_ui(
                                    RichText::new(format!("${} {}", food.price, food.effects))
                                        .size(24.0 * scale)
                                        .color(Color32::from_rgba_unmultiplied(80, 80, 80, alpha)),
                                    player.brainrot,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                ));
                            });
                        });
                        ui.add_space(16.0 * scale);
                    }
                });
            });
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
        _phone_state: &mut PhoneState,
        streaming_state: &mut StreamingState,
        player: &Player,
        scale: f32,
        alpha: u8,
    ) {
        ui.add_space(ui.available_height() * 0.4);
        let is_low_signal = player.signal <= 2;
        let button_text = if streaming_state.is_streaming {
            "Stop Streaming"
        } else if is_low_signal {
            "Low Signal"
        } else {
            "Start Streaming"
        };

        let button_res = ui.add_enabled(
            !is_low_signal || streaming_state.is_streaming,
            egui::Button::new(
                RichText::new(button_text)
                    .size(64.0 * scale)
                    .color(Color32::BLACK),
            )
            .stroke(egui::Stroke::new(2.0, Color32::BLACK))
            .fill(Color32::from_rgba_unmultiplied(200, 200, 200, alpha)),
        );
        if button_res.clicked() {
            streaming_state.is_streaming = !streaming_state.is_streaming;
            if streaming_state.is_streaming {
                streaming_state.viewers = streaming_state.subscribers;
                streaming_state.viewers_displayed = streaming_state.subscribers as f32;
                streaming_state.max_viewers = streaming_state.viewers;
            } else {
                streaming_state.viewers = 0;
                streaming_state.viewers_displayed = 0.0;
                streaming_state.max_viewers = 0;
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
