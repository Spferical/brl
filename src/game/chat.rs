use crate::game::animation::DamageAnimationMessage;
use crate::game::apply_brainrot_ui;
use crate::game::{DamageType, Player, TurnCounter};
use bevy::prelude::*;
use bevy_egui::EguiContexts;
use bevy_egui::egui::{self, Color32, RichText};
use rand::{Rng, seq::IndexedRandom};

use std::collections::VecDeque;

const MAX_CHAT_MESSAGES: usize = 15;
const MESSAGE_LIFETIME: f32 = 8.0;

const USERNAMES: &[&str] = &[
    "WizardOfYendor",
    "Rodney",
    "PogChamp",
    "BrainrotEnjoyer",
    "Rizzler",
    "NoCapFr",
    "Griswold",
    "Deckard_Cain",
    "DiabloLover",
    "Gyatt",
    "DungeonBackseater",
    "YoSoyUnKobold",
    "BasementDweller",
];

const GENERIC_MESSAGES: &[&str] = &[
    "POG",
    "L",
    "W",
    "Cringe",
    "Skibidi",
    "Rizz",
    "No cap",
    "FR FR",
    "ResidentSleeper",
    "Go faster",
    "Where are you going?",
    "LMAO",
    "LOL",
    "is this a speedrun?",
    "this is a vibe",
    "poggers",
    "monkaS",
    "Kappa",
];

const ATTACK_MESSAGES: &[&str] = &[
    "GET EM", "RIP", "EZ", "L MOB", "W ATTACK", "BONK", "CRIT??", "SHEEEESH", "11",
];

const DAMAGE_MESSAGES: &[&str] = &[
    "F",
    "L",
    "SKILL ISSUE",
    "HEALING?",
    "WATCH OUT",
    "OH NO",
    "RIP BOZO",
    "Rizz issue",
    "Brainrot moment",
    "GET GUD",
    "gg",
    "ggwp",
    "get wrecked lol",
];

const DONO_MESSAGES: &[&str] = &[
    "Nice kill!",
    "GET EM",
    "EZ MONEY",
    "Keep it up",
    "W",
    "POG",
    "Insane",
    "LETS GO",
];

const WHALE_MESSAGES: &[&str] = &[
    "HOLY COW",
    "GOAT",
    "MY HERO",
    "I LOVE THIS STREAM",
    "MARRY ME",
    "HAVE MY MONEY",
    "ABSOLUTE UNIT",
    "W STREAMER",
];

const FOOD_MESSAGES: &[&str] = &[
    "FOOD'S HERE",
    "FOOOOOOD",
    "CAN I HAVE SOME??",
    "W DELIVERY",
    "I'm hungry now",
    "eat it eat it",
    "is that a pizza?",
    "W DINNER",
    "private taxi for your burrito lmao",
    "mukbang stream when?",
    "tastyyyy?",
];

const CLOTHING_MESSAGES: &[&str] = &[
    "SHEEEESH THE DRIP",
    "W FIT",
    "NEW DRIP JUST DROPPED",
    "INSANE RIZZ",
    "RIZZLER MOMENT",
    "HE'S HIM",
    "SHE'S HER",
    "THE AURA IS INSANE",
    "POG FIT",
    "W PURCHASE",
];

const MOG_MESSAGES: &[&str] = &[
    "MOGGED",
    "SHEEEESH THE JAWLINE",
    "LIGHTNING BOLT EMOJI",
    "HE'S MOGGING THEM",
    "LIL BRO GOT MOGGED",
    "COULD NEVER BE ME",
    "AURA UNLIMITED",
];

const AURA_LOSS_MESSAGES: &[&str] = &[
    "AURA LOSS",
    "L RIZZ",
    "YOU GOT MOGGED",
    "LOOK AWAY",
    "ITS OVER",
    "NEGATIVE AURA",
    "DEBATING UNFOLLOWING",
    "CRINGE",
    "CURSED",
    "WHERES THE RIZZ??",
];

const BROKE_MESSAGES: &[&str] = &[
    "u broke",
    "does bankruptcy stack?",
    "the Klarna Kop is coming",
    "broke boi",
    "RIP your credit score",
    "L credit limit",
    "can't afford a burrito?",
    "bro is in the negative right now",
];

#[derive(Resource)]
pub struct ChatHistory {
    pub messages: Vec<ChatMessage>,
    pub queue: VecDeque<ChatMessage>,
    pub spawn_timer: Timer,
    pub pop_timer: Timer,
}

impl Default for ChatHistory {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            queue: VecDeque::new(),
            spawn_timer: Timer::from_seconds(1.0, TimerMode::Once),
            pop_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
        }
    }
}

pub struct ChatMessage {
    pub username: String,
    pub text: String,
    pub timer: Timer,
    pub donation: Option<i32>,
}

#[derive(Resource, Default)]
pub struct StreamingState {
    pub is_streaming: bool,
    pub subscribers: i32,
    pub max_viewers: i32,        // Max viewers in the current session
    pub viewers: i32,            // True count, updated on Turn
    pub viewers_displayed: f32,  // For UI animation
    pub viewers_fractional: f32, // For precise turn-based growth
    pub subscribers_fractional: f32,
}

pub fn update_streaming_stats(
    time: Res<Time>,
    mut streaming_state: ResMut<StreamingState>,
    _player: Single<&Player>,
) {
    if !streaming_state.is_streaming {
        streaming_state.viewers = 0;
        streaming_state.viewers_displayed = 0.0;
        streaming_state.max_viewers = 0;
        streaming_state.viewers_fractional = 0.0;
        return;
    }

    // Animate viewers_displayed towards viewers
    let target = streaming_state.viewers as f32;
    let diff = target - streaming_state.viewers_displayed;

    if diff.abs() > 0.1 {
        // Speed is proportional to the diff to ensure it roughly takes 1s
        // but has a minimum speed so it doesn't crawl at the end
        let speed = (diff.abs() / 1.0).max(10.0);
        let move_amt = speed * time.delta_secs();

        if diff > 0.0 {
            streaming_state.viewers_displayed =
                (streaming_state.viewers_displayed + move_amt).min(target);
        } else {
            streaming_state.viewers_displayed =
                (streaming_state.viewers_displayed - move_amt).max(target);
        }
    } else {
        streaming_state.viewers_displayed = target;
    }
}

pub fn update_streaming_turn(
    mut player: Single<&mut Player>,
    mut streaming_state: ResMut<StreamingState>,
    turn_counter: Res<TurnCounter>,
) {
    let sub_decay_factor = 0.98623; // half-life 50 turns
    let viewer_decay_factor = 0.933; // half-life 10 turns

    if streaming_state.is_streaming {
        if player.signal > 2 {
            // 1 brainrot every 10 turns
            if turn_counter.0.is_multiple_of(10) {
                player.brainrot += 1;
            }

            // Viewers growth per turn: a * e^(b * rizz)
            // b = ln(50)/40 = 0.0978 (50x increase from 10 to 50 rizz)
            // a = 100 / e^(100*b) = 0.00566 (100/turn at 100 rizz)
            let rizz = player.rizz as f32;
            let mut gain = 0.00566 * (0.0978 * rizz).exp();

            if player.has_subscription(crate::game::Subscription::UndergroundTVPro) {
                gain *= 3.0;
            }

            // Taper after 2000 viewers
            if streaming_state.viewers > 2000 {
                let overflow = (streaming_state.viewers - 2000) as f32;
                // 1 / n^2
                let factor = 2000.0 / (2000.0 + overflow);
                gain *= factor * factor;
            }

            streaming_state.viewers_fractional += gain;
            if streaming_state.viewers_fractional >= 1.0 {
                let i_gain = streaming_state.viewers_fractional.floor();
                streaming_state.viewers += i_gain as i32;
                streaming_state.viewers_fractional -= i_gain;
            }

            // 5% of max viewers this session
            streaming_state.max_viewers = streaming_state.max_viewers.max(streaming_state.viewers);
            let target_subs = (streaming_state.max_viewers as f32 * 0.05).floor() as i32;
            if target_subs > streaming_state.subscribers {
                streaming_state.subscribers = target_subs;
                streaming_state.subscribers_fractional = 0.0;
            }
        } else {
            // Streaming but lost signal: attenuate viewers aggressively but DO NOT decay subs
            let old_viewers = streaming_state.viewers as f32 + streaming_state.viewers_fractional;
            let new_viewers = old_viewers * viewer_decay_factor;
            streaming_state.viewers = new_viewers.floor() as i32;
            streaming_state.viewers_fractional = new_viewers.fract();
        }
    } else {
        // Not streaming: attenuate subscribers, viewers reset elsewhere
        let old_subs = streaming_state.subscribers as f32 + streaming_state.subscribers_fractional;
        let new_subs = old_subs * sub_decay_factor;
        streaming_state.subscribers = new_subs.floor() as i32;
        streaming_state.subscribers_fractional = new_subs.fract();

        streaming_state.viewers = 0;
        streaming_state.viewers_fractional = 0.0;
    }
}

pub fn update_money_timer(time: Res<Time>, mut player: Single<&mut Player>) {
    if player.money_gain_timer > 0.0 {
        player.money_gain_timer -= time.delta_secs();
    }
}

pub fn handle_payout(
    player: &mut Player,
    streaming_state: &StreamingState,
    chat: &mut ChatHistory,
) {
    if streaming_state.is_streaming && streaming_state.viewers > 0 {
        let mut rng = rand::rng();

        // $1 per 100 viewers
        let base_payout = (streaming_state.viewers as f32 / 100.0).max(1.0);

        // Add randomness: 20% to 250% of base
        let random_factor = rng.random_range(0.2..2.5);
        let mut payout = (base_payout * random_factor).round() as i32;

        // Whale chance: 1% chance for a massive 10x - 50x payout
        let mut is_whale = false;
        if rng.random_bool(0.01) {
            let multiplier = rng.random_range(10..50);
            payout *= multiplier;
            is_whale = true;
        }

        player.money += payout;
        player.last_gain_amount = payout;
        player.money_gain_timer = 2.0;

        // Queue a dono message
        let username = USERNAMES.choose(&mut rng).unwrap().to_string();
        let pool = if is_whale {
            WHALE_MESSAGES
        } else {
            DONO_MESSAGES
        };
        let text = pool.choose(&mut rng).unwrap().to_string();

        chat.queue.push_back(ChatMessage {
            username,
            text,
            timer: Timer::from_seconds(MESSAGE_LIFETIME, TimerMode::Once),
            donation: Some(payout),
        });
    }
}

pub fn queue_mog_message(chat: &mut ChatHistory, streaming_state: &StreamingState) {
    if streaming_state.is_streaming && streaming_state.viewers > 0 {
        let mut rng = rand::rng();
        queue_message(chat, &mut rng, MOG_MESSAGES);
    }
}

pub fn queue_food_delivery_message(
    chat: &mut ChatHistory,
    streaming_state: &StreamingState,
    food_idx: usize,
) {
    if streaming_state.is_streaming && streaming_state.viewers > 0 {
        let mut rng = rand::rng();
        let food = crate::game::delivery::FOODS[food_idx];
        if food.rizz > 0 {
            queue_message(chat, &mut rng, CLOTHING_MESSAGES);
        } else {
            queue_message(chat, &mut rng, FOOD_MESSAGES);
        }
    }
}

pub fn update_chat(
    time: Res<Time>,
    mut chat: ResMut<ChatHistory>,
    streaming_state: Res<StreamingState>,
    mut damage_events: MessageReader<DamageAnimationMessage>,
    player_query: Single<(&Player, Entity)>,
) {
    if !streaming_state.is_streaming || streaming_state.viewers == 0 {
        chat.messages.clear();
        chat.queue.clear();
        return;
    }

    let (player, player_entity) = *player_query;

    for msg in &mut chat.messages {
        msg.timer.tick(time.delta());
    }
    chat.messages.retain(|msg| !msg.timer.is_finished());

    let mut rng = rand::rng();

    // Event-based messages go to queue
    for event in damage_events.read() {
        if event.entity == player_entity {
            if event.ty == DamageType::Aura {
                queue_message(&mut chat, &mut rng, AURA_LOSS_MESSAGES);
            } else {
                queue_message(&mut chat, &mut rng, DAMAGE_MESSAGES);
            }
        } else {
            queue_message(&mut chat, &mut rng, ATTACK_MESSAGES);
        }
    }

    // Generic background messages go to queue
    chat.spawn_timer.tick(time.delta());
    if chat.spawn_timer.is_finished() {
        chat.spawn_timer
            .set_duration(std::time::Duration::from_secs_f32(
                rng.random_range(0.2..1.5),
            ));
        chat.spawn_timer.reset();

        // 1% chance per viewer, capped at 95%
        let spawn_chance = (streaming_state.viewers as f32 * 0.01).min(0.95);
        if rng.random_bool(spawn_chance as f64) {
            let pool = if player.money < 0 && rng.random_bool(0.3) {
                BROKE_MESSAGES
            } else {
                GENERIC_MESSAGES
            };
            queue_message(&mut chat, &mut rng, pool);
        }
    }

    // Pop from queue to visible messages over time
    chat.pop_timer.tick(time.delta());
    if chat.pop_timer.just_finished()
        && !chat.queue.is_empty()
        && let Some(msg) = chat.queue.pop_front()
    {
        chat.messages.push(msg);
        if chat.messages.len() > MAX_CHAT_MESSAGES {
            chat.messages.remove(0);
        }

        // Adjust pop speed based on queue size (clear backlog faster)
        let next_delay = if chat.queue.len() > 10 {
            0.05
        } else if chat.queue.len() > 5 {
            0.15
        } else {
            0.3
        };
        chat.pop_timer
            .set_duration(std::time::Duration::from_secs_f32(next_delay));
    }
}

fn queue_message(chat: &mut ChatHistory, rng: &mut impl Rng, pool: &[&str]) {
    let username = USERNAMES.choose(rng).unwrap().to_string();
    let text = pool.choose(rng).unwrap().to_string();

    chat.queue.push_back(ChatMessage {
        username,
        text,
        timer: Timer::from_seconds(MESSAGE_LIFETIME, TimerMode::Once),
        donation: None,
    });
}

pub fn draw_streaming_indicator(
    mut contexts: EguiContexts,
    streaming_state: Res<StreamingState>,
    player: Single<&Player>,
    active_delivery: Res<crate::game::delivery::ActiveDelivery>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let is_low_signal = player.signal <= 2;
    let (text, color) = if is_low_signal {
        ("Offline: Poor Signal", Color32::GRAY)
    } else {
        ("Streaming...", Color32::RED)
    };

    egui::Area::new(egui::Id::new("streaming_indicator"))
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 20.0))
        .show(ctx, |ui| {
            if streaming_state.is_streaming {
                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 8.0, color);
                    ui.label(
                        RichText::new(text)
                            .color(color)
                            .font(egui::FontId::new(
                                20.0,
                                egui::FontFamily::Name("press_start".into()),
                            ))
                            .strong(),
                    );
                });
            }

            for delivery in active_delivery.deliveries.iter() {
                ui.horizontal(|ui| {
                    ui.add_space(30.0); // Indent a bit
                    ui.label(
                        RichText::new(format!("Delivery in {} turns", delivery.turns_remaining))
                            .color(Color32::from_rgb(255, 165, 0)) // Orange
                            .font(egui::FontId::new(
                                16.0,
                                egui::FontFamily::Name("press_start".into()),
                            ))
                            .strong(),
                    );
                });
            }
        });
}

pub fn draw_chat(
    mut contexts: EguiContexts,
    chat: Res<ChatHistory>,
    streaming_state: Res<StreamingState>,
    player: Single<&Player>,
) {
    if !streaming_state.is_streaming
        || streaming_state.viewers == 0
        || chat.messages.is_empty()
        || player.signal <= 2
    {
        return;
    }

    let ctx = contexts.ctx_mut().unwrap();
    let screen_rect = ctx.content_rect();
    let chat_height_limit = screen_rect.height() / 3.0;

    // Position at bottom right
    egui::Area::new(egui::Id::new("chat_area"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-10.0, -10.0))
        .interactable(false)
        .show(ctx, |ui| {
            ui.set_max_width(300.0);
            ui.set_max_height(chat_height_limit);

            // Reverse scroll: latest at bottom
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;

                let num_messages = chat.messages.len();
                for (i, msg) in chat.messages.iter().rev().enumerate() {
                    // Fade out older messages
                    // Also fade out based on lifetime
                    let lifetime_alpha = (msg.timer.remaining_secs() / 2.0).min(1.0);
                    let position_alpha = 1.0 - (i as f32 / num_messages as f32).powi(2);
                    let alpha = lifetime_alpha * position_alpha;

                    if alpha <= 0.05 {
                        continue;
                    }

                    let mut text = format!("{}: {}", msg.username, msg.text);
                    if let Some(amt) = msg.donation {
                        text = format!("{} [${}]: {}", msg.username, amt, msg.text);
                    }

                    let job = apply_brainrot_ui(
                        text,
                        player.brainrot,
                        ui.style(),
                        egui::FontSelection::Default,
                        egui::Align::LEFT,
                    )
                    .into_layout_job(
                        ui.style(),
                        egui::FontSelection::Default,
                        egui::Align::LEFT,
                    );

                    // Apply alpha to all sections
                    let mut job = (*job).clone();
                    for section in &mut job.sections {
                        if msg.donation.is_some() {
                            // Highlight donations in gold
                            section.format.color =
                                egui::Color32::from_rgb(255, 215, 0).gamma_multiply(alpha);
                        } else {
                            section.format.color = section.format.color.gamma_multiply(alpha);
                        }
                    }

                    ui.add(
                        egui::Label::new(egui::WidgetText::LayoutJob(job.into()))
                            .wrap_mode(egui::TextWrapMode::Wrap),
                    );
                }
            });
        });
}
