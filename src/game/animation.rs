use std::time::Duration;

use bevy::{camera::visibility::RenderLayers, prelude::*};
use bevy_egui::egui::{self, Align, FontSelection, RichText};

use crate::game::{DAMAGE_Z, DamageType, apply_brainrot_ui};

const MAX_TICK: Duration = Duration::from_nanos(1_000_000_000 / 30);

pub fn jumping_text(
    ui: &mut egui::Ui,
    text: &str,
    brainrot: i32,
    time: f32,
    base_size: f32,
    color: Option<egui::Color32>,
) {
    ui.spacing_mut().item_spacing.x = 0.0;
    for (i, c) in text.chars().enumerate() {
        let phase = i as f32 * 0.5;
        let t = time * 10.0 - phase;
        let jump = (t.sin() * 5.0).max(0.0);

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.add_space(5.0 - jump);
            let mut rt = RichText::new(c.to_string()).size(base_size);
            if let Some(color) = color {
                rt = rt.color(color);
            }
            ui.add(
                egui::Label::new(apply_brainrot_ui(
                    rt,
                    brainrot,
                    ui.style(),
                    FontSelection::Default,
                    Align::LEFT,
                ))
                .selectable(false),
            );
            ui.add_space(jump);
        });
    }
}

#[derive(Component, Debug)]
pub struct MoveAnimation {
    pub from: Vec3,
    pub to: Vec3,
    pub timer: Timer,
    pub ease: EaseFunction,
    pub rotation: Option<f32>,
    pub sway: Option<f32>,
}

pub fn process_move_animations(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut MoveAnimation)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut animation) in query.iter_mut() {
        animation.timer.tick(time.delta().min(MAX_TICK));
        let fraction = animation.timer.fraction();
        let MoveAnimation { from, to, ease, .. } = *animation;
        transform.translation = EasingCurve::new(from, to, ease).sample_clamped(fraction);

        let mut rotation = 0.0;
        if let Some(total_rotation) = animation.rotation {
            rotation += total_rotation * fraction;
        }
        if let Some(sway_angle) = animation.sway {
            rotation += (fraction * std::f32::consts::PI).sin() * sway_angle;
        }
        transform.rotation = Quat::from_rotation_z(rotation);

        if animation.timer.is_finished() {
            commands.entity(entity).try_remove::<MoveAnimation>();
        }
    }
}

#[derive(Component, Debug)]
pub struct AttackAnimation {
    pub direction: Vec2,
    pub timer: Timer,
    pub base_translation: Vec3,
}

pub fn process_attack_animations(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut AttackAnimation)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut animation) in query.iter_mut() {
        animation.timer.tick(time.delta().min(MAX_TICK));
        let fraction = animation.timer.fraction();

        let bump = (fraction * std::f32::consts::PI).sin();
        let displacement_amount = bump * 12.0;

        let stretch = 1.0 + bump * 0.4;
        let squash = 1.0 - bump * 0.2;

        if animation.direction.x.abs() > animation.direction.y.abs() {
            transform.scale = Vec3::new(stretch, squash, 1.0);
        } else {
            transform.scale = Vec3::new(squash, stretch, 1.0);
        }

        transform.translation.x =
            animation.base_translation.x + animation.direction.x * displacement_amount;
        transform.translation.y =
            animation.base_translation.y + animation.direction.y * displacement_amount;
        transform.translation.z = animation.base_translation.z;

        if animation.timer.is_finished() {
            transform.scale = Vec3::ONE;
            transform.translation = animation.base_translation;
            commands.entity(entity).try_remove::<AttackAnimation>();
        }
    }
}

#[derive(Message)]
pub struct DamageAnimationMessage {
    pub entity: Entity,
    pub amount: i32,
    pub ty: DamageType,
    pub world_pos: Vec3,
    pub is_player: bool,
}

#[derive(Message, Default)]
pub struct FloatingTextMessage {
    pub entity: Option<Entity>,
    pub world_pos: Option<Vec3>,
    pub text: String,
    pub color: Color,
    pub delay: f32,
}

#[derive(Component)]
pub struct FloatingText {
    pub timer: Timer,
    pub delay_timer: Option<Timer>,
}

pub fn spawn_damage_animations(
    mut commands: Commands,
    mut messages: MessageReader<DamageAnimationMessage>,
    _q_transforms: Query<&Transform>,
) {
    for msg in messages.read() {
        let (text, color) = match msg.ty {
            DamageType::Physical => {
                if msg.is_player {
                    (format!("-{} HP", msg.amount), Color::srgb(1.0, 0.2, 0.2))
                } else {
                    (
                        format!("-{} physical damage", msg.amount),
                        Color::srgb(1.0, 0.2, 0.2),
                    )
                }
            }
            DamageType::Psychic => {
                if msg.is_player {
                    (
                        format!("+{} Brainrot", msg.amount),
                        Color::srgb(0.8, 0.2, 1.0),
                    )
                } else {
                    (
                        format!("-{} psychic damage", msg.amount),
                        Color::srgb(0.8, 0.2, 1.0),
                    )
                }
            }
            DamageType::Aura => {
                if msg.is_player {
                    (format!("-{} Rizz", msg.amount), Color::srgb(0.2, 0.8, 1.0))
                } else {
                    (
                        format!("-{} aura damage", msg.amount),
                        Color::srgb(0.2, 0.8, 1.0),
                    )
                }
            }
            DamageType::Boredom => {
                if msg.is_player {
                    (
                        format!("+{} Boredom", msg.amount),
                        Color::srgb(0.6, 0.6, 0.6),
                    )
                } else {
                    (
                        format!("-{} boredom damage", msg.amount),
                        Color::srgb(0.6, 0.6, 0.6),
                    )
                }
            }
            DamageType::Hunger => {
                if msg.is_player {
                    (
                        format!("+{} Hunger", msg.amount),
                        Color::srgb(1.0, 0.5, 0.0),
                    )
                } else {
                    (
                        format!("-{} hunger damage", msg.amount),
                        Color::srgb(1.0, 0.5, 0.0),
                    )
                }
            }
            DamageType::Strength => {
                if msg.is_player {
                    (
                        format!("-{} Strength", msg.amount),
                        Color::srgb(0.2, 1.0, 0.2),
                    )
                } else {
                    (
                        format!("-{} strength damage", msg.amount),
                        Color::srgb(0.2, 1.0, 0.2),
                    )
                }
            }
        };

        spawn_floating_text(&mut commands, text, color, msg.world_pos, 0.0);
    }
}

pub fn spawn_floating_messages(
    mut commands: Commands,
    mut messages: MessageReader<FloatingTextMessage>,
    q_transforms: Query<&Transform>,
) {
    for msg in messages.read() {
        let base_pos = if let Some(entity) = msg.entity {
            q_transforms
                .get(entity)
                .map(|t| t.translation)
                .unwrap_or_default()
        } else {
            msg.world_pos.unwrap_or_default()
        };

        spawn_floating_text(
            &mut commands,
            msg.text.clone(),
            msg.color,
            base_pos,
            msg.delay,
        );
    }
}

fn spawn_floating_text(
    commands: &mut Commands,
    text: String,
    mut color: Color,
    mut pos: Vec3,
    delay: f32,
) {
    pos.z = DAMAGE_Z + 10.0;

    let delay_timer = if delay > 0.0 {
        color.set_alpha(0.0); // hide initially
        Some(Timer::from_seconds(delay, TimerMode::Once))
    } else {
        None
    };

    commands.spawn((
        Text2d::new(text),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(color),
        Transform::from_translation(pos),
        FloatingText {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            delay_timer,
        },
    ));
}

pub fn update_floating_text(
    mut commands: Commands,
    mut query: Query<(Entity, &mut FloatingText, &mut Transform, &mut TextColor)>,
    time: Res<Time>,
) {
    for (entity, mut floating, mut transform, mut color) in query.iter_mut() {
        if let Some(ref mut delay) = floating.delay_timer {
            delay.tick(time.delta().min(MAX_TICK));
            if !delay.is_finished() {
                continue;
            }
        }

        floating.timer.tick(time.delta().min(MAX_TICK));
        let fraction = floating.timer.fraction();

        // Float up
        transform.translation.y += 30.0 * time.delta_secs();

        // Fade out
        let alpha = EasingCurve::new(1.0, 0.0, EaseFunction::CubicIn).sample_clamped(fraction);
        color.0.set_alpha(alpha);

        if floating.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Message)]
pub struct TitleDropMessage(pub String);

#[derive(Component)]
pub struct TitleDrop {
    pub timer: Timer,
}

pub fn update_title_drop(
    mut commands: Commands,
    mut msg_title_drop: MessageReader<TitleDropMessage>,
    query: Query<(Entity, &mut TitleDrop, &mut Transform)>,
    time: Res<Time>,
) {
    for TitleDropMessage(text) in msg_title_drop.read() {
        commands.spawn((
            TitleDrop {
                timer: Timer::from_seconds(5.0, TimerMode::Once),
            },
            Text2d::new(text),
            TextFont {
                font_size: 64.0,
                ..default()
            },
            TextColor(Color::WHITE),
            RenderLayers::layer(1),
        ));
    }
    for (entity, mut td, mut transform) in query {
        td.timer.tick(time.delta().min(MAX_TICK));
        transform.translation.y = 64.0;
        transform.translation.x = if td.timer.fraction() < 0.5 {
            EasingCurve::new(4000.0, 0.0, EaseFunction::QuinticOut)
                .sample_clamped(td.timer.fraction() * 2.0)
        } else {
            EasingCurve::new(0.0, -4000.0, EaseFunction::QuinticIn)
                .sample_clamped((td.timer.fraction() - 0.5) * 2.0)
        };
        if td.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
