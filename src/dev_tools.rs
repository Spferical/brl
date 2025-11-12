//! Development tools for the game. This plugin is only enabled in dev builds.

use bevy::{
    dev_tools::states::log_transitions,
    diagnostic::{DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin},
    input::common_conditions::{input_just_pressed, input_toggle_active},
    prelude::*,
};
use bevy_egui::{EguiContexts, egui};
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use crate::screens::Screen;

pub(super) fn plugin(app: &mut App) {
    // Log `Screen` state transitions.
    app.add_systems(Update, log_transitions::<Screen>);

    // Toggle the debug overlay for UI.
    app.add_systems(
        Update,
        (
            toggle_debug_ui.run_if(input_just_pressed(TOGGLE_KEY)),
            ui_performance.run_if(input_toggle_active(false, KeyCode::F3)),
        ),
    );
    app.add_plugins((
        WorldInspectorPlugin::new().run_if(input_toggle_active(false, KeyCode::F3)),
        FrameTimeDiagnosticsPlugin::default(),
        EntityCountDiagnosticsPlugin::default(),
    ));
}

const TOGGLE_KEY: KeyCode = KeyCode::F3;

fn toggle_debug_ui(mut options: ResMut<UiDebugOptions>) {
    options.toggle();
}

fn ui_performance(mut contexts: EguiContexts, diagnostics: Res<DiagnosticsStore>) {
    let ctx = contexts.ctx_mut().unwrap();
    egui::SidePanel::right("performance_ui_panel")
        .frame(
            egui::Frame::new()
                .fill(egui::Color32::from_black_alpha(240))
                .inner_margin(12.0),
        )
        .show(ctx, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            if let Some(value) = diagnostics
                .get(&FrameTimeDiagnosticsPlugin::FPS)
                .and_then(|fps| fps.smoothed())
            {
                ui.label(format!("FPS: {value:>4.0}"));
            }
            if let Some(value) = diagnostics
                .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
                .and_then(|time| time.smoothed())
            {
                ui.label(format!("Frame Time: {value:>7.3}ms"));
            }
            if let Some(value) = diagnostics
                .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
                .map(|time| time.values().fold(f64::NEG_INFINITY, |a, &b| a.max(b)))
            {
                ui.label(format!("Worst Frame: {value:>7.3}ms"));
            }
            if let Some(value) = diagnostics
                .get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT)
                .and_then(|v| v.value())
            {
                ui.label(format!("Entities: {value:>4}"));
            }
        });
}
