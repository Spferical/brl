//! The game's main screen states and transitions between them.

pub(crate) mod game_over;
mod gameplay;
mod loading;
mod title;

use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.init_state::<Screen>();

    app.add_plugins((
        gameplay::plugin,
        loading::plugin,
        title::plugin,
        game_over::plugin,
    ));
}

/// The game's main screen states.
#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub enum Screen {
    #[default]
    Title,
    Loading,
    Gameplay,
    GameOver,
}
