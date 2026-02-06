use bevy::prelude::*;

use crate::game::{Player, Turn};

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerIntent {
    Move(IVec2),
    Wait,
}

pub(crate) fn handle_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    player_entity: Single<Entity, With<Player>>,
) {
    let mut move_intent = IVec2::ZERO;
    for (key, dir) in [
        (KeyCode::KeyW, IVec2::new(0, 1)),
        (KeyCode::KeyA, IVec2::new(-1, 0)),
        (KeyCode::KeyS, IVec2::new(0, -1)),
        (KeyCode::KeyD, IVec2::new(1, 0)),
        (KeyCode::KeyH, IVec2::new(-1, 0)),
        (KeyCode::KeyJ, IVec2::new(0, -1)),
        (KeyCode::KeyK, IVec2::new(0, 1)),
        (KeyCode::KeyL, IVec2::new(1, 0)),
        (KeyCode::KeyY, IVec2::new(-1, 1)),
        (KeyCode::KeyU, IVec2::new(1, 1)),
        (KeyCode::KeyB, IVec2::new(-1, -1)),
        (KeyCode::KeyN, IVec2::new(1, -1)),
        (KeyCode::Numpad1, IVec2::new(-1, 1)),
        (KeyCode::Numpad2, IVec2::new(0, 1)),
        (KeyCode::Numpad3, IVec2::new(1, 1)),
        (KeyCode::Numpad4, IVec2::new(-1, 0)),
        (KeyCode::Numpad6, IVec2::new(1, 0)),
        (KeyCode::Numpad7, IVec2::new(-1, -1)),
        (KeyCode::Numpad8, IVec2::new(0, -1)),
        (KeyCode::Numpad9, IVec2::new(1, -1)),
        (KeyCode::ArrowUp, IVec2::new(0, 1)),
        (KeyCode::ArrowDown, IVec2::new(-1, 0)),
        (KeyCode::ArrowLeft, IVec2::new(0, -1)),
        (KeyCode::ArrowRight, IVec2::new(1, 0)),
    ] {
        if keyboard_input.just_pressed(key) {
            move_intent += dir;
        }
    }
    let intent = if move_intent != IVec2::ZERO {
        Some(PlayerIntent::Move(move_intent))
    } else if keyboard_input.any_just_pressed([KeyCode::Period, KeyCode::Space]) {
        Some(PlayerIntent::Wait)
    } else {
        None
    };
    if let Some(intent) = intent {
        commands.entity(*player_entity).insert(intent);
        commands.run_schedule(Turn);
    }
}
