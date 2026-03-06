use bevy::prelude::*;

use crate::game::{
    Interactable, InteractionType, PLAYER_Z, Stairs, TILE_Z,
    assets::WorldAssets,
    map::{self, MapPos},
    mapgen::MobKind,
};

pub(crate) fn spawn_mob(
    commands: &mut Commands,
    parent: Entity,
    pos: MapPos,
    mob_kind: MobKind,
    assets: &WorldAssets,
    frozen: bool,
) {
    let bundle = mob_kind.get_bundle(assets);
    let map_pos = MapPos(IVec2::from(pos.0));
    let transform = Transform::from_translation(map_pos.to_vec3(PLAYER_Z));
    let mut entity_cmds = commands.spawn((bundle, map_pos, transform));
    if frozen {
        entity_cmds.insert(crate::game::Frozen);
    }
    if mob_kind == MobKind::BrainrotEnemy {
        entity_cmds.insert(assets.get_brainrot_sprite());
    }
    let new_mob = entity_cmds.id();
    commands.entity(parent).add_child(new_mob);
}

pub(crate) fn spawn_stairs(
    world: Entity,
    commands: &mut Commands,
    assets: &WorldAssets,
    down_pos: rogue_algebra::Pos,
    up_pos: rogue_algebra::Pos,
    down_frozen: bool,
    up_frozen: bool,
) {
    let up_pos_map = MapPos(IVec2::from(up_pos));
    let down_pos_map = MapPos(IVec2::from(down_pos));
    let color = Color::srgb(0.4, 0.4, 0.4);
    commands.entity(world).with_children(|parent| {
        let mut up_stairs = parent.spawn((
            Name::new("Up Stairs"),
            up_pos_map,
            Transform::from_translation(up_pos_map.to_vec3(TILE_Z)),
            Stairs {
                destination: down_pos_map,
            },
            Interactable {
                action: "Go Up".to_string(),
                description: None,
                kind: InteractionType::Stairs,
            },
            assets.get_ascii_sprite('<', color),
            GlobalTransform::IDENTITY,
            InheritedVisibility::VISIBLE,
        ));
        if up_frozen {
            up_stairs.insert(crate::game::Frozen);
        }
        up_stairs.with_children(|p| {
            p.spawn((
                Sprite {
                    image: assets.get_solid_mask(),
                    color: Color::srgb(0.1, 0.1, 0.1),
                    custom_size: Some(Vec2::new(map::TILE_WIDTH, map::TILE_HEIGHT)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(0.0, 0.0, -0.1)),
            ));
        });

        let mut down_stairs = parent.spawn((
            Name::new("Down Stairs"),
            down_pos_map,
            Transform::from_translation(down_pos_map.to_vec3(TILE_Z)),
            Stairs {
                destination: up_pos_map,
            },
            Interactable {
                action: "Go Down".to_string(),
                description: None,
                kind: InteractionType::Stairs,
            },
            assets.get_ascii_sprite('>', color),
            GlobalTransform::IDENTITY,
            InheritedVisibility::VISIBLE,
        ));
        if down_frozen {
            down_stairs.insert(crate::game::Frozen);
        }
        down_stairs.with_children(|p| {
            p.spawn((
                Sprite {
                    image: assets.get_solid_mask(),
                    color: Color::srgb(0.1, 0.1, 0.1),
                    custom_size: Some(Vec2::new(map::TILE_WIDTH, map::TILE_HEIGHT)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(0.0, 0.0, -0.1)),
            ));
        });
    });
}
