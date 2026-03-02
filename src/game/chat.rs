use crate::game::Player;
use crate::game::animation::DamageAnimationMessage;
use crate::game::apply_brainrot_ui;
use crate::game::phone::PhoneState;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use rand::{Rng, seq::IndexedRandom};

#[derive(Resource, Default)]
pub struct ChatHistory {
    pub messages: Vec<ChatMessage>,
    pub spawn_timer: Timer,
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

pub fn update_chat(
    time: Res<Time>,
    mut chat: ResMut<ChatHistory>,
    phone_state: Res<PhoneState>,
    mut damage_events: MessageReader<DamageAnimationMessage>,
    player_query: Single<Entity, With<Player>>,
) {
    if !phone_state.is_streaming {
        chat.messages.clear();
        return;
    }

    let player_entity = *player_query;

    for msg in &mut chat.messages {
        msg.timer.tick(time.delta());
    }
    chat.messages.retain(|msg| !msg.timer.is_finished());

    let mut rng = rand::rng();

    for event in damage_events.read() {
        if event.entity == player_entity {
            add_message(&mut chat, &mut rng, DAMAGE_MESSAGES);
        } else {
            add_message(&mut chat, &mut rng, ATTACK_MESSAGES);
        }
    }

    chat.spawn_timer.tick(time.delta());
    if chat.spawn_timer.is_finished() {
        // Reset timer with some randomness
        chat.spawn_timer
            .set_duration(std::time::Duration::from_secs_f32(
                rng.random_range(0.5..2.0),
            ));
        chat.spawn_timer.reset();

        // More viewers = More messages
        // Base chance + viewer-dependent boost
        let spawn_chance = (phone_state.viewers as f32 * 0.01).min(0.8);
        if rng.random_bool(spawn_chance as f64 + 0.1) {
            add_message(&mut chat, &mut rng, GENERIC_MESSAGES);
        }
    }
}

fn add_message(chat: &mut ChatHistory, rng: &mut impl Rng, pool: &[&str]) {
    let username = USERNAMES.choose(rng).unwrap().to_string();
    let text = pool.choose(rng).unwrap().to_string();

    chat.messages.push(ChatMessage {
        username,
        text,
        timer: Timer::from_seconds(MESSAGE_LIFETIME, TimerMode::Once),
    });

    if chat.messages.len() > MAX_CHAT_MESSAGES {
        chat.messages.remove(0);
    }
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
