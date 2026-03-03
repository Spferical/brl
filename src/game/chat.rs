use crate::game::animation::DamageAnimationMessage;
use crate::game::apply_brainrot_ui;
use crate::game::{Player, TurnCounter};
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
    pub viewers: i32,            // True count, updated on Turn
    pub viewers_displayed: f32,  // For UI animation
    pub viewers_fractional: f32, // For precise turn-based growth
    pub subscribers_fractional: f32,
}

pub fn update_streaming_stats(time: Res<Time>, mut streaming_state: ResMut<StreamingState>) {
    if !streaming_state.is_streaming {
        streaming_state.viewers = 0;
        streaming_state.viewers_displayed = 0.0;
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
    if streaming_state.is_streaming {
        // 1 brainrot every 30 turns
        if turn_counter.0.is_multiple_of(30) {
            player.brainrot += 1;
        }

        // Viewers growth per turn: a * e^(b * rizz)
        // b = ln(50)/40 = 0.0978 (50x increase from 10 to 50 rizz)
        // a = 100 / e^(100*b) = 0.00566 (100/turn at 100 rizz)
        let rizz = player.rizz as f32;
        let mut gain = 0.00566 * (0.0978 * rizz).exp();

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

        // Sub growth: sublinear to viewers
        // 10% of sqrt(viewers) per turn
        let sub_growth = (streaming_state.viewers as f32).sqrt() * 0.1;
        streaming_state.subscribers_fractional += sub_growth;
        if streaming_state.subscribers_fractional >= 1.0 {
            let gain = streaming_state.subscribers_fractional.floor();
            streaming_state.subscribers += gain as i32;
            streaming_state.subscribers_fractional -= gain;
        }
    }

    // Sub decay: half-life 50 turns
    // new = old * 0.5 ^ (1 / 50)
    // 0.5 ^ (1 / 50) approx 0.98623.
    let decay_factor = 0.98623;
    let old_subs = streaming_state.subscribers as f32 + streaming_state.subscribers_fractional;
    let new_subs = old_subs * decay_factor;

    streaming_state.subscribers = new_subs.floor() as i32;
    streaming_state.subscribers_fractional = new_subs.fract();
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
    if streaming_state.is_streaming && streaming_state.subscribers > 0 {
        let mut rng = rand::rng();

        // $1 per 100 subscribers
        let base_payout = (streaming_state.subscribers / 100).max(1);

        // Add randomness: 50% to 150% of base
        let random_factor = rng.random_range(0.5..1.5);
        let mut payout = (base_payout as f32 * random_factor).round() as i32;

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

pub fn update_chat(
    time: Res<Time>,
    mut chat: ResMut<ChatHistory>,
    streaming_state: Res<StreamingState>,
    mut damage_events: MessageReader<DamageAnimationMessage>,
    player_query: Single<Entity, With<Player>>,
) {
    if !streaming_state.is_streaming {
        chat.messages.clear();
        chat.queue.clear();
        return;
    }

    let player_entity = *player_query;

    for msg in &mut chat.messages {
        msg.timer.tick(time.delta());
    }
    chat.messages.retain(|msg| !msg.timer.is_finished());

    let mut rng = rand::rng();

    // Event-based messages go to queue
    for event in damage_events.read() {
        if event.entity == player_entity {
            queue_message(&mut chat, &mut rng, DAMAGE_MESSAGES);
        } else {
            queue_message(&mut chat, &mut rng, ATTACK_MESSAGES);
        }
    }

    // Generic background messages go to queue
    chat.spawn_timer.tick(time.delta());
    if chat.spawn_timer.is_finished() {
        chat.spawn_timer
            .set_duration(std::time::Duration::from_secs_f32(
                rng.random_range(0.5..2.0),
            ));
        chat.spawn_timer.reset();

        let spawn_chance = (streaming_state.viewers as f32 * 0.01).min(0.8);
        if rng.random_bool(spawn_chance as f64 + 0.1) {
            queue_message(&mut chat, &mut rng, GENERIC_MESSAGES);
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

pub fn draw_streaming_indicator(mut contexts: EguiContexts, streaming_state: Res<StreamingState>) {
    if !streaming_state.is_streaming {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::Area::new(egui::Id::new("streaming_indicator"))
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 20.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 8.0, Color32::RED);
                ui.label(
                    RichText::new("Streaming...")
                        .color(Color32::RED)
                        .font(egui::FontId::new(
                            20.0,
                            egui::FontFamily::Name("press_start".into()),
                        ))
                        .strong(),
                );
            });
        });
}

pub fn draw_chat(
    mut contexts: EguiContexts,
    chat: Res<ChatHistory>,
    streaming_state: Res<StreamingState>,
    player: Single<&Player>,
) {
    if !streaming_state.is_streaming || chat.messages.is_empty() {
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
