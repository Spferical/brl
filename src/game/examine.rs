use super::HIGHLIGHT_Z;
use bevy::prelude::*;

use crate::game::assets::WorldAssets;

use crate::game::map::MapPos;

#[derive(Default, Resource)]
pub(crate) struct ExaminePos {
    pub(crate) pos: Option<MapPos>,
}

#[derive(Default, Resource)]
pub(crate) struct ExamineResults {
    pub(crate) info: Option<ExamineInfo>,
}

pub(crate) struct ExamineInfo {
    pub(crate) pos: MapPos,
    pub(crate) info: String,
}

pub(crate) fn update_examine_info(
    examine_pos: Res<ExaminePos>,
    mut examine_results: ResMut<ExamineResults>,
    q_pos: Query<(&MapPos, Option<&Name>)>,
) {
    if let Some(pos) = examine_pos.pos {
        let mut info = String::new();
        for (entity_pos, name) in q_pos.iter() {
            if *entity_pos == pos
                && let Some(name) = name
            {
                info.push_str(name.as_str());
                info.push('\n');
            }
        }
        examine_results.info = Some(ExamineInfo { pos, info });
    } else {
        examine_results.info = None;
    }
}

#[derive(Component)]
pub(crate) struct ExamineHighlight;

pub(crate) fn init_examine_highlight(world: Entity, commands: &mut Commands, assets: &WorldAssets) {
    let highlight = commands
        .spawn((
            Name::new("ExamineHighlight"),
            ExamineHighlight,
            assets.get_urizen_sprite(7908),
            Transform::IDENTITY,
            Visibility::Hidden,
        ))
        .id();
    commands.entity(world).add_child(highlight);
}

pub(crate) fn highlight_examine_tile(
    mut examine_highlight: Single<(&mut Visibility, &mut Transform), With<ExamineHighlight>>,
    examine_pos: Res<ExaminePos>,
) {
    if let Some(pos) = examine_pos.pos {
        *examine_highlight.0 = Visibility::Inherited;
        examine_highlight.1.translation = pos.to_vec3(HIGHLIGHT_Z);
    } else {
        *examine_highlight.0 = Visibility::Hidden;
    }
}
