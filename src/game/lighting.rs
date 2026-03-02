use bevy::{color::palettes::css::WHITE, platform::collections::HashMap, prelude::*};
use bevy_lit::prelude::{
    AmbientLight2d, LightOccluder2d, Lighting2dSettings, PenetrationSettings, PointLight2d,
    RaymarchSettings,
};

use crate::game::{
    Player,
    assets::WorldAssets,
    map::{TILE_HEIGHT, TILE_WIDTH},
};

#[derive(Component, Default)]
pub struct Occluder;

pub fn enable_lighting(commands: &mut Commands, camera_entity: Entity) {
    commands.entity(camera_entity).insert((
        Lighting2dSettings {
            penetration: PenetrationSettings {
                max: 50.0,
                intensity: 0.6,
                falloff: 0.1,
                sample_directions: 16,
                sample_steps: 10,
            },
            raymarch: RaymarchSettings {
                max_steps: 32,
                jitter_contrib: 0.5,
                sharpness: 10.0,
            },
            ..default()
        },
        AmbientLight2d {
            intensity: 0.15,
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
    q_added: Query<(Entity, Option<&Sprite>), (Added<Occluder>, Without<LightOccluder2d>)>,
    assets: Res<WorldAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    texture_atlas_layouts: Res<Assets<TextureAtlasLayout>>,
    mut mesh_cache: Local<HashMap<Option<usize>, Handle<Mesh>>>,
) {
    if q_added.is_empty() {
        return;
    }

    let mask = assets.get_urizen_sprite_mask();
    let solid_mask = assets.get_solid_mask();
    let layout = texture_atlas_layouts
        .get(&assets.get_urizen_layout())
        .unwrap();
    let atlas_size = layout.size.as_vec2();

    for (entity, sprite) in q_added.iter() {
        let index = sprite
            .and_then(|s| s.texture_atlas.as_ref())
            .map(|ta| ta.index);

        let mesh_handle = mesh_cache.entry(index).or_insert_with(|| {
            let mut mesh = Mesh::from(Rectangle::new(TILE_WIDTH, TILE_HEIGHT));

            if let Some(idx) = index {
                let rect = layout.textures[idx];
                let min = rect.min.as_vec2() / atlas_size;
                let max = rect.max.as_vec2() / atlas_size;

                let uvs = vec![
                    [min.x, min.y], // TL
                    [min.x, max.y], // BL
                    [max.x, max.y], // BR
                    [max.x, min.y], // TR
                ];

                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
            }
            meshes.add(mesh)
        });

        let occluder_mask = if index.is_some() {
            mask.clone()
        } else {
            solid_mask.clone()
        };

        commands.entity(entity).insert((
            Mesh2d(mesh_handle.clone()),
            LightOccluder2d {
                occluder_mask,
            },
        ));
    }
}

pub(super) fn on_add_player(mut commands: Commands, q_added: Query<Entity, Added<Player>>) {
    for entity in q_added.iter() {
        commands.entity(entity).insert(PointLight2d {
            intensity: 2.0,
            outer_radius: 1100.0,
            falloff: 100.0,
            color: Color::from(WHITE),
            ..default()
        });
    }
}
