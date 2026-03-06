use bevy::prelude::*;
use bevy_egui::egui::Ui;

use crate::game::{
    FactionMap, Player,
    map::{MapPos, WalkBlockedMap},
    mapgen::MapInfo,
};

#[derive(Resource, Default)]
pub struct DebugSettings {
    show_faction_map: Option<i32>,
    pub nohurt: bool,
    pub teleport_to: Option<usize>,
}

pub fn ui_settings(ui: &mut Ui, settings: &mut DebugSettings) {
    ui.horizontal(|ui| {
        ui.label("dijk map");
        ui.radio_value(&mut settings.show_faction_map, None, "N/A");
        ui.radio_value(&mut settings.show_faction_map, Some(-1), "-1");
        ui.radio_value(&mut settings.show_faction_map, Some(0), "0");
        ui.radio_value(&mut settings.show_faction_map, Some(1), "1");
        ui.checkbox(&mut settings.nohurt, "nohurt");
        ui.label("TP");
        for i in 0..10 {
            if ui.button(i.to_string()).clicked() {
                settings.teleport_to = Some(i);
            }
        }
    });
}

pub(crate) fn teleport_player(
    mut debug: ResMut<DebugSettings>,
    map_info: Res<MapInfo>,
    mut player: Single<&mut MapPos, With<Player>>,
    walk_blocked_map: Res<WalkBlockedMap>,
) {
    if let Some(i) = debug.teleport_to
        && let Some(level) = map_info.levels.get(i)
    {
        debug.teleport_to = None;
        for p in level.rect {
            if !walk_blocked_map.0.contains(&IVec2::from(p)) {
                **player = MapPos(IVec2::from(p));
                break;
            }
        }
    }
}

#[derive(Component)]
pub(crate) struct FactionText;

pub(crate) fn redo_faction_map(
    mut commands: Commands,
    settings: Res<DebugSettings>,
    faction_map: Res<FactionMap>,
    fac_text: Query<Entity, With<FactionText>>,
) {
    for entity in fac_text.iter() {
        commands.entity(entity).despawn();
    }
    if let Some(faction) = settings.show_faction_map
        && let Some(dijk_map) = faction_map.dijkstra_map_per_faction.get(&faction)
    {
        for (pos, val) in dijk_map.iter() {
            commands.spawn((
                FactionText,
                Transform::from_translation(pos.to_vec3(11.0)),
                Text2d(format!("{}", val)),
            ));
        }
    }
}
