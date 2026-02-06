use bevy::prelude::*;
use bevy_egui::egui::Ui;

use crate::game::{FactionMap, map::MapPos};

#[derive(Resource, Default)]
pub struct DebugSettings {
    show_faction_map: Option<i32>,
}

pub fn ui_settings(ui: &mut Ui, settings: &mut DebugSettings) {
    ui.horizontal(|ui| {
        ui.label("dijk map");
        ui.radio_value(&mut settings.show_faction_map, None, "N/A");
        ui.radio_value(&mut settings.show_faction_map, Some(-1), "-1");
        ui.radio_value(&mut settings.show_faction_map, Some(0), "0");
        ui.radio_value(&mut settings.show_faction_map, Some(1), "1");
    });
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
                Transform::from_translation(MapPos(IVec2::from(*pos)).to_vec3(11.0)),
                Text2d(format!("{}", val)),
            ));
        }
    }
}
