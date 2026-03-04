use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};

use crate::game::{Creature, Interactable};

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
pub struct BlocksMovement;

#[derive(Default, Resource, Deref, DerefMut)]
pub struct WalkBlockedMap(pub HashSet<IVec2>);

pub(crate) fn update_walk_blocked_map(
    mut map: ResMut<WalkBlockedMap>,
    q_blocks: Query<&MapPos, With<BlocksMovement>>,
) {
    map.clear();
    for MapPos(pos) in q_blocks.iter() {
        map.insert(*pos);
    }
}

#[derive(Resource, Default)]
pub(crate) struct PosToCreature(pub HashMap<IVec2, Entity>);

pub(crate) fn update_pos_to_creature(
    mut pos_to_creature: ResMut<PosToCreature>,
    creatures: Query<(Entity, &MapPos), With<Creature>>,
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

impl PosToInteractable {
    pub(crate) fn get<'a>(&'a self, pos: MapPos) -> impl Iterator<Item = Entity> + 'a {
        self.0.get(&pos).into_iter().flat_map(|v| v.iter()).copied()
    }
}

pub(crate) fn update_pos_to_interactable(
    mut pos_to_interactable: ResMut<PosToInteractable>,
    interactable: Query<(Entity, &MapPos), With<Interactable>>,
) {
    pos_to_interactable.0.clear();
    for (entity, pos) in interactable {
        pos_to_interactable.0.entry(*pos).or_default().push(entity);
    }
}
