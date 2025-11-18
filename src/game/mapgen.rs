use crate::game::{
    GameWorld, Mob, MobSpawner, MobTemplate, PLAYER_Z, Player, TILE_Z,
    assets::WorldAssets,
    camera::CameraFollow,
    map::{self, MAP_HEIGHT, MapPos},
};
use bevy::{platform::collections::HashMap, prelude::*};
use rand::Rng;

enum TileKind {
    Floor,
    Wall,
    TopSpawner,
    BottomSpawner,
}

pub(crate) fn gen_map(mut commands: Commands, assets: Res<WorldAssets>) {
    let rng = &mut rand::rng();
    let game_world = (
        GameWorld,
        Name::new("GameWorldRoot"),
        Transform::IDENTITY,
        GlobalTransform::IDENTITY,
        InheritedVisibility::VISIBLE,
    );

    let player_sprite = assets.get_urizen_sprite(104);
    let map_pos = MapPos(IVec2::new(3, MAP_HEIGHT / 2));
    let player = (
        Player,
        Name::new("Player"),
        CameraFollow,
        player_sprite,
        map_pos,
        Transform::from_translation(map_pos.to_vec3(PLAYER_Z)),
    );

    let world_rect = rogue_algebra::Rect {
        x1: 0,
        y1: 0,
        x2: map::MAP_WIDTH,
        y2: map::MAP_HEIGHT,
    };

    let mut draft = HashMap::new();
    // Fill in with walls
    for pos in world_rect.expand(1) {
        draft.insert(pos, TileKind::Wall);
    }
    // Dig a tunnel
    for pos in world_rect {
        let tile_kind = if rng.random_bool(0.1) {
            TileKind::Wall
        } else {
            TileKind::Floor
        };
        draft.insert(pos, tile_kind);
    }
    // Spawners on top and bottom
    for pos in world_rect.top_edge() {
        if rng.random_bool(0.2) {
            draft.insert(pos, TileKind::TopSpawner);
        }
    }
    for pos in world_rect.bottom_edge() {
        if rng.random_bool(0.2) {
            draft.insert(pos, TileKind::BottomSpawner);
        }
    }

    let mut tiles = vec![];
    let goblin_template = MobTemplate {
        mob: Mob {
            hp: 5,
            faction: -1,
            max_hp: 5,
            strength: 3,
        },
        sprite: assets.get_urizen_sprite(976),
    };
    let dwarf_template = MobTemplate {
        mob: Mob {
            hp: 5,
            faction: 1,
            max_hp: 5,
            strength: 3,
        },
        sprite: assets.get_urizen_sprite(2785),
    };

    let bottom_spawns = vec![goblin_template];
    let top_spawns = vec![dwarf_template];

    for (rogue_algebra::Pos { x, y }, tile_kind) in draft.into_iter() {
        let map_pos = MapPos(IVec2::new(x, y));
        let transform = Transform::from_translation(map_pos.to_vec3(TILE_Z));
        let mut tile = commands.spawn((map_pos, transform));
        match tile_kind {
            TileKind::Floor => {
                let sprite = assets.get_urizen_sprite(rng.random_range(1857..=1872));
                tile.insert(sprite);
            }
            TileKind::Wall => {
                let sprite = assets.get_urizen_sprite(rng.random_range(0..=1));
                tile.insert((sprite, map::BlocksMovement));
            }
            TileKind::TopSpawner => {
                let sprite = assets.get_urizen_sprite(207);
                tile.insert((
                    sprite,
                    MobSpawner {
                        spawns: top_spawns.clone(),
                        odds: 0.04,
                    },
                ));
            }
            TileKind::BottomSpawner => {
                let sprite = assets.get_urizen_sprite(207);
                tile.insert((
                    sprite,
                    MobSpawner {
                        spawns: bottom_spawns.clone(),
                        odds: 0.04,
                    },
                ));
            }
        }
        tiles.push(tile.id());
    }
    commands
        .spawn(game_world)
        .with_child(player)
        .add_children(&tiles);
}
