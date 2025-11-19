use bevy::prelude::*;

use crate::game::{Player, Turn};

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveIntent(pub IVec2);

pub(crate) fn handle_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    player_entity: Single<Entity, With<Player>>,
) {
    let mut intent = IVec2::ZERO;
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
            intent += dir;
        }
    }
    if intent != IVec2::ZERO {
        commands.entity(*player_entity).insert(MoveIntent(intent));
        commands.run_schedule(Turn);
    }
}
