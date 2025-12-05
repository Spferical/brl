use bevy::{color::palettes::tailwind::BLUE_600, prelude::*};
use bevy_lit::prelude::{
    AmbientLight2d, LightOccluder2d, Lighting2dSettings, PenetrationSettings, PointLight2d,
};

use crate::game::{assets::WorldAssets, map::{TILE_HEIGHT, TILE_WIDTH}, Player};

#[derive(Component, Default)]
pub struct Occluder;

pub fn enable_lighting(commands: &mut Commands, camera_entity: Entity) {
    commands.entity(camera_entity).insert((
        Lighting2dSettings {
            penetration: PenetrationSettings {
                max: 20.0,
                intensity: 1.0,
                falloff: 1.0,
                sample_directions: 16,
                sample_steps: 8,
            },
            ..default()
        },
        AmbientLight2d {
            intensity: 0.2,
            ..default()
        },
    ));
}

pub fn disable_lighting(commands: &mut Commands, camera_entity: Entity) {
    commands
        .entity(camera_entity)
        .remove::<Lighting2dSettings>()
        .remove::<AmbientLight2d>();
}

pub(super) fn on_add_occluder(
    mut commands: Commands,
    q_added: Query<Entity, (Added<Occluder>, Without<LightOccluder2d>)>,
    assets: Res<WorldAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mesh_cache: Local<Option<Handle<Mesh>>>,
) {
    if q_added.is_empty() {
        return;
    }
    let mesh = mesh_cache
        .get_or_insert_with(|| meshes.add(Rectangle::new(TILE_WIDTH, TILE_HEIGHT)))
        .clone();
    let mask = assets.get_urizen_sprite_mask();

    for entity in q_added.iter() {
        commands.entity(entity).insert((
            Mesh2d(mesh.clone()),
            LightOccluder2d {
                occluder_mask: mask.clone(),
                ..default()
            },
        ));
    }
}

pub(super) fn on_add_player(mut commands: Commands, q_added: Query<Entity, Added<Player>>) {
    for entity in q_added.iter() {
        commands.entity(entity).insert(PointLight2d {
            intensity: 2.0,
            outer_radius: 1100.0,
            falloff: 3.0,
            color: Color::from(BLUE_600),
            ..default()
        });
    }
}
