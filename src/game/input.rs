use bevy::prelude::*;

use crate::game::{Player, Turn, examine::ExaminePos, map::MapPos};

const DIRECTION_KEYS: &[(KeyCode, IVec2)] = &[
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
];

fn check_direction_keys(keyboard_input: &ButtonInput<KeyCode>) -> Option<IVec2> {
    let mut move_intent = IVec2::ZERO;
    for (key, dir) in DIRECTION_KEYS {
        if keyboard_input.just_pressed(*key) {
            move_intent += dir;
        }
    }
    if move_intent != IVec2::ZERO {
        Some(move_intent)
    } else {
        None
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerIntent {
    Move(IVec2),
    Wait,
}

#[derive(Resource, Default)]
pub(crate) enum InputMode {
    #[default]
    Normal,
    Examine(IVec2),
}

pub(crate) fn handle_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    player: Single<(Entity, &MapPos), With<Player>>,
    mut mode: ResMut<InputMode>,
    mut examine_pos: ResMut<ExaminePos>,
) {
    match *mode {
        InputMode::Normal => {
            let intent = if let Some(direction) = check_direction_keys(&keyboard_input) {
                Some(PlayerIntent::Move(direction))
            } else if keyboard_input.just_pressed(KeyCode::Period) {
                Some(PlayerIntent::Wait)
            } else if keyboard_input.just_pressed(KeyCode::KeyX) {
                *mode = InputMode::Examine(player.1.0);
                examine_pos.pos = Some(*player.1);
                None
            } else {
                None
            };
            if let Some(intent) = intent {
                commands.entity(player.0).insert(intent);
                commands.run_schedule(Turn);
            }
        }
        InputMode::Examine(pos) => {
            if let Some(direction) = check_direction_keys(&keyboard_input) {
                *mode = InputMode::Examine(pos + direction);
                examine_pos.pos = Some(MapPos(pos + direction));
            } else if keyboard_input.any_just_pressed([KeyCode::Escape, KeyCode::KeyX]) {
                *mode = InputMode::Normal;
                examine_pos.pos = None;
            }
        }
    }
}
