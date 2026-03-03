use bevy::prelude::*;
use bevy_egui::egui::{self, Color32, RichText};

use crate::game::apply_brainrot_ui;
use crate::game::assets::WorldAssets;
use crate::game::chat::StreamingState;
use crate::game::delivery::FOODS;
use crate::game::phone::PhoneState;
use crate::game::{Creature, Player};

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DungeonDashScreen {
    #[default]
    Menu,
    Checkout,
}

#[derive(Resource, Default)]
pub struct DungeonDashSelection {
    pub selected_food: Option<usize>,
    pub tip_percentage: u32,
    pub checkout_start_time: f64,
}

pub trait MobileApp: Send + Sync {
    fn name(&self) -> &str;
    fn splash_name(&self) -> &str;
    fn icon(&self, assets: &WorldAssets) -> Handle<Image>;
    fn draw_content(
        &self,
        ui: &mut egui::Ui,
        phone_state: &mut PhoneState,
        streaming_state: &mut StreamingState,
        player: &mut Player,
        creature: &mut Creature,
        player_pos: &crate::game::map::MapPos,
        active_delivery: &mut crate::game::delivery::ActiveDelivery,
        walk_blocked_map: &crate::game::map::WalkBlockedMap,
        scale: f32,
        alpha: u8,
        dd_screen: &DungeonDashScreen,
        next_dd_screen: &mut NextState<DungeonDashScreen>,
        dd_selection: &mut DungeonDashSelection,
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
        _player: &mut Player,
        _creature: &mut Creature,
        _player_pos: &crate::game::map::MapPos,
        _active_delivery: &mut crate::game::delivery::ActiveDelivery,
        _walk_blocked_map: &crate::game::map::WalkBlockedMap,
        _scale: f32,
        _alpha: u8,
        _dd_screen: &DungeonDashScreen,
        _next_dd_screen: &mut NextState<DungeonDashScreen>,
        _dd_selection: &mut DungeonDashSelection,
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
        ui: &mut egui::Ui,
        _phone_state: &mut PhoneState,
        _streaming_state: &mut StreamingState,
        player: &mut Player,
        creature: &mut Creature,
        player_pos: &crate::game::map::MapPos,
        active_delivery: &mut crate::game::delivery::ActiveDelivery,
        walk_blocked_map: &crate::game::map::WalkBlockedMap,
        scale: f32,
        alpha: u8,
        dd_screen: &DungeonDashScreen,
        next_dd_screen: &mut NextState<DungeonDashScreen>,
        dd_selection: &mut DungeonDashSelection,
    ) {
        let menu_alpha = ui.ctx().animate_bool_with_time(
            egui::Id::new("dd_menu_alpha"),
            *dd_screen == DungeonDashScreen::Menu,
            0.15,
        );

        if menu_alpha > 0.0 {
            let combined_alpha = (alpha as f32 * menu_alpha) as u8;
            self.draw_menu(
                ui,
                player,
                scale,
                combined_alpha,
                next_dd_screen,
                dd_selection,
            );
        } else if *dd_screen == DungeonDashScreen::Checkout {
            self.draw_checkout(
                ui,
                player,
                creature,
                player_pos,
                active_delivery,
                walk_blocked_map,
                scale,
                alpha,
                next_dd_screen,
                dd_selection,
            );
        }
    }
}

impl DungeonDash {
    fn draw_menu(
        &self,
        ui: &mut egui::Ui,
        player: &Player,
        scale: f32,
        alpha: u8,
        next_dd_screen: &mut NextState<DungeonDashScreen>,
        dd_selection: &mut DungeonDashSelection,
    ) {
        let item_height = 120.0 * scale;
        let spacing = 16.0 * scale;
        let total_height = FOODS.len() as f32 * (item_height + spacing);
        let available_height = ui.available_height();
        if total_height < available_height {
            ui.add_space((available_height - total_height) / 2.0);
        }

        egui::ScrollArea::vertical()
            .id_salt("dungeon_dash_food_list")
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    let width = ui.available_width() * 0.9;
                    for (i, food) in FOODS.iter().enumerate() {
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
                            ui.add_space(8.0 * scale);
                            ui.horizontal(|ui| {
                                ui.add_space(16.0 * scale);
                                ui.add(
                                    egui::Label::new(apply_brainrot_ui(
                                        RichText::new(food.name)
                                            .size(44.0 * scale)
                                            .color(Color32::BLACK),
                                        player.brainrot,
                                        ui.style(),
                                        egui::FontSelection::Default,
                                        egui::Align::LEFT,
                                    ))
                                    .selectable(false)
                                    .sense(egui::Sense::empty()),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.add_space(16.0 * scale);
                                        ui.add(
                                            egui::Label::new(apply_brainrot_ui(
                                                RichText::new(format!("${}", food.price))
                                                    .size(44.0 * scale)
                                                    .color(Color32::BLACK),
                                                player.brainrot,
                                                ui.style(),
                                                egui::FontSelection::Default,
                                                egui::Align::RIGHT,
                                            ))
                                            .selectable(false)
                                            .sense(egui::Sense::empty()),
                                        );
                                    },
                                );
                            });
                            ui.vertical_centered(|ui| {
                                ui.add(
                                    egui::Label::new(apply_brainrot_ui(
                                        RichText::new(food.effects).size(20.0 * scale).color(
                                            Color32::from_rgba_unmultiplied(80, 80, 80, alpha),
                                        ),
                                        player.brainrot,
                                        ui.style(),
                                        egui::FontSelection::Default,
                                        egui::Align::Center,
                                    ))
                                    .selectable(false)
                                    .sense(egui::Sense::empty()),
                                );
                            });
                        });

                        if response.clicked() {
                            next_dd_screen.set(DungeonDashScreen::Checkout);
                            dd_selection.selected_food = Some(i);
                            dd_selection.checkout_start_time = ui.input(|i| i.time);
                            dd_selection.tip_percentage = 15;
                        }

                        ui.add_space(16.0 * scale);
                    }
                });
            });
    }

    fn draw_checkout(
        &self,
        ui: &mut egui::Ui,
        player: &mut Player,
        _creature: &mut Creature,
        player_pos: &crate::game::map::MapPos,
        active_delivery: &mut crate::game::delivery::ActiveDelivery,
        walk_blocked_map: &crate::game::map::WalkBlockedMap,
        scale: f32,
        alpha: u8,
        next_dd_screen: &mut NextState<DungeonDashScreen>,
        dd_selection: &mut DungeonDashSelection,
    ) {
        let Some(food_idx) = dd_selection.selected_food else {
            next_dd_screen.set(DungeonDashScreen::Menu);
            return;
        };

        let food = FOODS[food_idx];
        let service_fee = (food.price as f32 * 0.3) as i32;
        let dungeon_tax = (food.price as f32 * 0.1) as i32;
        let delivery_fee = (food.price as f32 * 0.3) as i32;
        let subtotal = food.price + service_fee + dungeon_tax + delivery_fee;
        let tip = (subtotal as f32 * (dd_selection.tip_percentage as f32 / 100.0)) as i32;
        let total = subtotal + tip;

        let elapsed = ui.input(|i| i.time) - dd_selection.checkout_start_time;
        let step = 0.15; // Animation step duration

        ui.vertical_centered(|ui| {
            // 0. Header
            let header_alpha = 1.0;
            ui.add_space(40.0 * scale);
            ui.add(
                egui::Label::new(apply_brainrot_ui(
                    RichText::new("Checkout").size(64.0 * scale).color(
                        Color32::from_rgba_unmultiplied(
                            0,
                            0,
                            0,
                            (alpha as f32 * header_alpha) as u8,
                        ),
                    ),
                    player.brainrot,
                    ui.style(),
                    egui::FontSelection::Default,
                    egui::Align::Center,
                ))
                .selectable(false)
                .sense(egui::Sense::empty()),
            );

            ui.add_space(40.0 * scale);

            let lines = [
                (format!("{}", food.name), format!("${}", food.price)),
                ("Service Fee (30%)".to_string(), format!("${}", service_fee)),
                ("Dungeon Tax (10%)".to_string(), format!("${}", dungeon_tax)),
                (
                    "Delivery Fee (30%)".to_string(),
                    format!("${}", delivery_fee),
                ),
            ];

            // 1-4. Bill Lines
            for (i, (label, val)) in lines.iter().enumerate() {
                let delay = (i + 1) as f64 * step;
                let line_alpha = if elapsed >= delay { 1.0 } else { 0.0 };
                if line_alpha > 0.0 {
                    let color =
                        Color32::from_rgba_unmultiplied(0, 0, 0, (alpha as f32 * line_alpha) as u8);
                    ui.horizontal(|ui| {
                        ui.add_space(20.0 * scale);
                        ui.add(
                            egui::Label::new(apply_brainrot_ui(
                                RichText::new(label).size(32.0 * scale).color(color),
                                player.brainrot,
                                ui.style(),
                                egui::FontSelection::Default,
                                egui::Align::LEFT,
                            ))
                            .selectable(false)
                            .sense(egui::Sense::empty()),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(20.0 * scale);
                            ui.add(
                                egui::Label::new(apply_brainrot_ui(
                                    RichText::new(val).size(32.0 * scale).color(color),
                                    player.brainrot,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::RIGHT,
                                ))
                                .selectable(false)
                                .sense(egui::Sense::empty()),
                            );
                        });
                    });
                    ui.add_space(8.0 * scale);
                }
            }

            // 5. Tip Label
            let tip_label_delay = 5.0 * step;
            let tip_label_alpha = if elapsed >= tip_label_delay { 1.0 } else { 0.0 };
            if tip_label_alpha > 0.0 {
                let color = Color32::from_rgba_unmultiplied(
                    0,
                    0,
                    0,
                    (alpha as f32 * tip_label_alpha) as u8,
                );
                ui.horizontal(|ui| {
                    ui.add_space(20.0 * scale);
                    ui.add(
                        egui::Label::new(apply_brainrot_ui(
                            RichText::new("Tip").size(32.0 * scale).color(color),
                            player.brainrot,
                            ui.style(),
                            egui::FontSelection::Default,
                            egui::Align::LEFT,
                        ))
                        .selectable(false)
                        .sense(egui::Sense::empty()),
                    );
                });
            }

            // 6. Tip Radio Buttons
            let tip_radio_delay = 6.0 * step;
            let tip_radio_alpha = if elapsed >= tip_radio_delay { 1.0 } else { 0.0 };
            if tip_radio_alpha > 0.0 {
                let color = Color32::from_rgba_unmultiplied(
                    0,
                    0,
                    0,
                    (alpha as f32 * tip_radio_alpha) as u8,
                );
                ui.horizontal(|ui| {
                    ui.vertical_centered(|ui| {
                        // Make radio buttons smaller
                        ui.style_mut().spacing.interact_size.y = 20.0 * scale;
                        ui.style_mut().spacing.icon_width = 16.0 * scale;

                        ui.horizontal(|ui| {
                            ui.add_space(ui.available_width() * 0.2); // Rough centering
                            for t in [25, 20, 15] {
                                let res = ui.radio_value(
                                    &mut dd_selection.tip_percentage,
                                    t,
                                    RichText::new(format!("{}%", t)).color(color),
                                );
                                if res.changed() {
                                    ui.ctx().request_repaint();
                                }
                            }
                        });
                    });
                });
                ui.add_space(16.0 * scale);
            }

            // 7. Total
            let total_delay = 7.0 * step;
            let total_alpha = if elapsed >= total_delay { 1.0 } else { 0.0 };
            if total_alpha > 0.0 {
                let color =
                    Color32::from_rgba_unmultiplied(0, 0, 0, (alpha as f32 * total_alpha) as u8);
                ui.add_space(8.0 * scale);
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 2.0 * scale),
                    egui::Sense::hover(),
                );
                ui.painter().line_segment(
                    [
                        rect.left_top() + egui::vec2(20.0 * scale, 0.0),
                        rect.right_top() - egui::vec2(20.0 * scale, 0.0),
                    ],
                    egui::Stroke::new(2.0, color),
                );
                ui.add_space(16.0 * scale);

                ui.horizontal(|ui| {
                    ui.add_space(20.0 * scale);
                    ui.add(
                        egui::Label::new(apply_brainrot_ui(
                            RichText::new("Total")
                                .size(48.0 * scale)
                                .strong()
                                .color(color),
                            player.brainrot,
                            ui.style(),
                            egui::FontSelection::Default,
                            egui::Align::LEFT,
                        ))
                        .selectable(false)
                        .sense(egui::Sense::empty()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(20.0 * scale);
                        ui.add(
                            egui::Label::new(apply_brainrot_ui(
                                RichText::new(format!("${}", total))
                                    .size(48.0 * scale)
                                    .strong()
                                    .color(color),
                                player.brainrot,
                                ui.style(),
                                egui::FontSelection::Default,
                                egui::Align::RIGHT,
                            ))
                            .selectable(false)
                            .sense(egui::Sense::empty()),
                        );
                    });
                });
            }

            let est_delay = 7.0 * step;
            let est_alpha = if elapsed >= est_delay { 1.0 } else { 0.0 };
            if est_alpha > 0.0 {
                let color =
                    Color32::from_rgba_unmultiplied(80, 80, 80, (alpha as f32 * est_alpha) as u8);
                let estimate = match dd_selection.tip_percentage {
                    25 => "5-10 turns",
                    20 => "10-30 turns",
                    _ => "20-50 turns",
                };
                ui.add_space(20.0 * scale);
                ui.label(apply_brainrot_ui(
                    RichText::new(format!("Delivery Estimate: {}", estimate))
                        .size(24.0 * scale)
                        .color(color),
                    player.brainrot,
                    ui.style(),
                    egui::FontSelection::Default,
                    egui::Align::Center,
                ));
            }

            let buy_delay = 7.0 * step;
            let buy_alpha = if elapsed >= buy_delay { 1.0 } else { 0.0 };
            if buy_alpha > 0.0 {
                ui.add_space(40.0 * scale);
                let can_pay = player.money >= total;
                let button_color = if can_pay {
                    Color32::from_rgba_unmultiplied(100, 200, 100, (alpha as f32 * buy_alpha) as u8)
                } else {
                    Color32::from_rgba_unmultiplied(150, 150, 150, (alpha as f32 * buy_alpha) as u8)
                };

                let button = ui.add_enabled(
                    can_pay,
                    egui::Button::new(
                        RichText::new("BUY")
                            .size(64.0 * scale)
                            .color(Color32::BLACK),
                    )
                    .fill(button_color)
                    .stroke(egui::Stroke::new(2.0, Color32::BLACK)),
                );

                if button.clicked() {
                    player.money -= total;

                    let mut target_pos = player_pos.0;
                    let maxdist = 5;
                    let reachable = |p: crate::game::map::MapPos| p.adjacent();
                    let mut possible_spots = vec![];
                    for path in rogue_algebra::path::bfs_paths(&[*player_pos], maxdist, reachable) {
                        if let Some(pos) = path.last() {
                            if !walk_blocked_map.0.contains(&pos.0) {
                                possible_spots.push(pos.0);
                            }
                        }
                    }

                    if !possible_spots.is_empty() {
                        use rand::seq::IndexedRandom;
                        let mut rng = rand::rng();
                        target_pos = *possible_spots.choose(&mut rng).unwrap();
                    }

                    use rand::Rng;
                    let turns_remaining = match dd_selection.tip_percentage {
                        25 => rand::rng().random_range(5..=10),
                        20 => rand::rng().random_range(10..=30),
                        _ => rand::rng().random_range(20..=50),
                    };

                    active_delivery
                        .deliveries
                        .push(crate::game::delivery::Delivery {
                            turns_remaining,
                            target_pos,
                            food_idx,
                        });

                    next_dd_screen.set(DungeonDashScreen::Menu);
                }
            }

            let back_delay = 7.0 * step;
            let back_alpha = if elapsed >= back_delay { 1.0 } else { 0.0 };
            if back_alpha > 0.0 {
                ui.add_space(20.0 * scale);
                let button_color = Color32::from_rgba_unmultiplied(
                    200,
                    200,
                    200,
                    (alpha as f32 * back_alpha) as u8,
                );
                let back_button = ui.add(
                    egui::Button::new(
                        RichText::new("Back")
                            .size(32.0 * scale)
                            .color(Color32::BLACK),
                    )
                    .fill(button_color)
                    .stroke(egui::Stroke::new(2.0, Color32::BLACK)),
                );
                if back_button.clicked() {
                    next_dd_screen.set(DungeonDashScreen::Menu);
                }
            }
        });

        if elapsed < (11.0 * step + 0.5) {
            ui.ctx().request_repaint();
        }
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
        player: &mut Player,
        _creature: &mut Creature,
        _player_pos: &crate::game::map::MapPos,
        _active_delivery: &mut crate::game::delivery::ActiveDelivery,
        _walk_blocked_map: &crate::game::map::WalkBlockedMap,
        scale: f32,
        alpha: u8,
        _dd_screen: &DungeonDashScreen,
        _next_dd_screen: &mut NextState<DungeonDashScreen>,
        _dd_selection: &mut DungeonDashSelection,
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
