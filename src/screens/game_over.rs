use crate::game::apply_brainrot_ui;
use crate::game::chat::{ChatHistory, ChatMessage, USERNAMES};
use crate::screens::Screen;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use rand::{Rng, seq::IndexedRandom};

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<GameOverInfo>();
    app.add_systems(OnEnter(Screen::GameOver), setup_game_over);
    app.add_systems(
        Update,
        (update_game_over_timers, update_game_over_chat).run_if(in_state(Screen::GameOver)),
    );
    app.add_systems(
        EguiPrimaryContextPass,
        update_game_over_ui.run_if(in_state(Screen::GameOver)),
    );
    app.add_systems(OnExit(Screen::GameOver), cleanup_game_over);
}

#[derive(Resource, Default, Clone)]
pub struct GameOverInfo {
    pub cause: DeathCause,
    pub killer_name: Option<String>,
    pub brainrot: i32,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum DeathCause {
    #[default]
    LowHP,
    Boredom,
    Other,
}

#[derive(Resource)]
struct GameOverState {
    teletype_timer: Timer,
    teletype_index: usize,
    fade_timer: Timer,
    chat_history: ChatHistory,
    phase: GameOverPhase,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum GameOverPhase {
    Teletype,
    FadeOut,
    ShowChat,
}

const GAME_OVER_TEXT: &str = "GAME OVER";

fn setup_game_over(mut commands: Commands) {
    commands.insert_resource(GameOverState {
        teletype_timer: Timer::from_seconds(0.15, TimerMode::Repeating),
        teletype_index: 0,
        fade_timer: Timer::from_seconds(1.0, TimerMode::Once),
        chat_history: ChatHistory {
            spawn_timer: Timer::from_seconds(0.4, TimerMode::Repeating),
            ..default()
        },
        phase: GameOverPhase::Teletype,
    });
}

fn cleanup_game_over(mut commands: Commands) {
    commands.remove_resource::<GameOverState>();
}

fn update_game_over_timers(time: Res<Time>, mut state: ResMut<GameOverState>) {
    match state.phase {
        GameOverPhase::Teletype => {
            if state.teletype_index < GAME_OVER_TEXT.len() {
                state.teletype_timer.tick(time.delta());
                if state.teletype_timer.just_finished() {
                    state.teletype_index += 1;
                }
            } else {
                state.fade_timer.tick(time.delta());
                if state.fade_timer.just_finished() {
                    state.fade_timer.reset();
                    state.phase = GameOverPhase::FadeOut;
                }
            }
        }
        GameOverPhase::FadeOut => {
            state.fade_timer.tick(time.delta());
            if state.fade_timer.just_finished() {
                state.phase = GameOverPhase::ShowChat;
            }
        }
        GameOverPhase::ShowChat => {}
    }
}

fn update_game_over_ui(
    mut contexts: EguiContexts,
    state: Res<GameOverState>,
    game_over_info: Res<GameOverInfo>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
        .show(ctx, |ui| {
            let screen_rect = ui.max_rect();

            match state.phase {
                GameOverPhase::Teletype => {
                    ui.vertical_centered(|ui| {
                        ui.add_space(screen_rect.height() * 0.45);
                        let displayed_text = &GAME_OVER_TEXT[..state.teletype_index];
                        ui.label(
                            egui::RichText::new(displayed_text)
                                .font(egui::FontId::new(
                                    60.0,
                                    egui::FontFamily::Name("press_start".into()),
                                ))
                                .color(egui::Color32::RED),
                        );
                    });
                }
                GameOverPhase::FadeOut => {
                    let alpha = 1.0 - state.fade_timer.fraction();
                    ui.vertical_centered(|ui| {
                        ui.add_space(screen_rect.height() * 0.45);
                        ui.label(
                            egui::RichText::new(GAME_OVER_TEXT)
                                .font(egui::FontId::new(
                                    60.0,
                                    egui::FontFamily::Name("press_start".into()),
                                ))
                                .color(egui::Color32::RED.gamma_multiply(alpha)),
                        );
                    });
                }
                GameOverPhase::ShowChat => {
                    ui.vertical_centered(|ui| {
                        ui.add_space(screen_rect.height() * 0.1);

                        // Centered scrolling chat
                        let chat_height = screen_rect.height() * 0.6;
                        let chat_width = 600.0;

                        ui.allocate_ui_with_layout(
                            egui::vec2(chat_width, chat_height),
                            egui::Layout::bottom_up(egui::Align::Center),
                            |ui| {
                                ui.spacing_mut().item_spacing.y = 8.0;

                                for (i, msg) in state.chat_history.messages.iter().rev().enumerate()
                                {
                                    let alpha = 1.0 - (i as f32 / 20.0).powi(2);
                                    if alpha <= 0.05 {
                                        continue;
                                    }

                                    let text = format!("{}: {}", msg.username, msg.text);
                                    let widget_text = apply_brainrot_ui(
                                        text,
                                        game_over_info.brainrot,
                                        ui.style(),
                                        egui::FontSelection::Default,
                                        egui::Align::Center,
                                    );

                                    let job = widget_text.into_layout_job(
                                        ui.style(),
                                        egui::FontSelection::Default,
                                        egui::Align::Center,
                                    );
                                    let mut job = (*job).clone();
                                    for section in &mut job.sections {
                                        section.format.color =
                                            section.format.color.gamma_multiply(alpha);
                                    }

                                    ui.add(egui::Label::new(job));
                                }
                            },
                        );

                        // Buttons section
                        ui.add_space(20.0);
                        if ui
                            .button(egui::RichText::new("Try again?").font(egui::FontId::new(
                                24.0,
                                egui::FontFamily::Name("press_start".into()),
                            )))
                            .clicked()
                        {
                            next_screen.set(Screen::Loading);
                        }
                        ui.add_space(15.0);
                        if ui
                            .button(egui::RichText::new("Return to main menu").font(
                                egui::FontId::new(
                                    24.0,
                                    egui::FontFamily::Name("press_start".into()),
                                ),
                            ))
                            .clicked()
                        {
                            next_screen.set(Screen::Title);
                        }
                    });
                }
            }
        });
}

const LOW_HP_MESSAGES: &[&str] = &[
    "L HP",
    "HEAL??",
    "skill issue fr",
    "RIP BOZO",
    "glass cannon but without the cannon",
    "bro forgot to pot",
    "low rizz health bar",
    "negative aura death",
    "chat is this real?",
    "check the VOD, he was throwing",
    "literally 0 hp behavior",
    "bro got fanum taxed by a mob",
    "L stream today",
    "unsubbing after that one",
    "actually hardstuck level 1",
    "bro's health went to Ohio",
];

const BOREDOM_MESSAGES: &[&str] = &[
    "DIED OF BOREDOM LMAO",
    "so boring he actually died",
    "literally me in math class",
    "resident sleeper until death",
    "WAKE UP... oh wait",
    "L content",
    "ResidentSleeper",
    "most exciting brainrotRL gameplay",
    "bro forgot to subway surfers side-screen",
    "needed more soap cutting videos",
    "0/10 entertainment value",
    "chat we need a new streamer",
    "literally dying of cringe",
    "bro's boredom bar reached the sigma limit",
    "go next, this one's a sleeper",
];

const KILLER_MESSAGES: &[&str] = &[
    "GET RECKED BY {killer}",
    "{killer} IS HIM",
    "imagine dying to {killer} lol",
    "L to {killer}",
    "{killer} MOGGED",
    "AURA LOSS TO {killer}",
    "just gifted a sub to {killer}",
    "{killer} has more rizz than u",
    "can we get {killer} in the chat?",
    "imagine being fanum taxed by {killer}",
    "{killer} is literally peaking",
    "absolute sigma move by {killer}",
    "streamer diff vs {killer}",
];

fn update_game_over_chat(
    time: Res<Time>,
    mut state: ResMut<GameOverState>,
    game_over_info: Res<GameOverInfo>,
) {
    if state.phase != GameOverPhase::ShowChat {
        return;
    }

    let chat = &mut state.chat_history;
    chat.spawn_timer.tick(time.delta());

    if chat.spawn_timer.just_finished() {
        let mut rng = rand::rng();
        let username = USERNAMES.choose(&mut rng).unwrap().to_string();

        let pool = match game_over_info.cause {
            DeathCause::LowHP => LOW_HP_MESSAGES,
            DeathCause::Boredom => BOREDOM_MESSAGES,
            DeathCause::Other => &["F", "L", "GG", "unlucky"],
        };

        let mut text = pool.choose(&mut rng).unwrap().to_string();

        if let Some(killer) = &game_over_info.killer_name
            && rng.random_bool(0.4)
        {
            text = KILLER_MESSAGES
                .choose(&mut rng)
                .unwrap()
                .replace("{killer}", killer);
        }

        chat.messages.push(ChatMessage {
            username,
            text,
            timer: Timer::from_seconds(10.0, TimerMode::Once),
            donation: None,
        });

        if chat.messages.len() > 30 {
            chat.messages.remove(0);
        }
    }
}
