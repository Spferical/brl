use bevy::{color::palettes::css::WHITE, prelude::*};
use bevy_firefly::prelude::*;

use crate::game::{Player, assets::WorldAssets, map::BlocksMovement};

#[derive(Component, Default, Clone, Copy, Reflect)]
pub struct Occluder;

pub fn enable_lighting(commands: &mut Commands, camera_entity: Entity) {
    commands.entity(camera_entity).insert(FireflyConfig {
        ambient_color: Color::from(WHITE),
        ambient_brightness: 0.05,
        ..default()
    });
}

pub fn disable_lighting(commands: &mut Commands, camera_entity: Entity) {
    commands.entity(camera_entity).remove::<FireflyConfig>();
}

pub(super) fn on_add_occluder(
    mut commands: Commands,
    q_added: Query<
        (
            Entity,
            Option<&Sprite>,
            Option<&Text2d>,
            Has<BlocksMovement>,
        ),
        (Added<Occluder>, Without<Occluder2d>),
    >,
    assets: Res<WorldAssets>,
) {
    if q_added.is_empty() {
        return;
    }

    for (entity, sprite, text, blocks_movement) in q_added.iter() {
        let hull = if blocks_movement {
            // Walls always use solid hull for better occlusion
            assets.get_solid_hull()
        } else if let Some(sprite) = sprite {
            let index = sprite.texture_atlas.as_ref().map(|ta| ta.index);
            if let Some(idx) = index {
                assets
                    .urizen_hulls
                    .get(idx)
                    .cloned()
                    .unwrap_or_else(|| assets.get_solid_hull())
            } else {
                assets.get_solid_hull()
            }
        } else if let Some(text) = text {
            let c = text.0.chars().next().unwrap_or(' ');
            assets
                .char_hulls
                .get(&c)
                .cloned()
                .filter(|h| !h.is_empty())
                .unwrap_or_else(|| assets.get_solid_hull())
        } else {
            assets.get_solid_hull()
        };

        if !hull.is_empty() {
            if let Some(occluder) = Occluder2d::polygon(hull) {
                commands.entity(entity).insert(occluder);
            }
        }
    }
}

pub(super) fn on_add_player(mut commands: Commands, q_added: Query<Entity, Added<Player>>) {
    for entity in q_added.iter() {
        commands.entity(entity).insert((
            PointLight2d {
                intensity: 4.0,
                range: 1200.0,
                inner_range: 50.0,
                falloff: Falloff::Linear,
                color: Color::from(WHITE),
                ..default()
            },
            LightHeight(2.0),
        ));
    }
}
