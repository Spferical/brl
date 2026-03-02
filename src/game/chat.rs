use crate::game::animation::DamageAnimationMessage;
use crate::game::apply_brainrot_ui;
use crate::game::phone::{self, PhoneState};
use crate::game::{Player, TurnCounter};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use rand::{Rng, seq::IndexedRandom};

use std::collections::VecDeque;

pub fn update_streaming_turn(
    mut player: Single<&mut Player>,
    mut phone_state: ResMut<phone::PhoneState>,
    turn_counter: Res<TurnCounter>,
) {
    if phone_state.is_streaming {
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
        if phone_state.viewers > 2000 {
            let overflow = (phone_state.viewers - 2000) as f32;
            // 1 / n^2
            let factor = 2000.0 / (2000.0 + overflow);
            gain *= factor * factor;
        }

        phone_state.viewers_fractional += gain;
        if phone_state.viewers_fractional >= 1.0 {
            let i_gain = phone_state.viewers_fractional.floor();
            phone_state.viewers += i_gain as i32;
            phone_state.viewers_fractional -= i_gain;
        }

        // Sub growth: sublinear to viewers
        // 10% of sqrt(viewers) per turn
        let sub_growth = (phone_state.viewers as f32).sqrt() * 0.1;
        phone_state.subscribers_fractional += sub_growth;
        if phone_state.subscribers_fractional >= 1.0 {
            let gain = phone_state.subscribers_fractional.floor();
            phone_state.subscribers += gain as i32;
            phone_state.subscribers_fractional -= gain;
        }
    }

    // Sub decay: half-life 50 turns
    // new = old * 0.5 ^ (1 / 50)
    // 0.5 ^ (1 / 50) approx 0.98623.
    let decay_factor = 0.98623;
    let old_subs = phone_state.subscribers as f32 + phone_state.subscribers_fractional;
    let new_subs = old_subs * decay_factor;

    phone_state.subscribers = new_subs.floor() as i32;
    phone_state.subscribers_fractional = new_subs.fract();
}

pub fn update_money_timer(time: Res<Time>, mut player: Single<&mut Player>) {
    if player.money_gain_timer > 0.0 {
        player.money_gain_timer -= time.delta_secs();
    }
}

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
}

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

pub fn handle_payout(player: &mut Player, phone_state: &PhoneState) {
    if phone_state.is_streaming && phone_state.subscribers > 0 {
        // $1 per 100 subscribers (min $1 if streaming)
        let payout = (phone_state.subscribers / 100).max(1);
        player.money += payout;
        player.last_gain_amount = payout;
        player.money_gain_timer = 2.0; // Show for 2 seconds
    }
}

pub fn update_chat(
    time: Res<Time>,
    mut chat: ResMut<ChatHistory>,
    phone_state: Res<PhoneState>,
    mut damage_events: MessageReader<DamageAnimationMessage>,
    player_query: Single<Entity, With<Player>>,
) {
    if !phone_state.is_streaming {
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

        let spawn_chance = (phone_state.viewers as f32 * 0.01).min(0.8);
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
    });
}

pub fn draw_chat(
    mut contexts: EguiContexts,
    chat: Res<ChatHistory>,
    phone_state: Res<PhoneState>,
    player: Single<&Player>,
) {
    if !phone_state.is_streaming || chat.messages.is_empty() {
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

                    let text = format!("{}: {}", msg.username, msg.text);

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
                        section.format.color = section.format.color.gamma_multiply(alpha);
                    }

                    ui.add(
                        egui::Label::new(egui::WidgetText::LayoutJob(job.into()))
                            .wrap_mode(egui::TextWrapMode::Wrap),
                    );
                }
            });
        });
}
