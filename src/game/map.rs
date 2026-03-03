use bevy::{platform::collections::HashSet, prelude::*};

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
    #[allow(unused)]
    pub fn from_vec2(vec2: Vec2) -> Self {
        Self(IVec2 {
            x: (vec2.x / TILE_WIDTH) as i32,
            y: (vec2.y / TILE_HEIGHT) as i32,
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
