use bevy::prelude::*;
use bevy_egui::egui::{self, Color32, RichText};

use crate::game::apply_brainrot_ui;
use crate::game::assets::WorldAssets;
use crate::game::chat::StreamingState;
use crate::game::delivery::FOODS;
use crate::game::phone::{PhoneScreen, PhoneState};
use crate::game::upgrades::{UPGRADES, UpgradeMessage};
use crate::game::{Creature, Player};

const FROG_HANDLES: &[&str] = &["@Hopper", "@Ribbit", "@SwampKing"];
const GYM_BRO_HANDLES: &[&str] = &["@LiftHeavy", "@ProteinShake", "@DoYouEvenLift"];
const INFLUENCER_HANDLES: &[&str] = &["@LikeAndSubscribe", "@SponCon", "@TrendSetter"];
const NORMIE_HANDLES: &[&str] = &["@JustAGuy", "@AverageJoe", "@JohnDoe"];
const AMOGUS_HANDLES: &[&str] = &["@Sus", "@Imposter", "@RedIsSus"];
const CAPYBARA_HANDLES: &[&str] = &["@ChillVibes", "@WaterDog", "@OkayPullUp"];

const FROG_CONTENTS: &[&str] = &[
    "Ribbit ribbit...",
    "Looking for flies.",
    "It is Wednesday, my dudes.",
];
const GYM_BRO_CONTENTS: &[&str] = &[
    "Just hit a new PR! #gains",
    "Don't skip leg day bro.",
    "Where is my pre-workout?",
];
const INFLUENCER_CONTENTS: &[&str] = &[
    "New unboxing video dropping soon!  ",
    "Feeling blessed today.",
    "Link in bio!  ",
];
const NORMIE_CONTENTS: &[&str] = &[
    "What is going on?",
    "I just want to go home.",
    "Another day, another dollar.",
];
const AMOGUS_CONTENTS: &[&str] = &[
    "Doing tasks in electrical.",
    "I saw someone vent.",
    "Blue is acting sus.",
];
const CAPYBARA_CONTENTS: &[&str] = &["Chilling.", "Water is nice.", "Pull up."];

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

#[derive(Clone)]
pub struct Tweet {
    pub handle: String,
    pub content: String,
    pub hours_ago: u32,
    pub eggs: u32,
    pub glyph: char,
    pub color: Color32,
}

#[derive(Resource, Default)]
pub struct CockatriceState {
    pub tweets: Vec<Tweet>,
    pub scroll_timer: f64,
    pub initialized: bool,
    pub turn_timer: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum AppId {
    Crawlr,
    DungeonDash,
    UndergroundTV,
    Cockatrice,
    Upgrade,
}

pub trait MobileApp: Send + Sync {
    fn name(&self) -> &str;
    fn splash_name(&self) -> &str;
    fn icon(&self, assets: &WorldAssets) -> Option<Handle<Image>>;
    fn show_on_home_screen(&self) -> bool {
        true
    }
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
        msg_upgrade: &mut MessageWriter<UpgradeMessage>,
        next_phone_screen: &mut NextState<PhoneScreen>,
        cockatrice_state: &mut CockatriceState,
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
    fn icon(&self, assets: &WorldAssets) -> Option<Handle<Image>> {
        Some(assets.phone_app_icons.crawlr.clone())
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
        _msg_upgrade: &mut MessageWriter<UpgradeMessage>,
        _next_phone_screen: &mut NextState<PhoneScreen>,
        _cockatrice_state: &mut CockatriceState,
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
    fn icon(&self, assets: &WorldAssets) -> Option<Handle<Image>> {
        Some(assets.phone_app_icons.dungeon_dash.clone())
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
        _msg_upgrade: &mut MessageWriter<UpgradeMessage>,
        _next_phone_screen: &mut NextState<PhoneScreen>,
        _cockatrice_state: &mut CockatriceState,
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

        let has_platinum = player.has_subscription(crate::game::Subscription::DungeonDashPlatinum);

        egui::ScrollArea::vertical()
            .id_salt("dungeon_dash_food_list")
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    let width = ui.available_width() * 0.9;
                    for (i, food) in FOODS.iter().enumerate() {
                        let mut food_price = food.price;
                        if has_platinum {
                            food_price = (food_price as f32 * 0.25) as i32;
                        }

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
                                                RichText::new(format!("${}", food_price))
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
        let has_platinum = player.has_subscription(crate::game::Subscription::DungeonDashPlatinum);
        let mut food_price = food.price;
        if has_platinum {
            food_price = (food_price as f32 * 0.25) as i32;
        }

        let service_fee = (food_price as f32 * 0.3) as i32;
        let dungeon_tax = (food_price as f32 * 0.1) as i32;
        let delivery_fee = (food_price as f32 * 0.3) as i32;
        let subtotal = food_price + service_fee + dungeon_tax + delivery_fee;
        let tip = if has_platinum {
            0
        } else {
            (subtotal as f32 * (dd_selection.tip_percentage as f32 / 100.0)) as i32
        };
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
                (food.name.to_string(), format!("${}", food_price)),
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
            if tip_label_alpha > 0.0 && !has_platinum {
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
            if tip_radio_alpha > 0.0 && !has_platinum {
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
                let estimate = if has_platinum {
                    "5 turns"
                } else {
                    match dd_selection.tip_percentage {
                        25 => "5-10 turns",
                        20 => "10-30 turns",
                        _ => "20-50 turns",
                    }
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
                        if let Some(pos) = path.last()
                            && !walk_blocked_map.0.contains(&pos.0)
                        {
                            possible_spots.push(pos.0);
                        }
                    }

                    if !possible_spots.is_empty() {
                        use rand::seq::IndexedRandom;
                        let mut rng = rand::rng();
                        target_pos = *possible_spots.choose(&mut rng).unwrap();
                    }

                    use rand::Rng;
                    let turns_remaining = if player
                        .has_subscription(crate::game::Subscription::DungeonDashPlatinum)
                    {
                        5
                    } else {
                        match dd_selection.tip_percentage {
                            25 => rand::rng().random_range(5..=10),
                            20 => rand::rng().random_range(10..=30),
                            _ => rand::rng().random_range(20..=50),
                        }
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
    fn icon(&self, assets: &WorldAssets) -> Option<Handle<Image>> {
        Some(assets.phone_app_icons.underground_tv.clone())
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
        _msg_upgrade: &mut MessageWriter<UpgradeMessage>,
        _next_phone_screen: &mut NextState<PhoneScreen>,
        _cockatrice_state: &mut CockatriceState,
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
                streaming_state.viewers_fractional = 0.0;
            }
        }
    }
}

pub struct Cockatrice;

impl MobileApp for Cockatrice {
    fn name(&self) -> &str {
        "Cockatrice"
    }
    fn splash_name(&self) -> &str {
        "Cockatrice"
    }
    fn icon(&self, assets: &WorldAssets) -> Option<Handle<Image>> {
        Some(assets.phone_app_icons.cockatrice.clone())
    }
    fn draw_content(
        &self,
        ui: &mut egui::Ui,
        _phone_state: &mut PhoneState,
        _streaming_state: &mut StreamingState,
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
        _msg_upgrade: &mut MessageWriter<UpgradeMessage>,
        _next_phone_screen: &mut NextState<PhoneScreen>,
        cockatrice_state: &mut CockatriceState,
    ) {
        if cockatrice_state.tweets.is_empty() {
            ui.add_space(ui.available_height() * 0.4);
            ui.label(apply_brainrot_ui(
                RichText::new("No posts yet...")
                    .size(32.0 * scale)
                    .color(Color32::from_rgba_unmultiplied(0, 0, 0, alpha)),
                player.brainrot,
                ui.style(),
                egui::FontSelection::Default,
                egui::Align::Center,
            ));
            return;
        }

        let container_width = ui.available_width() * 0.9;
        let tweet_height = 300.0 * scale;
        let tweet_spacing = 32.0 * scale;
        let item_height = tweet_height + tweet_spacing;

        let total_height = cockatrice_state.tweets.len() as f32 * item_height;
        let scroll_speed = 100.0 * scale; // pixels per second
        let scroll_offset = (cockatrice_state.scroll_timer as f32 * scroll_speed) % total_height;

        ui.style_mut().spacing.item_spacing.y = 0.0;

        egui::ScrollArea::vertical()
            .id_salt("cockatrice_scroll")
            .vertical_scroll_offset(scroll_offset)
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(16.0 * scale); // Top padding to match the previous look
                    // Draw 2 copies so that when offset reaches total_height, the screen is still filled
                    for _ in 0..2 {
                        for tweet in &cockatrice_state.tweets {
                            // We allocate a rect exactly the size of the *visible* tweet card
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(container_width, tweet_height),
                                egui::Sense::hover(),
                            );

                            let fill = Color32::from_rgba_unmultiplied(250, 250, 245, alpha);
                            ui.painter().rect_filled(rect, 4.0 * scale, fill);
                            ui.painter().rect_stroke(
                                rect,
                                4.0 * scale,
                                egui::Stroke::new(2.0, Color32::BLACK),
                                egui::StrokeKind::Middle,
                            );

                            // create a child UI specifically tied to `rect` and NOT inheriting the cursor pos
                            let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));

                            child_ui.add_space(16.0 * scale);
                            child_ui.horizontal(|child_ui| {
                                child_ui.add_space(16.0 * scale);

                                // Profile Icon
                                let icon_size = 64.0 * scale;
                                let (icon_rect, _) = child_ui.allocate_exact_size(
                                    egui::vec2(icon_size, icon_size),
                                    egui::Sense::hover(),
                                );

                                child_ui.painter().rect_filled(
                                    icon_rect,
                                    2.0 * scale,
                                    Color32::from_rgba_unmultiplied(200, 200, 200, alpha),
                                );
                                child_ui.painter().rect_stroke(
                                    icon_rect,
                                    2.0 * scale,
                                    egui::Stroke::new(2.0, Color32::BLACK),
                                    egui::StrokeKind::Middle,
                                );

                                // Draw monster glyph as icon
                                let text_job = apply_brainrot_ui(
                                    RichText::new(tweet.glyph.to_string())
                                        .size(48.0 * scale)
                                        .color(Color32::from_rgba_unmultiplied(
                                            tweet.color.r(),
                                            tweet.color.g(),
                                            tweet.color.b(),
                                            (tweet.color.a() as f32 * (alpha as f32 / 255.0)) as u8,
                                        )),
                                    player.brainrot,
                                    child_ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                )
                                .into_layout_job(
                                    child_ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                );
                                let galley = child_ui.painter().layout_job((*text_job).clone());
                                child_ui.painter().galley(
                                    egui::pos2(
                                        icon_rect.center().x - galley.size().x / 2.0,
                                        icon_rect.center().y - galley.size().y / 2.0,
                                    ),
                                    galley,
                                    Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
                                );

                                child_ui.add_space(16.0 * scale);

                                // Username and Time
                                child_ui.vertical(|child_ui| {
                                    child_ui.horizontal(|child_ui| {
                                        child_ui.add(
                                            egui::Label::new(apply_brainrot_ui(
                                                RichText::new(&tweet.handle)
                                                    .size(32.0 * scale)
                                                    .color(Color32::from_rgba_unmultiplied(
                                                        0, 50, 100, alpha,
                                                    )),
                                                player.brainrot,
                                                child_ui.style(),
                                                egui::FontSelection::Default,
                                                egui::Align::LEFT,
                                            ))
                                            .selectable(false)
                                            .sense(egui::Sense::empty()),
                                        );

                                        child_ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |child_ui| {
                                                child_ui.add_space(16.0 * scale);
                                                child_ui.add(
                                                    egui::Label::new(apply_brainrot_ui(
                                                        RichText::new(format!(
                                                            "{}h",
                                                            tweet.hours_ago
                                                        ))
                                                        .size(28.0 * scale)
                                                        .color(Color32::from_rgba_unmultiplied(
                                                            100, 100, 100, alpha,
                                                        )),
                                                        player.brainrot,
                                                        child_ui.style(),
                                                        egui::FontSelection::Default,
                                                        egui::Align::RIGHT,
                                                    ))
                                                    .selectable(false)
                                                    .sense(egui::Sense::empty()),
                                                );
                                            },
                                        );
                                    });
                                });
                            });

                            child_ui.add_space(16.0 * scale);

                            // Content
                            child_ui.horizontal(|child_ui| {
                                child_ui.add_space(20.0 * scale);
                                child_ui.add(
                                    egui::Label::new(apply_brainrot_ui(
                                        RichText::new(&tweet.content)
                                            .size(36.0 * scale)
                                            .color(Color32::from_rgba_unmultiplied(0, 0, 0, alpha)),
                                        player.brainrot,
                                        child_ui.style(),
                                        egui::FontSelection::Default,
                                        egui::Align::LEFT,
                                    ))
                                    .wrap_mode(egui::TextWrapMode::Wrap)
                                    .selectable(false)
                                    .sense(egui::Sense::empty()),
                                );
                            });

                            child_ui.add_space(24.0 * scale);

                            // Footer (Eggs)
                            child_ui.horizontal(|child_ui| {
                                child_ui.add_space(20.0 * scale);
                                child_ui.add(
                                    egui::Label::new(apply_brainrot_ui(
                                        RichText::new(format!("{} Eggs", tweet.eggs))
                                            .size(24.0 * scale)
                                            .color(Color32::from_rgba_unmultiplied(
                                                80, 80, 80, alpha,
                                            )),
                                        player.brainrot,
                                        child_ui.style(),
                                        egui::FontSelection::Default,
                                        egui::Align::LEFT,
                                    ))
                                    .selectable(false)
                                    .sense(egui::Sense::empty()),
                                );
                            });

                            // Ensure the gap between tweets is explicitly allocated
                            ui.allocate_exact_size(
                                egui::vec2(container_width, tweet_spacing),
                                egui::Sense::hover(),
                            );
                        }
                    }
                });
            });
    }
}

pub struct Upgrade;

impl MobileApp for Upgrade {
    fn name(&self) -> &str {
        "Upgrade"
    }
    fn splash_name(&self) -> &str {
        "Upgrade"
    }
    fn icon(&self, _assets: &WorldAssets) -> Option<Handle<Image>> {
        None
    }
    fn show_on_home_screen(&self) -> bool {
        false
    }
    fn draw_content(
        &self,
        ui: &mut egui::Ui,
        _phone_state: &mut PhoneState,
        _streaming_state: &mut StreamingState,
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
        msg_upgrade: &mut MessageWriter<UpgradeMessage>,
        next_phone_screen: &mut NextState<PhoneScreen>,
        _cockatrice_state: &mut CockatriceState,
    ) {
        ui.add_space(40.0 * scale);
        ui.label(apply_brainrot_ui(
            RichText::new("Choose an Upgrade")
                .size(48.0 * scale)
                .color(Color32::from_rgba_unmultiplied(0, 0, 0, alpha)),
            player.brainrot,
            ui.style(),
            egui::FontSelection::Default,
            egui::Align::Center,
        ));
        ui.add_space(40.0 * scale);

        ui.vertical_centered(|ui| {
            let width = ui.available_width() * 0.9;
            for upgrade_idx in &player.upgrade_options {
                let upgrade = &UPGRADES[*upgrade_idx];
                let height = 180.0 * scale;
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
                    ui.add_space(16.0 * scale);
                    ui.vertical_centered(|ui| {
                        ui.add(
                            egui::Label::new(apply_brainrot_ui(
                                RichText::new(upgrade.name)
                                    .size(36.0 * scale)
                                    .color(Color32::BLACK),
                                player.brainrot,
                                ui.style(),
                                egui::FontSelection::Default,
                                egui::Align::Center,
                            ))
                            .selectable(false)
                            .sense(egui::Sense::empty()),
                        );
                        ui.add_space(8.0 * scale);
                        ui.add(
                            egui::Label::new(apply_brainrot_ui(
                                RichText::new(upgrade.describe())
                                    .size(24.0 * scale)
                                    .color(Color32::from_rgba_unmultiplied(80, 80, 80, alpha)),
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
                    msg_upgrade.write(UpgradeMessage {
                        upgrade: *upgrade_idx,
                    });
                    next_phone_screen.set(PhoneScreen::Home);
                }

                ui.add_space(16.0 * scale);
            }
        });
    }
}

pub fn get_apps() -> Vec<(AppId, Box<dyn MobileApp>)> {
    vec![
        (AppId::Crawlr, Box::new(Crawlr)),
        (AppId::DungeonDash, Box::new(DungeonDash)),
        (AppId::UndergroundTV, Box::new(UndergroundTV)),
        (AppId::Cockatrice, Box::new(Cockatrice)),
        (AppId::Upgrade, Box::new(Upgrade)),
    ]
}

pub fn update_cockatrice(
    time: Res<Time>,
    mut state: ResMut<CockatriceState>,
    mob_query: Query<(&Name, &Text2d, &TextColor), With<crate::game::Mob>>,
    phone_state: Res<PhoneState>,
    current_screen: Res<State<PhoneScreen>>,
    mut commands: Commands,
    player_query: Single<(Entity, &mut Player, &mut Creature)>,
) {
    if !state.initialized {
        use rand::Rng;
        let mut rng = rand::rng();
        for (name, text, color) in mob_query.iter() {
            let handle = match name.as_str() {
                "Giant Frog" => FROG_HANDLES[rng.random_range(0..FROG_HANDLES.len())],
                "Gym Bro" => GYM_BRO_HANDLES[rng.random_range(0..GYM_BRO_HANDLES.len())],
                "Influencer" => INFLUENCER_HANDLES[rng.random_range(0..INFLUENCER_HANDLES.len())],
                "Normie" => NORMIE_HANDLES[rng.random_range(0..NORMIE_HANDLES.len())],
                "Amogus" => AMOGUS_HANDLES[rng.random_range(0..AMOGUS_HANDLES.len())],
                "Capybara" => CAPYBARA_HANDLES[rng.random_range(0..CAPYBARA_HANDLES.len())],
                _ => "@Monster",
            };

            let content = match name.as_str() {
                "Giant Frog" => FROG_CONTENTS[rng.random_range(0..FROG_CONTENTS.len())],
                "Gym Bro" => GYM_BRO_CONTENTS[rng.random_range(0..GYM_BRO_CONTENTS.len())],
                "Influencer" => INFLUENCER_CONTENTS[rng.random_range(0..INFLUENCER_CONTENTS.len())],
                "Normie" => NORMIE_CONTENTS[rng.random_range(0..NORMIE_CONTENTS.len())],
                "Amogus" => AMOGUS_CONTENTS[rng.random_range(0..AMOGUS_CONTENTS.len())],
                "Capybara" => CAPYBARA_CONTENTS[rng.random_range(0..CAPYBARA_CONTENTS.len())],
                _ => "Rawr!",
            };

            let [r, g, b, a] = color.0.to_srgba().to_u8_array();
            state.tweets.push(Tweet {
                handle: handle.to_string(),
                content: content.to_string(),
                hours_ago: rng.random_range(1..24),
                eggs: rng.random_range(0..1000),
                glyph: text.0.chars().next().unwrap_or('?'),
                color: Color32::from_rgba_unmultiplied(r, g, b, a),
            });
        }
        use rand::seq::SliceRandom;
        state.tweets.shuffle(&mut rng);
        state.initialized = true;
    }

    if let PhoneScreen::App(AppId::Cockatrice) = current_screen.get() {
        if phone_state.is_open {
            let delta = time.delta_secs();
            state.scroll_timer += delta as f64;
            state.turn_timer += delta;

            if state.turn_timer >= 2.0 {
                state.turn_timer -= 2.0;

                let (entity, mut player, creature) = player_query.into_inner();
                if creature.hp > 0 {
                    player.brainrot += 5;
                    player.brainrot = player.brainrot.clamp(0, 100);
                    player.boredom -= 10;
                    player.boredom = player.boredom.clamp(0, 100);

                    commands
                        .entity(entity)
                        .insert(crate::game::input::PlayerIntent::Wait);
                    commands.run_schedule(crate::game::Turn);
                }
            }
        }
    }
}
