use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};

use crate::game::{Creature, Interactable, lighting::Occluder};

pub(crate) const TILE_WIDTH: f32 = 24.0;
pub(crate) const TILE_HEIGHT: f32 = 24.0;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MapPos(pub IVec2);

pub const DIRECTIONS: [IVec2; 8] = [
    IVec2 { x: 1, y: 0 },
    IVec2 { x: 1, y: 1 },
    IVec2 { x: 0, y: 1 },
    IVec2 { x: -1, y: 1 },
    IVec2 { x: -1, y: 0 },
    IVec2 { x: -1, y: -1 },
    IVec2 { x: 0, y: -1 },
    IVec2 { x: 1, y: -1 },
];

/// Offsets of distance 1 in the four cardinal direction.
#[allow(unused)]
pub const CARDINALS: [IVec2; 4] = [
    IVec2 { x: 0, y: 1 },
    IVec2 { x: 0, y: -1 },
    IVec2 { x: 1, y: 0 },
    IVec2 { x: -1, y: 0 },
];

impl MapPos {
    pub fn adjacent(&self) -> [MapPos; 8] {
        DIRECTIONS.map(|d| MapPos(self.0 + d))
    }
    pub fn to_vec2(self) -> Vec2 {
        Vec2 {
            x: TILE_WIDTH * self.0.x as f32,
            y: TILE_HEIGHT * self.0.y as f32,
        }
    }
    pub fn to_vec3(self, z: f32) -> Vec3 {
        self.to_vec2().extend(z)
    }
    #[allow(unused)]
    pub fn from_vec3(vec3: Vec3) -> Self {
        Self::from_vec2(vec3.xy())
    }
    pub fn from_vec2(vec2: Vec2) -> Self {
        Self(IVec2 {
            x: ((vec2.x / TILE_WIDTH) + 0.5) as i32,
            y: ((vec2.y / TILE_HEIGHT) + 0.5) as i32,
        })
    }
    #[allow(unused)]
    pub fn corners(&self) -> [Vec2; 4] {
        [
            self.to_vec2(),
            self.to_vec2() + Vec2::new(0.0, 1.0),
            self.to_vec2() + Vec2::new(1.0, 0.0),
            self.to_vec2() + Vec2::new(1.0, 1.0),
        ]
    }
}

#[derive(Component)]
pub struct Tile;

#[derive(Component)]
pub struct Grass;

#[derive(Component)]
pub struct BlocksMovement;

#[derive(Default, Resource, Deref, DerefMut)]
pub struct WalkBlockedMap(pub HashSet<IVec2>);

#[derive(Default, Resource, Deref, DerefMut)]
pub struct SightBlockedMap(pub HashSet<IVec2>);

#[derive(Default, Resource, Deref, DerefMut)]
pub struct PlayerVisibilityMap(pub HashSet<IVec2>);

#[derive(Default, Resource, Deref, DerefMut)]
pub struct PlayerMemoryMap(pub HashSet<IVec2>);

pub(crate) fn update_walk_blocked_map(
    mut map: ResMut<WalkBlockedMap>,
    q_blocks: Query<&MapPos, With<BlocksMovement>>,
    q_added: Query<(), Added<BlocksMovement>>,
) {
    if !q_added.is_empty() {
        map.clear();
        for MapPos(pos) in q_blocks.iter() {
            map.insert(*pos);
        }
    }
}

pub(crate) fn update_sight_blocked_map(
    mut sight_map: ResMut<SightBlockedMap>,
    q_blocks: Query<&MapPos, With<Occluder>>,
    q_added: Query<(), Added<BlocksMovement>>,
) {
    if !q_added.is_empty() {
        sight_map.clear();
        for MapPos(pos) in q_blocks.iter() {
            sight_map.insert(*pos);
        }
    }
}

pub(crate) fn update_player_visibility(
    mut player_vis_map: ResMut<PlayerVisibilityMap>,
    mut player_memory_map: ResMut<PlayerMemoryMap>,
    q_player: Query<&MapPos, (With<crate::game::Player>, Changed<MapPos>)>,
    sight_blocked_map: Res<SightBlockedMap>,
) {
    if let Some(player_pos) = q_player.iter().next() {
        player_vis_map.clear();
        for pos in rogue_algebra::fov::calculate_fov(player_pos.0.into(), 20, |pos| {
            sight_blocked_map.contains(&IVec2::from(pos))
        }) {
            player_vis_map.insert(IVec2::from(pos));
            player_memory_map.insert(IVec2::from(pos));
        }
    }
}

pub(crate) fn apply_hard_fov_to_tiles(
    player_vis_map: Res<PlayerVisibilityMap>,
    player_memory_map: Res<PlayerMemoryMap>,
    lighting_settings: Res<crate::game::lighting::LightingSettings>,
    mut q_tiles: Query<
        (
            &MapPos,
            &mut Visibility,
            Has<crate::game::Creature>,
            Option<&mut Sprite>,
            Option<&mut TextColor>,
            Option<&crate::game::assets::BaseColor>,
        ),
        (Without<crate::game::Player>, Without<crate::game::Frozen>),
    >,
) {
    #[allow(unused_mut)]
    for (pos, mut vis, is_creature, mut sprite, mut text_color, base_color) in q_tiles.iter_mut() {
        let is_visible = player_vis_map.contains(&pos.0);
        let is_remembered = player_memory_map.contains(&pos.0);

        let target_vis = if is_creature {
            if is_visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            }
        } else if is_remembered {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };

        if *vis != target_vis {
            *vis = target_vis;
        }

        // Apply simple lighting if fancy lighting is disabled
        if !lighting_settings.fancy_lighting && !is_creature && is_remembered {
            let dim_factor = if is_visible { 1.0 } else { 0.2 };
            let original_color = base_color.map(|bc| bc.0).unwrap_or(Color::WHITE);
            let linear = original_color.to_linear();
            let target_color = Color::LinearRgba(LinearRgba {
                red: linear.red * dim_factor,
                green: linear.green * dim_factor,
                blue: linear.blue * dim_factor,
                alpha: linear.alpha,
            });

            if let Some(mut sprite) = sprite
                && sprite.color != target_color
            {
                sprite.color = target_color;
            }
            if let Some(mut text_color) = text_color
                && text_color.0 != target_color
            {
                text_color.0 = target_color;
            }
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct PosToCreature(pub HashMap<IVec2, Entity>);

pub(crate) fn update_pos_to_creature(
    mut pos_to_creature: ResMut<PosToCreature>,
    creatures: Query<(Entity, &MapPos), (With<Creature>, Without<crate::game::Frozen>)>,
) {
    pos_to_creature.0.clear();
    for (entity, pos) in creatures {
        if pos_to_creature.0.insert(pos.0, entity).is_some() {
            warn!("Overlapping mobs at {}", pos.0);
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct PosToInteractable(pub HashMap<MapPos, Vec<Entity>>);

pub(crate) fn update_pos_to_interactable(
    mut pos_to_interactable: ResMut<PosToInteractable>,
    interactable: Query<(Entity, &MapPos), (With<Interactable>, Without<crate::game::Frozen>)>,
) {
    pos_to_interactable.0.clear();
    for (entity, pos) in interactable {
        pos_to_interactable.0.entry(*pos).or_default().push(entity);
    }
}
