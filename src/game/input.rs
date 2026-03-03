use std::sync::LazyLock;

use bevy::{input::keyboard::Key, platform::collections::HashMap, prelude::*};

use crate::game::{
    Ability, Player, PlayerAbilities, Turn, examine::ExaminePos, map::MapPos,
    targeting::ValidTargets,
};

static DIRECTION_KEYS: LazyLock<HashMap<Key, IVec2>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert(Key::Character("w".into()), IVec2::new(0, 1));
    m.insert(Key::Character("a".into()), IVec2::new(-1, 0));
    m.insert(Key::Character("s".into()), IVec2::new(0, -1));
    m.insert(Key::Character("d".into()), IVec2::new(1, 0));
    m.insert(Key::Character("h".into()), IVec2::new(-1, 0));
    m.insert(Key::Character("j".into()), IVec2::new(0, -1));
    m.insert(Key::Character("k".into()), IVec2::new(0, 1));
    m.insert(Key::Character("l".into()), IVec2::new(1, 0));
    m.insert(Key::Character("y".into()), IVec2::new(-1, 1));
    m.insert(Key::Character("u".into()), IVec2::new(1, 1));
    m.insert(Key::Character("b".into()), IVec2::new(-1, -1));
    m.insert(Key::Character("n".into()), IVec2::new(1, -1));
    m.insert(Key::ArrowUp, IVec2::new(0, 1));
    m.insert(Key::ArrowDown, IVec2::new(-1, 0));
    m.insert(Key::ArrowLeft, IVec2::new(0, -1));
    m.insert(Key::ArrowRight, IVec2::new(1, 0));
    m
});

fn check_direction_keys(keyboard_input: &ButtonInput<Key>) -> Option<IVec2> {
    let mut move_intent = IVec2::ZERO;
    for (key, dir) in DIRECTION_KEYS.iter() {
        if keyboard_input.just_pressed(key.clone()) {
            move_intent += dir;
        }
    }
    if move_intent != IVec2::ZERO {
        Some(move_intent)
    } else {
        None
    }
}

static ABILITY_KEYS: LazyLock<Vec<Key>> = LazyLock::new(|| {
    vec![
        Key::Character("1".into()),
        Key::Character("2".into()),
        Key::Character("3".into()),
        Key::Character("4".into()),
        Key::Character("5".into()),
        Key::Character("6".into()),
        Key::Character("7".into()),
        Key::Character("8".into()),
        Key::Character("9".into()),
        Key::Character("0".into()),
    ]
});

fn check_ability_keys(keyboard_input: &ButtonInput<Key>) -> Option<usize> {
    for (i, key) in ABILITY_KEYS.iter().enumerate() {
        if keyboard_input.just_pressed(key.clone()) {
            return Some(i);
        }
    }
    None
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerIntent {
    Move(IVec2),
    Wait,
    UseStairs,
    UseAbility(Ability, MapPos),
}

#[derive(Resource, Default)]
pub(crate) enum InputMode {
    #[default]
    Normal,
    Examine(IVec2),
    Targeting(Ability, IVec2),
}

pub(crate) fn handle_input(
    keyboard_input: Res<ButtonInput<Key>>,
    mut commands: Commands,
    player: Single<(Entity, &MapPos), With<Player>>,
    mut mode: ResMut<InputMode>,
    mut examine_pos: ResMut<ExaminePos>,
    abilities: Res<PlayerAbilities>,
    valid_targets: Res<ValidTargets>,
) {
    match *mode {
        InputMode::Normal => {
            let intent = if let Some(direction) = check_direction_keys(&keyboard_input) {
                Some(PlayerIntent::Move(direction))
            } else if let Some(idx) = check_ability_keys(&keyboard_input)
                && let Some(ability) = abilities.abilities.get_index(idx)
            {
                *mode = InputMode::Targeting(*ability, player.1.0);
                examine_pos.pos = Some(*player.1);
                None
            } else if keyboard_input.just_pressed(Key::Character(".".into())) {
                Some(PlayerIntent::Wait)
            } else if keyboard_input.any_just_pressed([
                Key::Character("<".into()),
                Key::Character(">".into()),
                Key::Enter,
            ]) {
                Some(PlayerIntent::UseStairs)
            } else if keyboard_input.just_pressed(Key::Character("x".into())) {
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
            } else if keyboard_input.any_just_pressed([Key::Escape, Key::Character("x".into())]) {
                *mode = InputMode::Normal;
                examine_pos.pos = None;
            }
        }
        InputMode::Targeting(ability, pos) => {
            if let Some(direction) = check_direction_keys(&keyboard_input) {
                *mode = InputMode::Targeting(ability, pos + direction);
                examine_pos.pos = Some(MapPos(pos + direction));
            } else if keyboard_input.any_just_pressed([Key::Escape, Key::Character("x".into())]) {
                *mode = InputMode::Normal;
                examine_pos.pos = None;
            } else if keyboard_input.any_just_pressed([Key::Space, Key::Enter])
                && valid_targets.targets.contains(&MapPos(pos))
            {
                *mode = InputMode::Normal;
                examine_pos.pos = None;
                commands
                    .entity(player.0)
                    .insert(PlayerIntent::UseAbility(ability, MapPos(pos)));
                commands.run_schedule(Turn);
            }
        }
    }
}
