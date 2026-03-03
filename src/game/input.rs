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
    m.insert(Key::ArrowDown, IVec2::new(0, -1));
    m.insert(Key::ArrowLeft, IVec2::new(-1, 0));
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

#[derive(Message)]
pub struct AbilityClicked(pub Ability);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerIntent {
    Move(MapPos),
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
    window: Single<&Window>,
    mut msg_ability_clicked: MessageReader<AbilityClicked>,
    mut msg_cursor: MessageReader<CursorMoved>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<Key>>,
    mut commands: Commands,
    player: Single<(Entity, &MapPos), With<Player>>,
    mut mode: ResMut<InputMode>,
    mut examine_pos: ResMut<ExaminePos>,
    abilities: Res<PlayerAbilities>,
    valid_targets: Res<ValidTargets>,
    camera: Single<(&Camera, &GlobalTransform)>,
) {
    let (camera, camera_transform) = camera.into_inner();
    let mouse_pos = window.cursor_position();
    let tile_clicked = if let Some(mouse_pos) = mouse_pos
        && mouse_button_input.just_pressed(MouseButton::Left)
    {
        camera
            .viewport_to_world(camera_transform, mouse_pos)
            .ok()
            .map(|ray| MapPos::from_vec2(ray.origin.truncate()))
    } else {
        None
    };
    let mouse_move_pos = msg_cursor
        .read()
        .last()
        .and_then(|CursorMoved { position, .. }| {
            camera.viewport_to_world(camera_transform, *position).ok()
        })
        .map(|ray| MapPos::from_vec2(ray.origin.truncate()));
    let selected_ability = msg_ability_clicked
        .read()
        .last()
        .map(|AbilityClicked(ability)| ability)
        .or_else(|| {
            check_ability_keys(&keyboard_input).and_then(|idx| abilities.abilities.get_index(idx))
        });
    if let Some(ability) = selected_ability {
        *mode = InputMode::Targeting(*ability, player.1.0);
        examine_pos.pos = Some(*player.1);
    }

    if keyboard_input.pressed(Key::Shift) && matches!(*mode, InputMode::Normal) {
        if let Some(sprint) = abilities
            .abilities
            .iter()
            .find(|a| matches!(a, Ability::Sprint))
        {
            *mode = InputMode::Targeting(*sprint, player.1.0);
            examine_pos.pos = Some(*player.1);
        }
    }

    let mut intent = None;

    match *mode {
        InputMode::Normal => {
            if let Some(direction) = check_direction_keys(&keyboard_input) {
                intent = Some(PlayerIntent::Move(MapPos(player.1.0 + direction)));
            } else if let Some(pos) = tile_clicked {
                if pos == *player.1 {
                    intent = Some(PlayerIntent::Wait);
                } else {
                    intent = Some(PlayerIntent::Move(pos));
                }
            } else if keyboard_input.just_pressed(Key::Character(".".into())) {
                intent = Some(PlayerIntent::Wait);
            } else if keyboard_input.any_just_pressed([
                Key::Character("<".into()),
                Key::Character(">".into()),
                Key::Enter,
            ]) {
                intent = Some(PlayerIntent::UseStairs);
            } else if keyboard_input.just_pressed(Key::Character("x".into())) {
                *mode = InputMode::Examine(player.1.0);
                examine_pos.pos = Some(*player.1);
            }
        }
        InputMode::Examine(pos) => {
            if let Some(direction) = check_direction_keys(&keyboard_input) {
                *mode = InputMode::Examine(pos + direction);
                examine_pos.pos = Some(MapPos(pos + direction));
            } else if keyboard_input.any_just_pressed([Key::Escape, Key::Character("x".into())]) {
                *mode = InputMode::Normal;
                examine_pos.pos = None;
            } else if keyboard_input
                .any_just_pressed([Key::Character("<".into()), Key::Character(">".into())])
            {
                intent = Some(PlayerIntent::UseStairs);
            } else if let Some(mouse_pos) = mouse_move_pos {
                *mode = InputMode::Examine(mouse_pos.0);
                examine_pos.pos = Some(mouse_pos);
            }
        }
        InputMode::Targeting(ability, pos) => {
            if ability == Ability::Sprint && keyboard_input.just_released(Key::Shift) {
                *mode = InputMode::Normal;
                examine_pos.pos = None;
            } else if let Some(clicked) = tile_clicked
                && valid_targets.targets.contains(&clicked)
            {
                *mode = InputMode::Normal;
                examine_pos.pos = None;
                intent = Some(PlayerIntent::UseAbility(ability, clicked))
            } else if keyboard_input
                .any_just_pressed([Key::Character("<".into()), Key::Character(">".into())])
            {
                intent = Some(PlayerIntent::UseStairs);
            } else if let Some(direction) = check_direction_keys(&keyboard_input) {
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
                intent = Some(PlayerIntent::UseAbility(ability, MapPos(pos)))
            } else if let Some(mouse_pos) = mouse_move_pos {
                *mode = InputMode::Targeting(ability, mouse_pos.0);
                examine_pos.pos = Some(mouse_pos);
            }
        }
    };

    if let Some(intent) = intent {
        commands.entity(player.0).insert(intent);
        commands.run_schedule(Turn);
    }
}
